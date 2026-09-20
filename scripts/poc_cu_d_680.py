#!/usr/bin/env python3
"""CU-D-680: group-update --browser targets the right browser when group ids collide.

Throwaway 127.0.0.1 tabs only. Native groups 1/3 untouched. Never Allow / OS cursor.
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
OUT = Path(".local/desktop-cu/cu-d-680.json")
CHROME_TITLE = "vcu-d-680-chrome"
EDGE_TITLE = "vcu-d-680-edge"


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


def err_message(resp):
    err = resp.get("error") or {}
    parts = [err.get("message"), err.get("detail"), data(resp).get("error")]
    return " ".join(str(p) for p in parts if p)


def groups_by_browser():
    merged = {"chrome": {}, "edge": {}}
    for g in data(vcu(["browser", "tabs"])).get("groups") or []:
        browser = str(g.get("browser") or "")
        gid = str(g.get("group_id") or "")
        if browser in merged and gid:
            merged[browser][gid] = g
    return merged


def protected(groups_resp):
    out = {}
    for g in data(groups_resp).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            out[title] = str(g.get("group_id") or "")
    return out


def numeric(gid):
    try:
        return int(gid)
    except (TypeError, ValueError):
        return None


def open_pair(port, browser, stamp):
    tabs = []
    for i in range(2):
        op = vcu(
            [
                "browser",
                "open",
                "--background",
                "--browser",
                browser,
                "--url",
                f"http://127.0.0.1:{port}/{browser}?i={i}&t={stamp}",
            ]
        )
        tab = str(data(op).get("tab_id") or "")
        if op.get("ok") is not True or not tab:
            return None, op
        tabs.append(tab)
        time.sleep(0.35)
    return tabs, None


def group_pair(browser, tabs, title, color):
    resp = vcu(
        [
            "browser",
            "group",
            "--tabs",
            ",".join(tabs),
            "--browser",
            browser,
            "--title",
            title,
            "--color",
            color,
        ]
    )
    payload = data(resp)
    gid = str(payload.get("group_id") or "")
    if not gid:
        gid = str((payload.get("group") or {}).get("group_id") or "")
    if not gid:
        groups = payload.get("groups") or []
        if groups and isinstance(groups[0], dict):
            gid = str(groups[0].get("group_id") or "")
    return resp, gid


def main():
    report = {"ok": False}
    opened = []
    httpd = None
    before_protected = protected(vcu(["browser", "tabs"]))
    try:
        ready = False
        browsers = []
        for _ in range(25):
            ping = vcu(["browser", "ping"])
            status = vcu(["daemon", "status"])
            browsers = [str(b) for b in (data(status).get("extension_browsers") or [])]
            if ping.get("ok") is True and "chrome" in browsers and "edge" in browsers:
                ready = True
                break
            time.sleep(0.4)
        if not ready:
            report.update({"step": "health", "browsers": browsers})
            print("CU-D-680 FAIL need chrome+edge", browsers)
            return 1

        class Handler(BaseHTTPRequestHandler):
            def do_GET(self):
                body = b"<!doctype html><title>vcu-d-680</title><div id='who'>d680</div>"
                self.send_response(200)
                self.send_header("Content-Type", "text/html; charset=utf-8")
                self.end_headers()
                self.wfile.write(body)

            def log_message(self, *_args):
                return

        httpd = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        threading.Thread(target=httpd.serve_forever, daemon=True).start()
        port = httpd.server_address[1]
        stamp = int(time.time())

        pairs = {}
        for browser, title, color in (
            ("chrome", CHROME_TITLE, "blue"),
            ("edge", EDGE_TITLE, "red"),
        ):
            tabs, failure = open_pair(port, browser, stamp)
            if failure is not None:
                report.update({"step": f"open-{browser}", "error": failure.get("error")})
                print("CU-D-680 FAIL open", browser, failure.get("error"))
                return 1
            opened.extend((browser, tab) for tab in tabs)
            grouped, gid = group_pair(browser, tabs, title, color)
            if grouped.get("ok") is not True or not gid:
                report.update({"step": f"group-{browser}", "error": grouped.get("error")})
                print("CU-D-680 FAIL group", browser, grouped.get("error"), data(grouped))
                return 1
            pairs[browser] = {"tabs": tabs, "gid": gid, "color": color, "title": title}
            time.sleep(0.3)

        listing = groups_by_browser()
        stamped_ok = bool(
            listing["chrome"].get(pairs["chrome"]["gid"])
            and listing["edge"].get(pairs["edge"]["gid"])
        )

        collision = pairs["chrome"]["gid"] == pairs["edge"]["gid"]
        cycles = 0
        while not collision and cycles < 6:
            chrome_id = numeric(pairs["chrome"]["gid"])
            edge_id = numeric(pairs["edge"]["gid"])
            if chrome_id is None or edge_id is None:
                break
            low = "chrome" if chrome_id < edge_id else "edge"
            high = "edge" if low == "chrome" else "chrome"
            target = pairs[high]["gid"]
            tabs = pairs[low]["tabs"]
            vcu(["browser", "ungroup", "--tabs", ",".join(tabs), "--browser", low])
            time.sleep(0.25)
            regrouped, gid = group_pair(low, tabs, pairs[low]["title"], pairs[low]["color"])
            cycles += 1
            if regrouped.get("ok") is not True or not gid:
                break
            pairs[low]["gid"] = gid
            if gid == target:
                collision = True

        before = groups_by_browser()
        chrome_before = before["chrome"].get(pairs["chrome"]["gid"]) or {}
        edge_before = before["edge"].get(pairs["edge"]["gid"]) or {}

        ambiguous_live = False
        unique_resolve_ok = None
        ambig = vcu(
            [
                "browser",
                "group-update",
                "--group",
                pairs["chrome"]["gid"],
                "--title",
                "vcu-d-680-no-browser",
            ]
        )
        if collision:
            ambiguous_live = ambig.get("ok") is False and "ambiguous" in err_message(ambig)
        else:
            unique_resolve_ok = ambig.get("ok") is True
            vcu(
                [
                    "browser",
                    "group-update",
                    "--group",
                    pairs["chrome"]["gid"],
                    "--browser",
                    "chrome",
                    "--title",
                    str(chrome_before.get("title") or CHROME_TITLE),
                ]
            )
            time.sleep(0.3)

        upd_chrome = vcu(
            [
                "browser",
                "group-update",
                "--group",
                pairs["chrome"]["gid"],
                "--browser",
                "chrome",
                "--title",
                CHROME_TITLE + "-2",
                "--color",
                "purple",
                "--collapsed",
                "true",
            ]
        )
        time.sleep(0.3)
        after_chrome = groups_by_browser()
        chrome_new = after_chrome["chrome"].get(pairs["chrome"]["gid"]) or {}
        edge_after_chrome = after_chrome["edge"].get(pairs["edge"]["gid"]) or {}
        chrome_ok = (
            upd_chrome.get("ok") is True
            and str(chrome_new.get("title") or "") == CHROME_TITLE + "-2"
            and str(chrome_new.get("color") or "") == "purple"
            and chrome_new.get("collapsed") is True
        )
        edge_untouched = (
            str(edge_after_chrome.get("title") or "") == str(edge_before.get("title") or "")
            and str(edge_after_chrome.get("color") or "") == str(edge_before.get("color") or "")
        )

        upd_edge = vcu(
            [
                "browser",
                "group-update",
                "--group",
                pairs["edge"]["gid"],
                "--browser",
                "edge",
                "--title",
                EDGE_TITLE + "-2",
                "--color",
                "cyan",
                "--collapsed",
                "false",
            ]
        )
        time.sleep(0.3)
        after_edge = groups_by_browser()
        edge_new = after_edge["edge"].get(pairs["edge"]["gid"]) or {}
        chrome_after_edge = after_edge["chrome"].get(pairs["chrome"]["gid"]) or {}
        edge_ok = (
            upd_edge.get("ok") is True
            and str(edge_new.get("title") or "") == EDGE_TITLE + "-2"
            and str(edge_new.get("color") or "") == "cyan"
            and edge_new.get("collapsed") is False
        )
        chrome_kept = (
            str(chrome_after_edge.get("title") or "") == CHROME_TITLE + "-2"
            and str(chrome_after_edge.get("color") or "") == "purple"
        )

        missing = vcu(
            [
                "browser",
                "group-update",
                "--group",
                "999999999",
                "--browser",
                "chrome",
                "--title",
                "vcu-d-680-missing",
            ]
        )
        missing_text = err_message(missing).lower()
        missing_ok = missing.get("ok") is False and (
            "no group with id" in missing_text or "not found" in missing_text
        )

        wrong_ok = None
        if not collision:
            wrong = vcu(
                [
                    "browser",
                    "group-update",
                    "--group",
                    pairs["edge"]["gid"],
                    "--browser",
                    "chrome",
                    "--title",
                    "vcu-d-680-wrong-browser",
                ]
            )
            wrong_text = err_message(wrong).lower()
            wrong_ok = wrong.get("ok") is False and (
                "no group with id" in wrong_text or "not found" in wrong_text
            )
            time.sleep(0.3)
        after_wrong = groups_by_browser()
        chrome_intact = (
            str((after_wrong["chrome"].get(pairs["chrome"]["gid"]) or {}).get("title") or "")
            == CHROME_TITLE + "-2"
        )

        for browser in ("chrome", "edge"):
            vcu(
                [
                    "browser",
                    "ungroup",
                    "--tabs",
                    ",".join(pairs[browser]["tabs"]),
                    "--browser",
                    browser,
                ]
            )
            time.sleep(0.25)
        time.sleep(0.4)
        final_groups = groups_by_browser()
        groups_cleaned = not final_groups["chrome"] and not final_groups["edge"]
        after_protected = protected(vcu(["browser", "tabs"]))

        ok = all(
            [
                stamped_ok,
                chrome_ok,
                edge_ok,
                edge_untouched,
                chrome_kept,
                missing_ok,
                chrome_intact,
                groups_cleaned,
                before_protected == after_protected,
                ambiguous_live if collision else True,
                True if unique_resolve_ok is None else unique_resolve_ok,
                True if wrong_ok is None else wrong_ok,
            ]
        )
        report = {
            "ok": ok,
            "collision": collision,
            "collision_cycles": cycles,
            "ambiguous_live": ambiguous_live,
            "ambiguous_covered_by": None if collision else "cargo test -p vcu-server --test app_http (upd_amb asserts InvalidInput ambiguous)",
            "chrome_group": pairs["chrome"]["gid"],
            "edge_group": pairs["edge"]["gid"],
            "stamped_ok": stamped_ok,
            "ambiguous_error": err_message(ambig).strip(),
            "unique_resolve_ok": unique_resolve_ok,
            "chrome_update_ok": chrome_ok,
            "edge_untouched": edge_untouched,
            "edge_update_ok": edge_ok,
            "chrome_kept": chrome_kept,
            "missing_ok": missing_ok,
            "wrong_browser_ok": wrong_ok,
            "chrome_intact_after_wrong": chrome_intact,
            "groups_cleaned": groups_cleaned,
            "protected_ok": before_protected == after_protected,
        }
        if ok:
            print(
                "CU-D-680 OK collision",
                collision,
                "chrome",
                pairs["chrome"]["gid"],
                "edge",
                pairs["edge"]["gid"],
            )
            return 0
        print("CU-D-680 FAIL", json.dumps(report, ensure_ascii=False))
        return 1
    finally:
        for browser, tab in opened:
            vcu(["browser", "close", "--tab", tab, "--browser", browser])
        if httpd is not None:
            httpd.shutdown()
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps(report, indent=2))


if __name__ == "__main__":
    sys.exit(main())
