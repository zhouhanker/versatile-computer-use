#!/usr/bin/env python3
"""CU-D-610: observe --tab then live type/scroll bind last observe.

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
OUT = Path(".local/desktop-cu/cu-d-610.json")
MARKER = "vcu-d-610"

HTML = b"""<!doctype html>
<meta charset="utf-8">
<title>vcu-d-610</title>
<style>html,body{margin:0;background:#fff}#spacer{height:2400px}</style>
<input id="entry" value="">
<div id="typed">typed=</div>
<div id="scroll">y=0</div>
<div id="spacer"></div>
<div id="bottom">bottom</div>
<script>
const entry=document.getElementById('entry');
const typed=document.getElementById('typed');
const scroll=document.getElementById('scroll');
entry.addEventListener('input',()=>{typed.textContent='typed='+entry.value});
setInterval(()=>{scroll.textContent='y='+Math.round(window.scrollY||document.documentElement.scrollTop||0)}, 50);
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


def extract_match(tab, selector):
    resp = vcu(["browser", "extract", "--tab", tab, "--selector", selector])
    matches = data(resp).get("matches") or []
    if not matches:
        return resp, "", ""
    m = matches[0] if isinstance(matches[0], dict) else {}
    return resp, str(m.get("text") or "").strip(), str(m.get("value") or "").strip()


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
        url = f"http://127.0.0.1:{httpd.server_address[1]}/vcu-d-610?t={int(time.time())}"
        opened = vcu(["browser", "open", "--background", "--url", url])
        our_tab = str(data(opened).get("tab_id") or "")
        if opened.get("ok") is not True or not our_tab.isdigit():
            report.update({"step": "open", "error": opened.get("error")})
            print("CU-D-610 FAIL open")
            return 1

        ready = False
        for _ in range(20):
            time.sleep(0.25)
            ex, _text, _val = extract_match(our_tab, "#entry")
            if ex.get("ok") is True:
                ready = True
                break
        if not ready:
            report.update({"step": "ready", "tab": our_tab})
            print("CU-D-610 FAIL ready")
            return 1

        obs = {"ok": False}
        od = {}
        for _ in range(4):
            obs = vcu(["browser", "observe", "--tab", our_tab])
            od = data(obs)
            if obs.get("ok") is True and str(od.get("tab_id")) == our_tab:
                break
            time.sleep(0.4)
        if obs.get("ok") is not True or str(od.get("tab_id")) != our_tab:
            obs = vcu(["browser", "observe", "--tab", our_tab, "--pixels", "false"])
            od = data(obs)
        cid = str(od.get("capture_id") or "")
        if obs.get("ok") is not True or str(od.get("tab_id")) != our_tab:
            report.update({"step": "observe", "error": obs.get("error"), "observe": od})
            print("CU-D-610 FAIL observe")
            return 1

        typed = vcu(["browser", "type", "--selector", "#entry", "--text", MARKER])
        td = data(typed)
        if not (
            typed.get("ok") is True
            and td.get("typed") is True
            and td.get("source") == "extension_dom"
            and td.get("tab_id_source") == "last_observe"
            and str(td.get("tab_id")) == our_tab
            and td.get("os_cursor_used") is False
        ):
            report.update({"step": "type", "type": td, "error": typed.get("error")})
            print("CU-D-610 FAIL type", typed.get("error"))
            return 1

        got_value = ""
        for _ in range(12):
            time.sleep(0.15)
            _ex, text, value = extract_match(our_tab, "#entry")
            got_value = value or text
            if MARKER in got_value:
                break
        if MARKER not in got_value:
            report.update({"step": "extract_type", "value": got_value})
            print("CU-D-610 FAIL extract type", got_value)
            return 1

        scrolled = vcu(["browser", "scroll", "--dy", "900"])
        sd = data(scrolled)
        if not (
            scrolled.get("ok") is True
            and sd.get("scrolled") is True
            and sd.get("source") == "extension_dom"
            and sd.get("tab_id_source") == "last_observe"
            and str(sd.get("tab_id")) == our_tab
            and sd.get("os_cursor_used") is False
        ):
            report.update({"step": "scroll", "scroll": sd, "error": scrolled.get("error")})
            print("CU-D-610 FAIL scroll", scrolled.get("error"))
            return 1

        y_text = ""
        for _ in range(12):
            time.sleep(0.15)
            _ex, text, _value = extract_match(our_tab, "#scroll")
            y_text = text
            if text.startswith("y=") and text != "y=0":
                break
        y_ok = y_text.startswith("y=") and y_text != "y=0"

        capture_still = True
        if cid:
            click = vcu([
                "browser", "click",
                "--space", "viewport",
                "--capture", cid,
                "--pixel-x", "1",
                "--pixel-y", "1",
                "--dry-run",
            ])
            cd = data(click)
            capture_still = (
                click.get("ok") is True
                and cd.get("dry_run") is True
                and cd.get("source") == "extension_dom"
            )

        prot1 = protected(vcu(["browser", "tabs"]))
        ok = y_ok and capture_still and prot0 == prot1 and MARKER in got_value
        report = {
            "ok": ok,
            "tab_id": our_tab,
            "capture_id": cid,
            "type_source": td.get("source"),
            "type_tab_source": td.get("tab_id_source"),
            "typed_value": got_value,
            "scroll_source": sd.get("source"),
            "scroll_tab_source": sd.get("tab_id_source"),
            "scroll_y": y_text,
            "capture_dry_run_ok": capture_still,
            "groups_ok": prot0 == prot1,
            "os_cursor_used": td.get("os_cursor_used") or sd.get("os_cursor_used"),
        }
        if ok:
            print("CU-D-610 OK", our_tab, got_value, y_text)
            return 0
        print("CU-D-610 FAIL", json.dumps(report, ensure_ascii=False))
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
