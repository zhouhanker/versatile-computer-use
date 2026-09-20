#!/usr/bin/env python3
"""CU-D-400: dual-browser live lens hello + tabs merge.

Wakes Chrome and Edge MV3 workers by opening one 127.0.0.1 tab in each
already-running USER window, then checks tabs merge.
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


def health_browsers():
    st = vcu(["daemon", "status"])
    d = data(st)
    return list(d.get("extension_browsers") or []), int(d.get("extension_browser_count") or 0)


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
        body = b"<html><body>vcu-d-400</body></html>"
        self.send_response(200)
        self.send_header("Content-Type", "text/html")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        return


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

    hb, hc = health_browsers()
    step("health_before", True, {"extension_browsers": hb, "extension_browser_count": hc})

    before = vcu(["browser", "tabs"])
    prot_before = protected_snapshot(before)
    b0 = browsers_of(before)
    step(
        "tabs_before",
        before.get("ok") is True and data(before).get("source") == "extension_tabs",
        {"n_tabs": len(data(before).get("tabs") or []), "browsers": dict(b0), "protected": prot_before},
    )

    httpd = ThreadingHTTPServer(("127.0.0.1", 0), Wake)
    port = httpd.server_address[1]
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    wake_url = "http://127.0.0.1:%s%s" % (port, WAKE_PATH)
    apps = []
    if chrome_proc:
        apps.append("Google Chrome")
    if edge_proc:
        apps.append("Microsoft Edge")
    for app in apps:
        open_tab(app, wake_url)
        time.sleep(2)
    seen = []
    both = False
    for i in range(12):
        hb_i, hc_i = health_browsers()
        seen.append({"t": i, "browsers": hb_i, "count": hc_i})
        if hc_i >= 2 and "chrome" in hb_i and "edge" in hb_i:
            both = True
            break
        time.sleep(0.5)
    step("wake_open", True, {"url": wake_url, "apps": apps, "poll_samples": seen[-6:]})

    hb2, hc2 = health_browsers()
    step("health_after", hc2 >= 2, {"extension_browsers": hb2, "extension_browser_count": hc2})

    after = vcu(["browser", "tabs"])
    prot_after = protected_snapshot(after)
    b1 = browsers_of(after)
    tabs_ok = step(
        "tabs_after",
        after.get("ok") is True and data(after).get("source") == "extension_tabs",
        {"n_tabs": len(data(after).get("tabs") or []), "browsers": dict(b1), "protected": prot_after},
    )
    prot_ok = step("protected_groups_unchanged", prot_before == prot_after, {"before": prot_before, "after": prot_after})
    chrome_hello = step(
        "chrome_hello",
        b1.get("chrome", 0) > 0 or "chrome" in hb2,
        {"tabs": b1.get("chrome", 0), "health": hb2},
    )
    edge_hello = step(
        "edge_hello",
        b1.get("edge", 0) > 0 or "edge" in hb2,
        {"tabs": b1.get("edge", 0), "health": hb2},
    )
    merge_ok = step(
        "tabs_merge",
        chrome_hello and edge_hello and (b1.get("chrome", 0) > 0 and b1.get("edge", 0) > 0 or hc2 >= 2),
        {"browser_count": int(b1.get("chrome", 0) > 0) + int(b1.get("edge", 0) > 0), "browsers": dict(b1), "health": hb2},
    )
    cursor_ok = step("os_cursor", data(ping).get("os_cursor_used") is False, {})

    if wake_url:
        close_tab("Google Chrome", wake_url)
        close_tab("Microsoft Edge", wake_url)
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
