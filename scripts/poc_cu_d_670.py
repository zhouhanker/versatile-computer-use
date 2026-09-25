#!/usr/bin/env python3
"""CU-D-670: open --browser and group same-browser tabs.

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
OUT = Path(".local/desktop-cu/cu-d-670.json")
TITLE = "vcu-d-670"


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
    return text, str(data(resp).get("browser") or "")


def main():
    report = {"ok": False}
    opened = []
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
            print("CU-D-670 FAIL need chrome+edge", browsers)
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                who = "edge" if "edge" in self.path else "chrome"
                html = f"<!doctype html><title>vcu-d-670</title><div id='who'>{who}</div>".encode()
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(html)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        port = httpd.server_address[1]
        chrome_tabs = []
        for i in range(2):
            op = vcu(["browser", "open", "--background", "--browser", "chrome", "--url", f"http://127.0.0.1:{port}/chrome?i={i}&t={int(time.time())}"])
            tab = str(data(op).get("tab_id") or "")
            br = str(data(op).get("browser") or "")
            if op.get("ok") is not True or not tab.isdigit() or br != "chrome":
                report.update({"step": "open-chrome", "error": op.get("error"), "data": data(op)})
                print("CU-D-670 FAIL open chrome", op.get("error"))
                return 1
            chrome_tabs.append(tab)
            opened.append(("chrome", tab))
            time.sleep(0.35)
        edge_op = vcu(["browser", "open", "--background", "--browser", "edge", "--url", f"http://127.0.0.1:{port}/edge?t={int(time.time())}"])
        edge_tab = str(data(edge_op).get("tab_id") or "")
        if edge_op.get("ok") is not True or str(data(edge_op).get("browser") or "") != "edge":
            report.update({"step": "open-edge", "error": edge_op.get("error")})
            print("CU-D-670 FAIL open edge")
            return 1
        opened.append(("edge", edge_tab))
        time.sleep(0.4)
        who_c, b_c = extract_who(chrome_tabs[0], "chrome")
        who_e, b_e = extract_who(edge_tab, "edge")
        mixed = vcu(["browser", "group", "--tabs", f"{chrome_tabs[0]},{edge_tab}", "--title", TITLE])
        mixed_ok = mixed.get("ok") is False
        grouped = vcu(["browser", "group", "--tabs", f"{chrome_tabs[0]},{chrome_tabs[1]}", "--title", TITLE, "--browser", "chrome", "--color", "blue"])
        gd = data(grouped)
        group_ok = grouped.get("ok") is True and str(gd.get("browser") or "") == "chrome"
        ungrouped = vcu(["browser", "ungroup", "--tabs", f"{chrome_tabs[0]},{chrome_tabs[1]}", "--browser", "chrome"])
        ug_ok = ungrouped.get("ok") is True
        prot1 = protected(vcu(["browser", "tabs"]))
        ok = (
            who_c == "chrome"
            and b_c == "chrome"
            and who_e == "edge"
            and b_e == "edge"
            and mixed_ok
            and group_ok
            and ug_ok
            and prot0 == prot1
        )
        report = {
            "ok": ok,
            "chrome_tabs": chrome_tabs,
            "edge_tab": edge_tab,
            "who_chrome": who_c,
            "who_edge": who_e,
            "mixed_denied": mixed_ok,
            "mixed_error": (mixed.get("error") or {}).get("message"),
            "group_ok": group_ok,
            "ungroup_ok": ug_ok,
            "groups_ok": prot0 == prot1,
        }
        if ok:
            print("CU-D-670 OK chrome", chrome_tabs, "edge", edge_tab)
            return 0
        print("CU-D-670 FAIL", json.dumps(report, ensure_ascii=False))
        return 1
    finally:
        for kind, tab in opened:
            vcu(["browser", "close", "--tab", tab, "--browser", kind])
        if httpd is not None:
            httpd.shutdown()
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    sys.exit(main())
