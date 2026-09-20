#!/usr/bin/env python3
"""CU-D-660: hover/scroll/wait/screenshot --browser targets Chrome vs Edge.

Throwaway 127.0.0.1 only. Groups 1/3 untouched. Never Allow / OS cursor.
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
OUT = Path(".local/desktop-cu/cu-d-660.json")


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


def protected(tabs):
    out = {}
    for g in data(tabs).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            out[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    return out


def main():
    report = {"ok": False}
    opened = {}
    httpd = None
    prot0 = protected(vcu(["browser", "tabs"]))
    try:
        ready = False
        browsers = []
        for _ in range(20):
            ping = vcu(["browser", "ping"])
            st = vcu(["daemon", "status"])
            browsers = [str(b) for b in (data(st).get("extension_browsers") or [])]
            if ping.get("ok") is True and "chrome" in browsers and "edge" in browsers:
                ready = True
                break
            time.sleep(0.3)
        if not ready:
            report.update({"step": "health", "browsers": browsers})
            print("CU-D-660 FAIL need chrome+edge", browsers)
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                who = "edge" if "edge" in self.path else "chrome"
                html = (
                    "<!doctype html><title>vcu-d-660</title>"
                    f'<div id="who">{who}</div><div id="spacer" style="height:1800px"></div>'
                ).encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(html)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        port = httpd.server_address[1]
        for kind in ("chrome", "edge"):
            obs = vcu(["browser", "observe", "--browser", kind, "--pixels", "false"])
            if obs.get("ok") is not True:
                report.update({"step": f"observe-{kind}", "error": obs.get("error")})
                print("CU-D-660 FAIL observe", kind)
                return 1
            op = vcu(["browser", "open", "--background", "--url", f"http://127.0.0.1:{port}/{kind}?t={int(time.time())}"])
            tab = str(data(op).get("tab_id") or "")
            if op.get("ok") is not True or not tab.isdigit():
                report.update({"step": f"open-{kind}", "error": op.get("error")})
                print("CU-D-660 FAIL open", kind)
                return 1
            opened[kind] = tab
            time.sleep(0.4)

        chrome_tab = opened["chrome"]
        edge_tab = opened["edge"]
        wrong = vcu(["browser", "hover", "--selector", "#who", "--tab", chrome_tab, "--browser", "edge"])
        wrong_ok = wrong.get("ok") is False
        hovered = vcu(["browser", "hover", "--selector", "#who", "--tab", chrome_tab, "--browser", "chrome"])
        hd = data(hovered)
        scrolled = vcu(["browser", "scroll", "--dy", "400", "--tab", chrome_tab, "--browser", "chrome"])
        sd = data(scrolled)
        waited = vcu(["browser", "wait", "--selector", "#who", "--text", "edge", "--tab", edge_tab, "--browser", "edge", "--ms", "2000"])
        wd = data(waited)
        shot = vcu(["browser", "screenshot", "--tab", edge_tab, "--browser", "edge"])
        sh = data(shot)
        shot_ok = shot.get("ok") is True and str(sh.get("browser") or "") == "edge"
        ok = (
            wrong_ok
            and hovered.get("ok") is True
            and hd.get("hovered") is True
            and str(hd.get("browser") or "") == "chrome"
            and hd.get("os_cursor_used") is False
            and scrolled.get("ok") is True
            and sd.get("scrolled") is True
            and str(sd.get("browser") or "") == "chrome"
            and waited.get("ok") is True
            and wd.get("found") is True
            and str(wd.get("browser") or "") == "edge"
            and prot0 == protected(vcu(["browser", "tabs"]))
        )
        report = {
            "ok": ok,
            "chrome_tab": chrome_tab,
            "edge_tab": edge_tab,
            "wrong_denied": wrong_ok,
            "hover_browser": hd.get("browser"),
            "scroll_browser": sd.get("browser"),
            "wait_browser": wd.get("browser"),
            "shot_ok": shot_ok,
            "shot_error": (shot.get("error") or {}).get("message"),
            "groups_ok": prot0 == protected(vcu(["browser", "tabs"])),
        }
        if ok:
            print("CU-D-660 OK chrome", chrome_tab, "edge", edge_tab, "shot", shot_ok)
            return 0
        print("CU-D-660 FAIL", json.dumps(report, ensure_ascii=False))
        return 1
    finally:
        for kind, tab in list(opened.items()):
            vcu(["browser", "close", "--tab", tab, "--browser", kind])
        if httpd is not None:
            httpd.shutdown()
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    sys.exit(main())
