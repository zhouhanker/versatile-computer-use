#!/usr/bin/env python3
"""CU-D-032: browser regression — ping, tabs, open-as-tab, screenshot click.

Does not touch USER Edge groups titled 1 / 3. Opens one throwaway tab and closes it.
"""
from __future__ import annotations

import json
import subprocess
import threading
import time
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
FIXTURE_ROOT = REPO / "extension"
OUT = Path(".local/desktop-cu/cu-d-032.json")
PROTECTED = {"1", "3"}


def vcu(args):
    p = subprocess.run(["vcu", *args, "--json"], capture_output=True, text=True, encoding="utf-8", errors="replace")
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:600], "code": p.returncode}


def data(resp):
    return resp.get("data") if isinstance(resp.get("data"), dict) else {}


def group_snapshot(tabs_resp):
    groups = data(tabs_resp).get("groups") or []
    out = {}
    for g in groups:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            ids = g.get("tab_ids") or g.get("tabs") or []
            out[title] = sorted(str(x) for x in ids)
    return out


def main() -> int:
    report = {"ok": False, "steps": []}
    our_tab = None
    httpd = None

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)

    ping = vcu(["browser", "ping"])
    ext = data(ping).get("extension") or {}
    step(
        "ping",
        ping.get("ok") is True and data(ping).get("pong") is True,
        {
            "version": ext.get("version"),
            "hud": data(ping).get("hud"),
            "os_cursor_used": data(ping).get("os_cursor_used"),
            "error": ping.get("error"),
        },
    )

    before = vcu(["browser", "tabs"])
    bdata = data(before)
    tabs = bdata.get("tabs") or []
    protected_before = group_snapshot(before)
    window_ids = sorted({str(t.get("window_id")) for t in tabs if t.get("window_id") is not None})
    step(
        "tabs",
        before.get("ok") is True and bdata.get("source") == "extension_tabs" and len(tabs) >= 1,
        {
            "n_tabs": len(tabs),
            "n_groups": len(bdata.get("groups") or []),
            "source": bdata.get("source"),
            "protected": protected_before,
            "windows": window_ids,
            "error": before.get("error"),
        },
    )

    handler = lambda *a, **k: SimpleHTTPRequestHandler(*a, directory=str(FIXTURE_ROOT), **k)
    httpd = ThreadingHTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    url = f"http://127.0.0.1:{httpd.server_address[1]}/tests/fixtures/interaction.html?cu-d-032=1"

    opened = vcu(["browser", "open", "--background", "--url", url])
    o = data(opened)
    our_tab = str(o.get("tab_id") or "")
    our_win = str(o.get("window_id") or "")
    new_window = o.get("new_window")
    after_open = vcu(["browser", "tabs"])
    our_meta = next((t for t in (data(after_open).get("tabs") or []) if str(t.get("tab_id")) == our_tab), {})
    if not our_win:
        our_win = str(our_meta.get("window_id") or "")
    opened_ok = (
        opened.get("ok") is True
        and our_tab.isdigit()
        and new_window is not True
        and (not window_ids or our_win in window_ids)
    )
    step(
        "open_as_tab",
        opened_ok,
        {
            "tab_id": our_tab,
            "window_id": our_win,
            "existing_windows": window_ids,
            "new_window": new_window,
            "focused": o.get("focused"),
            "source": o.get("source"),
            "error": opened.get("error"),
        },
    )

    try:
        time.sleep(0.6)
        selected = vcu(["browser", "select", "--tab", our_tab])
        step(
            "select_throwaway",
            selected.get("ok") is True,
            {"error": selected.get("error"), "source": data(selected).get("source")},
        )
        time.sleep(0.3)
        shot = vcu(["browser", "screenshot", "--tab", our_tab])
        s = data(shot)
        capture = s.get("capture_id")
        step(
            "screenshot",
            shot.get("ok") is True and bool(capture) and s.get("os_cursor_used") is not True,
            {
                "capture_id": capture,
                "width": s.get("width"),
                "height": s.get("height"),
                "source": s.get("source"),
                "os_cursor_used": s.get("os_cursor_used"),
                "error": shot.get("error"),
            },
        )
        clicked = {"ok": False}
        if capture:
            clicked = vcu(
                [
                    "browser",
                    "click",
                    "--space",
                    "viewport",
                    "--capture",
                    str(capture),
                    "--pixel-x",
                    "24",
                    "--pixel-y",
                    "24",
                    "--dry-run",
                    "--tab",
                    our_tab,
                ]
            )
        c = data(clicked)
        extc = c.get("extension") if isinstance(c.get("extension"), dict) else {}
        step(
            "screenshot_click_dry_run",
            clicked.get("ok") is True and c.get("os_cursor_used") is not True and c.get("pressed") is not True,
            {
                "pressed": c.get("pressed"),
                "os_cursor_used": c.get("os_cursor_used"),
                "source": c.get("source"),
                "trusted": extc.get("trusted"),
                "error": clicked.get("error"),
            },
        )

        live = vcu(["browser", "click", "--tab", our_tab, "--selector", "#counter"])
        l = data(live)
        extracted = vcu(["browser", "extract", "--tab", our_tab, "--selector", "#result"])
        matches = data(extracted).get("matches") or []
        text = matches[0].get("text") if matches else ""
        step(
            "dom_click_throwaway",
            live.get("ok") is True
            and l.get("source") == "extension_dom"
            and l.get("os_cursor_used") is not True
            and "clicks=1" in str(text),
            {
                "source": l.get("source"),
                "os_cursor_used": l.get("os_cursor_used"),
                "result": text,
                "error": live.get("error"),
            },
        )
    finally:
        if our_tab:
            closed = vcu(["browser", "close", "--tab", our_tab])
            step("close_throwaway", closed.get("ok") is True, {"error": closed.get("error")})
        if httpd:
            httpd.shutdown()

    after = vcu(["browser", "tabs"])
    protected_after = group_snapshot(after)
    after_ids = {str(t.get("tab_id")) for t in (data(after).get("tabs") or [])}
    step(
        "protected_groups_unchanged",
        protected_before == protected_after and our_tab not in after_ids,
        {"before": protected_before, "after": protected_after},
    )

    needed = {s["name"]: s["ok"] for s in report["steps"]}
    report["ok"] = all(
        needed.get(n)
        for n in (
            "ping",
            "tabs",
            "open_as_tab",
            "select_throwaway",
            "screenshot",
            "screenshot_click_dry_run",
            "dom_click_throwaway",
            "close_throwaway",
            "protected_groups_unchanged",
        )
    )
    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
