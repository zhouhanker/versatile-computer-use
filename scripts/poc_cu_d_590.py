#!/usr/bin/env python3
"""CU-D-590: observe --tab live viewport click on a throwaway 127.0.0.1 page.

Opens a background tab in an existing USER window, observes that tab without
stealing OS frontmost, live-clicks the #hit button via observe capture_id,
checks 0→1, then closes the tab. Groups 1/3 untouched. Never Allow / OS cursor.
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
OUT = Path(".local/desktop-cu/cu-d-590.json")

HTML = b"""<!doctype html>
<meta charset="utf-8">
<title>vcu-d-590</title>
<style>
html,body{margin:0;background:#fff}
#hit{position:absolute;left:0;top:0;width:320px;height:120px;font:48px/120px system-ui;
text-align:center;background:#a83db3;color:#fff;border:0}
</style>
<button id="hit">0</button>
<script>
const b=document.getElementById('hit');
b.addEventListener('click',()=>{b.textContent=String((Number(b.textContent)||0)+1)});
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


def extract_text(tab):
    resp = vcu(["browser", "extract", "--tab", tab, "--selector", "#hit"])
    matches = data(resp).get("matches") or []
    text = ""
    if matches:
        text = str(matches[0].get("text") or matches[0].get("value") or "")
    return resp, text.strip()


def sidecar(path):
    p = Path(str(path))
    side = p.with_suffix(".json")
    if not side.is_file():
        return {}
    try:
        return json.loads(side.read_text())
    except json.JSONDecodeError:
        return {}


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
        url = f"http://127.0.0.1:{httpd.server_address[1]}/vcu-d-590?t={int(time.time())}"
        opened = vcu(["browser", "open", "--background", "--url", url])
        our_tab = str(data(opened).get("tab_id") or "")
        if opened.get("ok") is not True or not our_tab.isdigit():
            report.update({"error": opened.get("error"), "open": opened, "step": "open"})
            print("CU-D-590 FAIL open")
            return 1

        hit0 = None
        for _ in range(20):
            time.sleep(0.25)
            ex, text = extract_text(our_tab)
            if ex.get("ok") is True and text == "0":
                hit0 = text
                break
        if hit0 != "0":
            report.update({"error": "throwaway page did not extract #hit=0", "tab": our_tab})
            print("CU-D-590 FAIL extract0")
            return 1

        obs = vcu(["browser", "observe", "--tab", our_tab])
        od = data(obs)
        cid = str(od.get("capture_id") or (od.get("snapshot") or {}).get("capture_id") or "")
        shot = str(od.get("screenshot_path") or (od.get("snapshot") or {}).get("screenshot_path") or "")
        meta = sidecar(shot)
        width = float(meta.get("width") or od.get("screenshot_width") or 0)
        height = float(meta.get("height") or od.get("screenshot_height") or 0)
        css_w = float(((meta.get("viewport") or {}).get("width")) or 0)
        css_h = float(((meta.get("viewport") or {}).get("height")) or 0)
        if obs.get("ok") is not True or str(od.get("tab_id")) != our_tab or not cid:
            report.update({"error": obs.get("error"), "observe": od, "step": "observe"})
            print("CU-D-590 FAIL observe")
            return 1
        if width <= 0 or height <= 0 or css_w <= 0 or css_h <= 0:
            report.update({"error": "observe sidecar missing viewport size", "meta": meta, "shot": shot})
            print("CU-D-590 FAIL sidecar")
            return 1

        # CSS (160, 60) is the #hit button center (320x120 at origin).
        px = 160.0 * width / css_w
        py = 60.0 * height / css_h
        click = vcu([
            "browser", "click",
            "--space", "viewport",
            "--capture", cid,
            "--pixel-x", f"{px:.3f}",
            "--pixel-y", f"{py:.3f}",
        ])
        cd = data(click)
        if not (
            click.get("ok") is True
            and cd.get("dry_run") is not True
            and cd.get("source") == "extension_dom"
            and cd.get("os_cursor_used") is False
            and str(cd.get("tab_id")) == our_tab
        ):
            report.update({"error": click.get("error"), "click": cd, "step": "click", "px": px, "py": py})
            print("CU-D-590 FAIL click", click.get("error"))
            return 1

        hit1 = None
        for _ in range(12):
            time.sleep(0.15)
            ex, text = extract_text(our_tab)
            if ex.get("ok") is True:
                hit1 = text
                if text == "1":
                    break
        prot1 = protected(vcu(["browser", "tabs"]))
        ok = hit1 == "1" and prot0 == prot1 and cid and str(od.get("tab_id")) == our_tab
        report = {
            "ok": ok,
            "tab_id": our_tab,
            "capture_id": cid,
            "observe_source": od.get("source"),
            "frontmost_matched": od.get("frontmost_matched"),
            "click_source": cd.get("source"),
            "click_dry_run": cd.get("dry_run"),
            "hit0": hit0,
            "hit1": hit1,
            "px": px,
            "py": py,
            "groups_ok": prot0 == prot1,
            "os_cursor_used": cd.get("os_cursor_used"),
        }
        if ok:
            print("CU-D-590 OK", our_tab, "0->1")
            return 0
        print("CU-D-590 FAIL", report)
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
