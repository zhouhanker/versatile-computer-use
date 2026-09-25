#!/usr/bin/env python3
"""Edge web cursor uses the same fog center as the macOS guide.

Does not open Chrome. Does not touch existing tabs. Does not move the OS cursor.
Does not click Allow.
"""
from __future__ import annotations

import ctypes
import json
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VCU = ROOT / "target" / "x86_64-pc-windows-gnu" / "debug" / "vcu.exe"
if not VCU.is_file():
    VCU = ROOT / "target" / "debug" / "vcu.exe"
HTML = """<!doctype html>
<meta charset="utf-8">
<title>vcu-edge-cursor</title>
<button id="hit">0</button>
<div id="probe">none</div>
<script>
document.getElementById("hit").addEventListener("click", () => {
  const b = document.getElementById("hit");
  b.textContent = String((Number(b.textContent) || 0) + 1);
});
setInterval(() => {
  const cursor = document.getElementById("vcu-virtual-cursor");
  const probe = document.getElementById("probe");
  if (!cursor || !probe) return;
  probe.textContent = cursor.getAttribute("data-vcu-fog") || "missing";
}, 40);
</script>
""".encode("utf-8")


class POINT(ctypes.Structure):
    _fields_ = [("x", ctypes.c_long), ("y", ctypes.c_long)]


def cursor_pos():
    pt = POINT()
    ctypes.windll.user32.GetCursorPos(ctypes.byref(pt))
    return f"{pt.x},{pt.y}"


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


def extract_text(tab, selector):
    ex = vcu(["browser", "extract", "--browser", "edge", "--tab", tab, "--selector", selector])
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


def main():
    if not VCU.is_file():
        print("EDGE-CURSOR FAIL vcu missing")
        return 1
    cursor0 = cursor_pos()
    installed = vcu(["browser", "install-lens", "--from", str(ROOT / "extension"), "--reload"])
    if installed.get("ok") is not True:
        print("EDGE-CURSOR FAIL install-lens")
        print(json.dumps(installed, ensure_ascii=False)[:600])
        return 1
    pong = None
    for _ in range(20):
        time.sleep(0.25)
        ping = vcu(["browser", "ping"])
        if ping.get("ok") is True and data(ping).get("pong") is True:
            pong = ping
            break
    if pong is None:
        print("EDGE-CURSOR FAIL ping after reload")
        return 1
    before = vcu(["browser", "tabs"])
    if before.get("ok") is not True or (data(before).get("browsers") or []) != ["edge"]:
        print(f"EDGE-CURSOR FAIL expected only edge, got {data(before).get('browsers')}")
        return 1
    existing = tab_map(before)
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{httpd.server_address[1]}/vcu-edge-cursor?t={int(time.time())}"
    our_tab = None
    try:
        opened = vcu(["browser", "open", "--browser", "edge", "--background", "--url", url])
        our_tab = str(data(opened).get("tab_id") or "")
        if opened.get("ok") is not True or not our_tab.isdigit() or our_tab in existing:
            print("EDGE-CURSOR FAIL open")
            return 1
        ready = False
        for _ in range(24):
            time.sleep(0.25)
            ex, text = extract_text(our_tab, "#hit")
            if ex.get("ok") is True and text == "0" and data(ex).get("source") == "extension_dom":
                ready = True
                break
        if not ready:
            print("EDGE-CURSOR FAIL extract0")
            return 1
        click = vcu(["browser", "click", "--browser", "edge", "--tab", our_tab, "--selector", "#hit"])
        if click.get("ok") is not True or data(click).get("source") != "extension_dom" or data(click).get("os_cursor_used") is True:
            print("EDGE-CURSOR FAIL click")
            print(json.dumps(click, ensure_ascii=False)[:600])
            return 1
        fog = None
        hit = None
        for _ in range(20):
            time.sleep(0.2)
            _, hit_text = extract_text(our_tab, "#hit")
            _, probe = extract_text(our_tab, "#probe")
            if hit_text == "1":
                hit = hit_text
            if probe == "6,6,36":
                fog = probe
            if hit == "1" and fog == "6,6,36":
                break
        if hit != "1" or fog != "6,6,36":
            print(f"EDGE-CURSOR FAIL hit={hit} fog={fog}")
            return 1
        closed = vcu(["browser", "close", "--browser", "edge", "--tab", our_tab])
        if closed.get("ok") is not True:
            print("EDGE-CURSOR FAIL close")
            return 1
        our_tab = None
        after = vcu(["browser", "tabs"])
        now = tab_map(after)
        missing = [tid for tid in existing if tid not in now or now[tid]["url"] != existing[tid]["url"]]
        extra = [tid for tid in now if tid not in existing]
        if missing or extra:
            print(f"EDGE-CURSOR FAIL tabs changed missing={missing} extra={extra}")
            return 1
        cursor1 = cursor_pos()
        if cursor0 != cursor1:
            print(f"EDGE-CURSOR FAIL cursor moved {cursor0} -> {cursor1}")
            return 1
        print(f"EDGE-CURSOR OK edge fog=6,6,36 hit=0->1 source=extension_dom cursor={cursor1}")
        return 0
    finally:
        if our_tab:
            vcu(["browser", "close", "--browser", "edge", "--tab", our_tab])
        httpd.shutdown()


if __name__ == "__main__":
    raise SystemExit(main())
