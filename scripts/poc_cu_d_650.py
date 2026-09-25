#!/usr/bin/env python3
"""CU-D-650: extract/type/click --browser targets Chrome vs Edge.

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
OUT = Path(".local/desktop-cu/cu-d-650.json")


def vcu(args):
    cmd = [VCU, *args]
    if "--json" not in cmd:
        cmd.append("--json")
    p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
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


def extract_who(tab, browser):
    resp = vcu(["browser", "extract", "--selector", "#who", "--tab", tab, "--browser", browser])
    matches = data(resp).get("matches") or []
    text = ""
    if matches and isinstance(matches[0], dict):
        text = str(matches[0].get("text") or "").strip()
    return resp, text, str(data(resp).get("browser") or "")


def main():
    report = {"ok": False}
    opened = {}
    httpd = None
    prot0 = protected(vcu(["browser", "tabs"]))
    try:
        st = vcu(["daemon", "status"])
        browsers = [str(b) for b in (data(st).get("extension_browsers") or [])]
        if "chrome" not in browsers or "edge" not in browsers:
            print("CU-D-650 FAIL need chrome+edge", browsers)
            report.update({"step": "health", "browsers": browsers})
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                who = "edge" if "edge" in self.path else "chrome"
                html = (
                    "<!doctype html><title>vcu-d-650</title>"
                    f'<div id="who">{who}</div><input id="entry" value="">'
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
                print("CU-D-650 FAIL observe", kind)
                return 1
            op = vcu(["browser", "open", "--background", "--url", f"http://127.0.0.1:{port}/{kind}?t={int(time.time())}"])
            tab = str(data(op).get("tab_id") or "")
            if op.get("ok") is not True or not tab.isdigit():
                report.update({"step": f"open-{kind}", "error": op.get("error")})
                print("CU-D-650 FAIL open", kind)
                return 1
            opened[kind] = tab
            time.sleep(0.4)

        chrome_tab = opened["chrome"]
        edge_tab = opened["edge"]
        wrong = vcu(["browser", "extract", "--selector", "#who", "--tab", chrome_tab, "--browser", "edge"])
        wrong_ok = wrong.get("ok") is False
        ch, who_c, b_c = extract_who(chrome_tab, "chrome")
        ed, who_e, b_e = extract_who(edge_tab, "edge")
        typed = vcu(["browser", "type", "--selector", "#entry", "--text", "vcu-d-650", "--tab", chrome_tab, "--browser", "chrome"])
        td = data(typed)
        clicked = vcu(["browser", "click", "--selector", "#who", "--tab", edge_tab, "--browser", "edge", "--dry-run"])
        cd = data(clicked)
        ok = (
            wrong_ok
            and ch.get("ok") is True and who_c == "chrome" and b_c == "chrome"
            and ed.get("ok") is True and who_e == "edge" and b_e == "edge"
            and typed.get("ok") is True and td.get("typed") is True and str(td.get("browser") or "") == "chrome"
            and clicked.get("ok") is True and cd.get("dry_run") is True and str(cd.get("browser") or "") == "edge"
            and cd.get("os_cursor_used") is False
        )
        prot1 = protected(vcu(["browser", "tabs"]))
        ok = ok and prot0 == prot1
        report = {
            "ok": ok,
            "chrome_tab": chrome_tab,
            "edge_tab": edge_tab,
            "wrong_denied": wrong_ok,
            "who_chrome": who_c,
            "who_edge": who_e,
            "typed": td.get("typed"),
            "click_dry_run": cd.get("dry_run"),
            "groups_ok": prot0 == prot1,
        }
        if ok:
            print("CU-D-650 OK chrome", chrome_tab, "edge", edge_tab)
            return 0
        print("CU-D-650 FAIL", json.dumps(report, ensure_ascii=False))
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
