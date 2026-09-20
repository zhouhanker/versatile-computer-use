#!/usr/bin/env python3
"""CU-D-600: observe without --tab fails unless frontmost is USER Chrome/Edge.

Does not steal OS frontmost. Groups 1/3 untouched. Never Allow / OS cursor.
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

PROTECTED = {"1", "3"}
ROOT = Path(__file__).resolve().parents[1]
_debug = ROOT / "target/debug/vcu"
VCU = os.environ.get("VCU") or (str(_debug) if _debug.exists() else "vcu")
OUT = Path(".local/desktop-cu/cu-d-600.json")

HTML = b"""<!doctype html><title>vcu-d-600</title><p>ok</p>"""


def vcu(args):
    cmd = [VCU, *args]
    if "--json" not in cmd:
        cmd.append("--json")
    p = subprocess.run(cmd, capture_output=True, text=True)
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:800], "code": p.returncode}


def data(resp):
    return resp.get("data") if isinstance(resp.get("data"), dict) else {}


def err_msg(resp):
    err = resp.get("error") if isinstance(resp.get("error"), dict) else {}
    return str(err.get("message") or "")


def protected(tabs):
    out = {}
    for g in data(tabs).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            out[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    return out


def frontmost_app():
    p = subprocess.run(
        [
            "osascript",
            "-e",
            'tell application "System Events" to get name of first application process whose frontmost is true',
        ],
        capture_output=True,
        text=True,
    )
    return (p.stdout or "").strip()


def is_user_browser(name: str) -> bool:
    l = name.lower()
    if "edge" in l:
        return True
    return "chrome" in l and "edge" not in l


def main():
    report = {"ok": False}
    our_tab = None
    httpd = None
    prot0 = protected(vcu(["browser", "tabs"]))
    try:
        front = frontmost_app()
        default = vcu(["browser", "observe"])
        default_msg = err_msg(default)
        browser_front = is_user_browser(front)
        if browser_front:
            default_ok = default.get("ok") is True and bool(data(default).get("tab_id"))
            default_honest = default_ok
        else:
            default_ok = False
            default_honest = (
                default.get("ok") is False
                and "frontmost is not USER Chrome/Edge" in default_msg
                and "observe --tab" in default_msg
            )

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(HTML)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        url = f"http://127.0.0.1:{httpd.server_address[1]}/vcu-d-600?t={int(time.time())}"
        opened = vcu(["browser", "open", "--background", "--url", url])
        our_tab = str(data(opened).get("tab_id") or "")
        tab_ok = False
        if opened.get("ok") is True and our_tab.isdigit():
            time.sleep(0.4)
            obs = vcu(["browser", "observe", "--tab", our_tab])
            tab_ok = obs.get("ok") is True and str(data(obs).get("tab_id")) == our_tab

        prot1 = protected(vcu(["browser", "tabs"]))
        ok = default_honest and tab_ok and prot0 == prot1
        report = {
            "ok": ok,
            "frontmost": front,
            "browser_front": browser_front,
            "default_ok": default.get("ok"),
            "default_error": default_msg,
            "default_honest": default_honest,
            "tab_id": our_tab,
            "tab_observe_ok": tab_ok,
            "groups_ok": prot0 == prot1,
        }
        if ok:
            print("CU-D-600 OK", "frontmost=", front, "default_ok=", default.get("ok"))
            return 0
        print("CU-D-600 FAIL", json.dumps(report, ensure_ascii=False))
        return 1
    finally:
        if our_tab and our_tab.isdigit():
            vcu(["browser", "close", "--tab", our_tab])
        if httpd is not None:
            httpd.shutdown()
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    sys.exit(main())
