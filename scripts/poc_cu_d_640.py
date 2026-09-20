#!/usr/bin/env python3
"""CU-D-640: close --browser targets Chrome vs Edge.

Throwaway 127.0.0.1 only. Groups 1/3 untouched. Never Allow / OS cursor.
Does not call select (would steal OS window focus).
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
OUT = Path(".local/desktop-cu/cu-d-640.json")


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


def tab_ids(kind=None):
    tabs = data(vcu(["browser", "tabs"])).get("tabs") or []
    out = []
    for t in tabs:
        if kind and str(t.get("browser") or "") != kind:
            continue
        out.append(str(t.get("tab_id") or ""))
    return [x for x in out if x]


def main():
    report = {"ok": False}
    opened = {}
    httpd = None
    prot0 = protected(vcu(["browser", "tabs"]))
    try:
        st = vcu(["daemon", "status"])
        browsers = [str(b) for b in (data(st).get("extension_browsers") or [])]
        if "chrome" not in browsers or "edge" not in browsers:
            report.update({"step": "health", "browsers": browsers})
            print("CU-D-640 FAIL need chrome+edge lens", browsers)
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                who = "edge" if "edge" in self.path else "chrome"
                body = b"<!doctype html><title>vcu-d-640</title>" + f'<div id="who">{who}</div>'.encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        port = httpd.server_address[1]
        for kind in ("chrome", "edge"):
            obs = vcu(["browser", "observe", "--browser", kind, "--pixels", "false"])
            if obs.get("ok") is not True:
                report.update({"step": f"observe-{kind}", "error": obs.get("error")})
                print("CU-D-640 FAIL observe", kind)
                return 1
            url = f"http://127.0.0.1:{port}/{kind}?t={int(time.time())}"
            op = vcu(["browser", "open", "--background", "--url", url])
            tab = str(data(op).get("tab_id") or "")
            if op.get("ok") is not True or not tab.isdigit():
                report.update({"step": f"open-{kind}", "error": op.get("error")})
                print("CU-D-640 FAIL open", kind)
                return 1
            opened[kind] = tab
            time.sleep(0.3)

        chrome_tab = opened["chrome"]
        edge_tab = opened["edge"]
        wrong = vcu(["browser", "close", "--tab", chrome_tab, "--browser", "edge"])
        wrong_ok = wrong.get("ok") is False
        closed_chrome = vcu(["browser", "close", "--tab", chrome_tab, "--browser", "chrome"])
        cd = data(closed_chrome)
        chrome_closed = (
            closed_chrome.get("ok") is True
            and cd.get("closed") is True
            and str(cd.get("browser") or "") == "chrome"
            and chrome_tab not in tab_ids("chrome")
            and edge_tab in tab_ids("edge")
        )
        closed_edge = vcu(["browser", "close", "--tab", edge_tab, "--browser", "edge"])
        ed = data(closed_edge)
        edge_closed = (
            closed_edge.get("ok") is True
            and ed.get("closed") is True
            and str(ed.get("browser") or "") == "edge"
            and edge_tab not in tab_ids("edge")
        )
        if chrome_closed:
            opened.pop("chrome", None)
        if edge_closed:
            opened.pop("edge", None)
        prot1 = protected(vcu(["browser", "tabs"]))
        ok = wrong_ok and chrome_closed and edge_closed and prot0 == prot1
        report = {
            "ok": ok,
            "chrome_tab": chrome_tab,
            "edge_tab": edge_tab,
            "wrong_browser_denied": wrong_ok,
            "wrong_error": (wrong.get("error") or {}).get("message"),
            "chrome_closed": chrome_closed,
            "edge_closed": edge_closed,
            "groups_ok": prot0 == prot1,
        }
        if ok:
            print("CU-D-640 OK chrome", chrome_tab, "edge", edge_tab)
            return 0
        print("CU-D-640 FAIL", json.dumps(report, ensure_ascii=False))
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
