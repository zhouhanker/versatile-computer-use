#!/usr/bin/env python3
"""CU-D-400: dual-browser live lens hello + tabs merge.

May open one throwaway 127.0.0.1 tab in Chrome and/or Edge to wake MV3 SW.
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
OUT = Path(".local/desktop-cu/cu-d-400.json")
VCU = os.environ.get("VCU", "vcu")
WAKE_PATH = "/vcu-d-400-wake"


def vcu(args):
    p = subprocess.run([VCU, *args, "--json"], capture_output=True, text=True)
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


def close_app_url(app, url):
    script = chr(10).join([
        "tell application \"" + app + "\"",
        "  set theUrl to \"" + url + "\"",
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
    try:
        subprocess.run(["osascript", "-e", script], capture_output=True, text=True, timeout=5)
    except Exception:
        pass


def main():
    report = {"ok": False, "steps": []}
    httpd = None
    wake_url = None

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        print(("OK  " if ok else "FAIL") + name, extra or "")
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)

    ping = vcu(["browser", "ping"])
    ext = data(ping).get("extension") or {}
    ping_ok = step(
        "ping",
        ping.get("ok") is True and data(ping).get("pong") is True,
        {"version": ext.get("version"), "os_cursor_used": data(ping).get("os_cursor_used")},
    )

    login = vcu(["browser", "login-state"])
    users = data(login).get("user_browsers") or []
    names = {str(b.get("name") or "") for b in users}
    chrome_proc = any(n == "Chrome" or n.endswith("Chrome") for n in names)
    edge_proc = any("Edge" in n for n in names)
    step(
        "login_processes",
        login.get("ok") is True and chrome_proc and edge_proc,
        {"names": sorted(names), "never_click_allow": data(login).get("never_click_allow")},
    )

    before = vcu(["browser", "tabs"])
    prot_before = protected_snapshot(before)
    b0 = browsers_of(before)
    step(
        "tabs_before",
        before.get("ok") is True and data(before).get("source") == "extension_tabs",
        {"n_tabs": len(data(before).get("tabs") or []), "browsers": dict(b0), "protected": prot_before},
    )

    missing = []
    if b0.get("chrome", 0) == 0 and chrome_proc:
        missing.append("Google Chrome")
    if b0.get("edge", 0) == 0 and edge_proc:
        missing.append("Microsoft Edge")
    if missing:
        class Wake(BaseHTTPRequestHandler):
            def do_GET(self):
                body = b"<html><body>vcu-d-400</body></html>"
                self.send_response(200)
                self.send_header("Content-Type", "text/html")
                self.send_header("Content-Length", str(len(body)))
                self.end_headers()
                self.wfile.write(body)
            def log_message(self, fmt, *args):
                return
        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Wake)
        port = httpd.server_address[1]
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        wake_url = "http://127.0.0.1:%s%s" % (port, WAKE_PATH)
        for app in missing:
            subprocess.run(["open", "-a", app, wake_url], capture_output=True, text=True, check=False)
        time.sleep(3.5)
        step("wake_open", True, {"url": wake_url, "apps": missing})

    after = vcu(["browser", "tabs"])
    prot_after = protected_snapshot(after)
    b1 = browsers_of(after)
    tabs_ok = step(
        "tabs_after",
        after.get("ok") is True and data(after).get("source") == "extension_tabs",
        {"n_tabs": len(data(after).get("tabs") or []), "browsers": dict(b1), "protected": prot_after},
    )
    prot_ok = step("protected_groups_unchanged", prot_before == prot_after, {"before": prot_before, "after": prot_after})
    chrome_hello = step("chrome_hello", b1.get("chrome", 0) > 0, {"count": b1.get("chrome", 0)})
    edge_hello = step("edge_hello", b1.get("edge", 0) > 0, {"count": b1.get("edge", 0)})
    merge_ok = step(
        "tabs_merge",
        chrome_hello and edge_hello,
        {"browser_count": int(b1.get("chrome", 0) > 0) + int(b1.get("edge", 0) > 0), "browsers": dict(b1)},
    )
    cursor_ok = step("os_cursor", data(ping).get("os_cursor_used") is False, {})

    if wake_url:
        close_app_url("Google Chrome", wake_url)
        close_app_url("Microsoft Edge", wake_url)
        if httpd:
            httpd.shutdown()
        step("wake_closed", True, {"url": wake_url})

    report["ok"] = all([ping_ok, tabs_ok, prot_ok, chrome_hello, edge_hello, merge_ok, cursor_ok])
    OUT.write_text(json.dumps(report, indent=2) + "\n")
    if report["ok"]:
        print("CHROME_HELLO_OK")
        print("EDGE_HELLO_OK")
        print("MERGE_OK browser_count=2")
        print("CU-D-400 OK")
        return 0
    print("CU-D-400 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
