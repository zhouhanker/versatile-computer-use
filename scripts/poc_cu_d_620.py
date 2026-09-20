#!/usr/bin/env python3
"""CU-D-620: observe --tab then live hover + DOM wait bind last observe.

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
OUT = Path(".local/desktop-cu/cu-d-620.json")

HTML = b"""<!doctype html>
<meta charset="utf-8">
<title>vcu-d-620</title>
<style>html,body{margin:0;background:#fff}#pad{width:240px;height:80px;background:#a83db3;color:#fff}</style>
<div id="pad">pad</div>
<div id="hov">hovered=0</div>
<div id="ready">wait</div>
<script>
document.getElementById('pad').addEventListener('mouseover',()=>{
  document.getElementById('hov').textContent='hovered=1';
});
setTimeout(()=>{document.getElementById('ready').textContent='ready'},400);
</script>
"""


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


def extract_text(tab, selector):
    resp = vcu(["browser", "extract", "--tab", tab, "--selector", selector])
    matches = data(resp).get("matches") or []
    if not matches:
        return resp, ""
    m = matches[0] if isinstance(matches[0], dict) else {}
    return resp, str(m.get("text") or m.get("value") or "").strip()


def main():
    report = {"ok": False}
    our_tab = None
    httpd = None
    prot0 = protected(vcu(["browser", "tabs"]))
    try:
        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.send_header("Cache-Control", "no-store")
                self.end_headers()
                self.wfile.write(HTML)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        url = f"http://127.0.0.1:{httpd.server_address[1]}/vcu-d-620?t={int(time.time())}"
        opened = vcu(["browser", "open", "--background", "--url", url])
        our_tab = str(data(opened).get("tab_id") or "")
        if opened.get("ok") is not True or not our_tab.isdigit():
            report.update({"step": "open", "error": opened.get("error")})
            print("CU-D-620 FAIL open")
            return 1

        ready = False
        for _ in range(20):
            time.sleep(0.25)
            ex, text = extract_text(our_tab, "#pad")
            if ex.get("ok") is True and text:
                ready = True
                break
        if not ready:
            report.update({"step": "ready"})
            print("CU-D-620 FAIL ready")
            return 1

        obs = {"ok": False}
        od = {}
        for args in (
            ["browser", "observe", "--tab", our_tab],
            ["browser", "observe", "--tab", our_tab, "--pixels", "false"],
        ):
            obs = vcu(args)
            od = data(obs)
            if obs.get("ok") is True and str(od.get("tab_id")) == our_tab:
                break
        if obs.get("ok") is not True or str(od.get("tab_id")) != our_tab:
            report.update({"step": "observe", "error": obs.get("error")})
            print("CU-D-620 FAIL observe")
            return 1

        hovered = vcu(["browser", "hover", "--selector", "#pad"])
        hd = data(hovered)
        if not (
            hovered.get("ok") is True
            and hd.get("hovered") is True
            and hd.get("source") == "extension_dom"
            and hd.get("tab_id_source") == "last_observe"
            and str(hd.get("tab_id")) == our_tab
            and hd.get("os_cursor_used") is False
        ):
            report.update({"step": "hover", "hover": hd, "error": hovered.get("error")})
            print("CU-D-620 FAIL hover", hovered.get("error"))
            return 1

        hov = ""
        for _ in range(12):
            time.sleep(0.15)
            _ex, hov = extract_text(our_tab, "#hov")
            if hov == "hovered=1":
                break
        if hov != "hovered=1":
            report.update({"step": "extract_hover", "hov": hov})
            print("CU-D-620 FAIL extract hover", hov)
            return 1

        waited = vcu(["browser", "wait", "--selector", "#ready", "--text", "ready", "--ms", "2000"])
        wd = data(waited)
        if not (
            waited.get("ok") is True
            and wd.get("found") is True
            and wd.get("source") == "extension_dom"
            and wd.get("tab_id_source") == "last_observe"
            and str(wd.get("tab_id")) == our_tab
        ):
            report.update({"step": "wait", "wait": wd, "error": waited.get("error")})
            print("CU-D-620 FAIL wait", waited.get("error"))
            return 1

        miss = vcu(["browser", "wait", "--selector", "#missing", "--ms", "200"])
        miss_ok = miss.get("ok") is False and str((miss.get("error") or {}).get("code") or "") == "ActionFailed"

        prot1 = protected(vcu(["browser", "tabs"]))
        ok = miss_ok and prot0 == prot1 and hov == "hovered=1"
        report = {
            "ok": ok,
            "tab_id": our_tab,
            "hover_source": hd.get("source"),
            "hover_tab_source": hd.get("tab_id_source"),
            "hovered": hov,
            "wait_source": wd.get("source"),
            "wait_tab_source": wd.get("tab_id_source"),
            "waited_ms": wd.get("waited_ms"),
            "wait_miss_ok": miss_ok,
            "groups_ok": prot0 == prot1,
            "os_cursor_used": hd.get("os_cursor_used"),
        }
        if ok:
            print("CU-D-620 OK", our_tab, hov, "wait_ms", wd.get("waited_ms"))
            return 0
        print("CU-D-620 FAIL", json.dumps(report, ensure_ascii=False))
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
