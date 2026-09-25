#!/usr/bin/env python3
"""CU-D-630: observe --browser directs Chrome vs Edge last observe.

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
OUT = Path(".local/desktop-cu/cu-d-630.json")


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


def extract_who(tab=None):
    args = ["browser", "extract", "--selector", "#who"]
    if tab:
        args += ["--tab", tab]
    resp = vcu(args)
    matches = data(resp).get("matches") or []
    text = ""
    if matches and isinstance(matches[0], dict):
        text = str(matches[0].get("text") or matches[0].get("value") or "").strip()
    return resp, text, str(data(resp).get("tab_id") or ""), str(data(resp).get("tab_id_source") or "")


def main():
    report = {"ok": False}
    opened = []
    httpd = None
    prot0 = protected(vcu(["browser", "tabs"]))
    try:
        st = vcu(["daemon", "status"])
        browsers = [str(b) for b in (data(st).get("extension_browsers") or [])]
        if "chrome" not in browsers or "edge" not in browsers:
            report.update({"step": "health", "browsers": browsers})
            print("CU-D-630 FAIL need chrome+edge lens", browsers)
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                who = "edge" if "edge" in self.path else "chrome"
                body = (
                    b"<!doctype html><title>vcu-d-630</title>"
                    + f'<div id="who">{who}</div>'.encode()
                )
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        port = httpd.server_address[1]
        results = {}
        for kind in ("chrome", "edge"):
            obs = vcu(["browser", "observe", "--browser", kind, "--pixels", "false"])
            od = data(obs)
            if obs.get("ok") is not True or str(od.get("browser") or "") != kind:
                report.update({"step": f"observe-{kind}", "error": obs.get("error"), "data": od})
                print("CU-D-630 FAIL observe", kind, obs.get("error"))
                return 1
            url = f"http://127.0.0.1:{port}/{kind}?t={int(time.time())}"
            op = vcu(["browser", "open", "--background", "--url", url])
            tab = str(data(op).get("tab_id") or "")
            if op.get("ok") is not True or not tab.isdigit():
                report.update({"step": f"open-{kind}", "error": op.get("error")})
                print("CU-D-630 FAIL open", kind)
                return 1
            opened.append(tab)
            time.sleep(0.4)
            obs2 = vcu(["browser", "observe", "--tab", tab, "--browser", kind, "--pixels", "false"])
            od2 = data(obs2)
            if obs2.get("ok") is not True or str(od2.get("tab_id")) != tab or str(od2.get("browser") or "") != kind:
                report.update({"step": f"observe-tab-{kind}", "error": obs2.get("error"), "data": od2})
                print("CU-D-630 FAIL observe --tab", kind)
                return 1
            who_resp, who, who_tab, who_src = extract_who()
            if who_resp.get("ok") is not True or who != kind or who_src != "last_observe" or who_tab != tab:
                report.update({
                    "step": f"extract-{kind}",
                    "who": who,
                    "tab": who_tab,
                    "src": who_src,
                    "error": who_resp.get("error"),
                })
                print("CU-D-630 FAIL extract", kind, who, who_src)
                return 1
            results[kind] = {"tab_id": tab, "who": who, "observe_browser": od2.get("browser")}
            vcu(["browser", "close", "--tab", tab])
            opened.remove(tab)

        if len(opened) == 2 and opened[0] == opened[1]:
            amb = vcu(["browser", "observe", "--tab", opened[0], "--pixels", "false"])
            amsg = str((amb.get("error") or {}).get("message") or "")
            amb_ok = amb.get("ok") is False and "ambiguous" in amsg
        else:
            amb_ok = True

        prot1 = protected(vcu(["browser", "tabs"]))
        ok = (
            results.get("chrome", {}).get("who") == "chrome"
            and results.get("edge", {}).get("who") == "edge"
            and amb_ok
            and prot0 == prot1
        )
        report = {
            "ok": ok,
            "browsers": browsers,
            "results": results,
            "ids_collided": opened[0] == opened[1] if len(opened) == 2 else False,
            "ambiguous_ok": amb_ok,
            "groups_ok": prot0 == prot1,
        }
        if ok:
            print("CU-D-630 OK chrome", results["chrome"]["tab_id"], "edge", results["edge"]["tab_id"])
            return 0
        print("CU-D-630 FAIL", json.dumps(report, ensure_ascii=False))
        return 1
    finally:
        for tab in opened:
            vcu(["browser", "close", "--tab", tab])
        if httpd is not None:
            httpd.shutdown()
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    sys.exit(main())
