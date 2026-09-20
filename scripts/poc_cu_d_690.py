#!/usr/bin/env python3
"""CU-D-690: extension DOM sets a native <select> and reports input_path=dom_select.

Throwaway 127.0.0.1 page only. Native groups untouched. Never Allow / OS cursor.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
_debug = ROOT / "target/debug/vcu"
VCU = os.environ.get("VCU") or (str(_debug) if _debug.exists() else "vcu")
OUT = Path(".local/desktop-cu/cu-d-690.json")

PAGE = """<!doctype html>
<title>vcu-d-690</title>
<select id="pick">
  <option value="low">general question</option>
  <option value="normal">bug or problem</option>
</select>
<input id="txt" type="text">
<div id="out">none</div>
<div id="events">0</div>
<script>
  const pick = document.getElementById('pick');
  const out = document.getElementById('out');
  const events = document.getElementById('events');
  let seen = [];
  pick.addEventListener('input', () => { seen.push('input'); events.textContent = seen.join(','); });
  pick.addEventListener('change', () => { seen.push('change'); events.textContent = seen.join(','); out.textContent = pick.value; });
</script>
"""


def vcu(args):
    cmd = [VCU, *args]
    if "--json" not in cmd:
        cmd.append("--json")
    proc = subprocess.run(cmd, capture_output=True, text=True)
    try:
        return json.loads(proc.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (proc.stdout or proc.stderr or "")[:800]}


def data(resp):
    payload = resp.get("data") if isinstance(resp.get("data"), dict) else {}
    nested = payload.get("extension") if isinstance(payload.get("extension"), dict) else {}
    merged = dict(nested)
    merged.update(payload)
    return merged


def err_text(resp):
    err = resp.get("error") or {}
    return " ".join(str(p) for p in (err.get("message"), err.get("detail")) if p)


def extract_text(tab, selector):
    resp = vcu(["browser", "extract", "--selector", selector, "--tab", tab, "--browser", "edge"])
    matches = data(resp).get("matches") or []
    if not matches or not isinstance(matches[0], dict):
        return ""
    return str(matches[0].get("text") or "").strip()


def set_select(tab, value):
    return vcu(["browser", "type", "--selector", "#pick", "--text", value, "--tab", tab, "--browser", "edge"])


def main():
    report = {"ok": False}
    tab = ""
    httpd = None
    try:
        ready = False
        for _ in range(20):
            ping = vcu(["browser", "ping"])
            status = vcu(["daemon", "status"])
            browsers = [str(b) for b in (data(status).get("extension_browsers") or [])]
            if ping.get("ok") is True and "edge" in browsers:
                ready = True
                break
            time.sleep(0.4)
        if not ready:
            print("CU-D-690 FAIL need edge lens")
            report.update({"step": "health"})
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                body = PAGE.encode("utf-8")
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        port = httpd.server_address[1]
        opened = vcu(
            [
                "browser",
                "open",
                "--background",
                "--browser",
                "edge",
                "--url",
                f"http://127.0.0.1:{port}/select?t={int(time.time())}",
            ]
        )
        tab = str(data(opened).get("tab_id") or "")
        if opened.get("ok") is not True or not tab:
            print("CU-D-690 FAIL open", opened.get("error"))
            report.update({"step": "open", "error": opened.get("error")})
            return 1
        time.sleep(0.6)

        initial_out = extract_text(tab, "#out")

        by_label = set_select(tab, "bug or problem")
        label_value = str(data(by_label).get("selected_value") or "")
        label_path = str(data(by_label).get("input_path") or "")
        time.sleep(0.3)
        out_after_label = extract_text(tab, "#out")
        events_after_label = extract_text(tab, "#events")

        by_value = set_select(tab, "low")
        value_value = str(data(by_value).get("selected_value") or "")
        time.sleep(0.3)
        out_after_value = extract_text(tab, "#out")

        missing = set_select(tab, "no-such-option")
        missing_text = err_text(missing).lower()
        time.sleep(0.3)
        out_after_missing = extract_text(tab, "#out")

        typed = vcu(
            ["browser", "type", "--selector", "#txt", "--text", "vcu-d-690", "--tab", tab, "--browser", "edge"]
        )
        typed_path = str(data(typed).get("input_path") or "")

        select_ok = (
            by_label.get("ok") is True
            and label_path == "dom_select"
            and label_value == "normal"
            and out_after_label == "normal"
            and "change" in events_after_label
        )
        value_ok = by_value.get("ok") is True and value_value == "low" and out_after_value == "low"
        missing_ok = missing.get("ok") is False and "select option not found" in missing_text
        untouched_ok = initial_out == "none" and out_after_missing == "low"
        text_ok = typed.get("ok") is True and typed_path in ("", "dom_type")

        ok = all([select_ok, value_ok, missing_ok, untouched_ok, text_ok])
        report = {
            "ok": ok,
            "tab": tab,
            "label_path": label_path,
            "label_value": label_value,
            "out_after_label": out_after_label,
            "events_after_label": events_after_label,
            "value_value": value_value,
            "out_after_value": out_after_value,
            "missing_error": err_text(missing).strip(),
            "out_after_missing": out_after_missing,
            "typed_input_path": typed_path,
        }
        if ok:
            print("CU-D-690 OK dom_select", label_value, "by_value", value_value)
            return 0
        print("CU-D-690 FAIL", json.dumps(report, ensure_ascii=False))
        return 1
    finally:
        if tab:
            vcu(["browser", "close", "--tab", tab, "--browser", "edge"])
        if httpd is not None:
            httpd.shutdown()
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    sys.exit(main())
