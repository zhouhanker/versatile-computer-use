#!/usr/bin/env python3
"""Edge-only DOM POC on a throwaway 127.0.0.1 page.

Does not open Chrome. Does not touch existing tabs. Does not move the OS cursor.
"""
from __future__ import annotations

import json
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VCU = ROOT / "target" / "debug" / "vcu.exe"
HTML = """<!doctype html>
<meta charset="utf-8">
<title>vcu-edge-dom</title>
<button id="hit">0</button>
<input id="box" value="">
<script>
document.getElementById("hit").addEventListener("click", () => {
  const b = document.getElementById("hit");
  b.textContent = String((Number(b.textContent) || 0) + 1);
});
</script>
""".encode("utf-8")


def vcu(args):
    cmd = [str(VCU), *args]
    if "--json" not in cmd:
        cmd.append("--json")
    proc = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
    try:
        parsed = json.loads(proc.stdout or "{}")
    except json.JSONDecodeError:
        parsed = {"ok": False, "raw": (proc.stdout or proc.stderr or "")[:500], "code": proc.returncode}
    parsed["_code"] = proc.returncode
    return parsed


def data(resp):
    return resp.get("data") if isinstance(resp.get("data"), dict) else {}


def tab_map(resp):
    out = {}
    for tab in data(resp).get("tabs") or []:
        out[str(tab.get("tab_id"))] = {
            "browser": tab.get("browser"),
            "url": tab.get("url"),
            "title": tab.get("title"),
        }
    return out


def main():
    if not VCU.is_file():
        print("EDGE-DOM FAIL debug vcu missing")
        return 1
    before = vcu(["browser", "tabs"])
    if before.get("ok") is not True:
        print("EDGE-DOM FAIL tabs")
        return 1
    browsers = data(before).get("browsers") or []
    if browsers != ["edge"]:
        print(f"EDGE-DOM FAIL expected only edge, got {browsers}")
        return 1
    existing = tab_map(before)
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{httpd.server_address[1]}/vcu-edge-dom?t={int(time.time())}"
    our_tab = None
    try:
        opened = vcu(["browser", "open", "--browser", "edge", "--background", "--url", url])
        our_tab = str(data(opened).get("tab_id") or "")
        if opened.get("ok") is not True or not our_tab.isdigit() or our_tab in existing:
            print("EDGE-DOM FAIL open")
            print(json.dumps(opened, ensure_ascii=False)[:600])
            return 1
        extracted = None
        source = None
        for _ in range(24):
            time.sleep(0.25)
            ex = vcu(["browser", "extract", "--browser", "edge", "--tab", our_tab, "--selector", "#hit"])
            matches = data(ex).get("matches") or []
            source = data(ex).get("source")
            text = ""
            if matches:
                text = str(matches[0].get("text") or "").strip()
            if ex.get("ok") is True and text == "0":
                extracted = ex
                break
        if extracted is None or source != "extension_dom":
            print(f"EDGE-DOM FAIL extract0 source={source}")
            return 1
        click = vcu(["browser", "click", "--browser", "edge", "--tab", our_tab, "--selector", "#hit"])
        if click.get("ok") is not True or data(click).get("os_cursor_used") is True:
            print("EDGE-DOM FAIL click")
            print(json.dumps(click, ensure_ascii=False)[:600])
            return 1
        after_click = None
        for _ in range(16):
            time.sleep(0.2)
            ex, text = extract_hit(our_tab)
            if text == "1" and data(ex).get("source") == "extension_dom":
                after_click = text
                break
        if after_click != "1":
            print("EDGE-DOM FAIL click did not change text")
            return 1
        typed = vcu([
            "browser", "type", "--browser", "edge", "--tab", our_tab,
            "--selector", "#box", "--text", "vcu-edge-dom",
        ])
        if typed.get("ok") is not True or data(typed).get("os_cursor_used") is True:
            print("EDGE-DOM FAIL type")
            print(json.dumps(typed, ensure_ascii=False)[:600])
            return 1
        box = None
        for _ in range(16):
            time.sleep(0.2)
            ex = vcu(["browser", "extract", "--browser", "edge", "--tab", our_tab, "--selector", "#box"])
            matches = data(ex).get("matches") or []
            value = ""
            if matches:
                value = str(matches[0].get("value") or matches[0].get("text") or "")
            if data(ex).get("source") == "extension_dom" and "vcu-edge-dom" in value:
                box = value
                break
        if box is None:
            print("EDGE-DOM FAIL typed value")
            return 1
        closed = vcu(["browser", "close", "--browser", "edge", "--tab", our_tab])
        if closed.get("ok") is not True:
            print("EDGE-DOM FAIL close")
            return 1
        our_tab = None
        after = vcu(["browser", "tabs"])
        now = tab_map(after)
        missing = [tid for tid in existing if tid not in now or now[tid]["url"] != existing[tid]["url"]]
        extra = [tid for tid in now if tid not in existing]
        if missing or extra or data(after).get("browsers") != ["edge"]:
            print(f"EDGE-DOM FAIL tabs changed missing={missing} extra={extra}")
            return 1
        if data(after).get("os_cursor_used") is True:
            print("EDGE-DOM FAIL cursor")
            return 1
        print(f"EDGE-DOM OK edge tab closed hit=0->1 typed={box} source=extension_dom")
        return 0
    finally:
        if our_tab:
            vcu(["browser", "close", "--browser", "edge", "--tab", our_tab])
        httpd.shutdown()


def extract_hit(tab):
    ex = vcu(["browser", "extract", "--browser", "edge", "--tab", tab, "--selector", "#hit"])
    matches = data(ex).get("matches") or []
    text = str(matches[0].get("text") or "").strip() if matches else ""
    return ex, text


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(HTML)

    def log_message(self, *_args):
        return


if __name__ == "__main__":
    raise SystemExit(main())
