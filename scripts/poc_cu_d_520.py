#!/usr/bin/env python3
"""CU-D-520: dual-browser tabs include Edge while lens is alive.

If Edge worker is only hellos (not a poll loop), open one throwaway 127.0.0.1
tab in the existing USER Edge window to start polling, then close it.
Does not touch USER groups titled 1 / 3. Never clicks Allow. Never warps OS cursor.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import threading
import time
from collections import Counter
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

PROTECTED = {"1", "3"}
OUT = Path(".local/desktop-cu/cu-d-520.json")
VCU = os.environ.get("VCU", "vcu")
WAKE_PATH = "/vcu-d-520-wake"


def vcu(args):
    cmd = [VCU, *args]
    if "--json" not in cmd:
        cmd.append("--json")
    if "--user-dir" not in cmd and os.environ.get("VCU_DIR"):
        cmd[1:1] = ["--user-dir", os.environ["VCU_DIR"]]
    p = subprocess.run(cmd, capture_output=True, text=True)
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:800], "code": p.returncode}


def data(resp):
    return resp.get("data") if isinstance(resp.get("data"), dict) else {}


def protected_snapshot(tabs_resp):
    out = {}
    for g in data(tabs_resp).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            ids = g.get("tab_ids") or g.get("tabs") or []
            out[title] = sorted(str(x) for x in ids)
    return out


def browsers_of(tabs_resp):
    tabs = data(tabs_resp).get("tabs") or []
    return Counter(str(t.get("browser") or "missing") for t in tabs)


def osascript(script):
    try:
        subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=6)
    except Exception:
        pass


def open_tab(app, url):
    subprocess.run(["open", "-a", app, url], capture_output=True, text=True, check=False)


def close_tab(app, url):
    nl = chr(10)
    script = nl.join([
        'tell application "' + app + '"',
        '  set theUrl to "' + url + '"',
        "  repeat with w in windows",
        "    set tabList to tabs of w",
        "    repeat with t in tabList",
        "      try",
        "        if (URL of t) starts with theUrl then",
        "          close t",
        "        end if",
        "      end try",
        "    end repeat",
        "  end repeat",
        "end tell",
        "",
    ])
    osascript(script)


class Wake(BaseHTTPRequestHandler):
    def do_GET(self):
        body = b"<html><body>vcu-d-520</body></html>"
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        return


def main():
    httpd = None
    wake_url = None
    steps = []
    before = vcu(["browser", "tabs"])
    prot_before = protected_snapshot(before)
    counts = browsers_of(before)
    bd = data(before)
    steps.append({
        "tabs_before": True,
        "counts": dict(counts),
        "browser_count": bd.get("browser_count"),
        "browsers_failed": bd.get("browsers_failed"),
    })

    if counts.get("edge", 0) < 1:
        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Wake)
        port = httpd.server_address[1]
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        wake_url = "http://127.0.0.1:%s%s" % (port, WAKE_PATH)
        open_tab("Microsoft Edge", wake_url)
        steps.append({"wake_open": True, "url": wake_url})
        time.sleep(3)
        before = vcu(["browser", "tabs"])
        counts = browsers_of(before)
        bd = data(before)
        steps.append({
            "tabs_after_wake": True,
            "counts": dict(counts),
            "browsers_failed": bd.get("browsers_failed"),
            "browser_count": bd.get("browser_count"),
        })

    chrome_n = int(counts.get("chrome") or 0)
    edge_n = int(counts.get("edge") or 0)
    merge_ok = (
        int(bd.get("browser_count") or 0) >= 2
        and chrome_n >= 1
        and edge_n >= 1
    )
    after = vcu(["browser", "tabs"]) if wake_url else before
    prot_after = protected_snapshot(after)
    groups_ok = prot_before == prot_after
    if wake_url:
        close_tab("Microsoft Edge", wake_url)
        if httpd:
            httpd.shutdown()
        steps.append({"wake_closed": True, "url": wake_url})
        prot_closed = protected_snapshot(vcu(["browser", "tabs"]))
        groups_ok = groups_ok and prot_before == prot_closed
        prot_after = prot_closed
    ok = merge_ok and groups_ok and (before.get("ok") is True)
    report = {
        "ok": ok,
        "CHROME_TABS_OK": chrome_n >= 1,
        "EDGE_TABS_OK": edge_n >= 1,
        "MERGE_OK": merge_ok,
        "GROUPS_OK": groups_ok,
        "chrome_tabs": chrome_n,
        "edge_tabs": edge_n,
        "protected": prot_after,
        "steps": steps,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    if chrome_n >= 1:
        print("CHROME_TABS_OK count=%s" % chrome_n)
    else:
        print("CHROME_TABS_MISSING")
    if edge_n >= 1:
        print("EDGE_TABS_OK count=%s" % edge_n)
    else:
        print("EDGE_TABS_MISSING failed=%s" % (bd.get("browsers_failed"),))
    if merge_ok:
        print("MERGE_OK browser_count=%s" % bd.get("browser_count"))
    else:
        print("MERGE_FAIL", dict(counts), bd.get("browser_count"), bd.get("browsers_failed"))
    if groups_ok:
        print("GROUPS_OK")
    else:
        print("GROUPS_CHANGED", prot_before, prot_after)
    if ok:
        print("CU-D-520 OK")
        return 0
    print("CU-D-520 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
