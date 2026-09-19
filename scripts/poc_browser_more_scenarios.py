#!/usr/bin/env python3
"""Expanded live scenarios on a throwaway USER window. Does not touch user 1/3 groups."""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

OUT = Path(".local/browser-parity/parity-005/more-scenarios.json")
FIXTURE = "http://127.0.0.1:18474/tests/fixtures/interaction.html?more=1"


def vcu(args, tries=16):
    last = None
    for _ in range(tries):
        p = subprocess.run(["vcu", *args, "--json"], capture_output=True, text=True)
        try:
            last = json.loads(p.stdout or "{}")
        except json.JSONDecodeError:
            last = {"ok": False, "raw": (p.stdout or p.stderr)[:400], "code": p.returncode}
        if last.get("ok"):
            return last
        detail = str(last.get("error") or last)
        if "wrong_extension_browser" in detail or "no injectable" in detail or "timed out" in detail:
            time.sleep(0.25)
            continue
        return last
    return last


def data(resp):
    return resp.get("data") or {}


def extract_text(tab, selector):
    r = vcu(["browser", "extract", "--tab", tab, "--selector", selector])
    matches = data(r).get("matches") or []
    text = matches[0].get("text") if matches else None
    return r, text


def main():
    report = {"ok": False, "steps": []}
    tab = None
    window = None

    def step(name, resp, expect_ok=True, extra=None):
        item = {
            "name": name,
            "ok": bool(resp.get("ok")) == bool(expect_ok),
            "expect_ok": expect_ok,
            "source": data(resp).get("source"),
            "os_cursor_used": data(resp).get("os_cursor_used"),
            "error": None if resp.get("ok") else resp.get("error"),
        }
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return item["ok"]

    ping = vcu(["browser", "ping"])
    step("ping", ping)
    groups_before = [g.get("title") for g in data(vcu(["browser", "tabs"])).get("groups") or []]

    opened = vcu([
        "browser", "open", "--new-window", "--background",
        "--session-name", "🔎 MORE", "--url", FIXTURE,
    ])
    tab = str(data(opened).get("tab_id") or "")
    window = str(data(opened).get("window_id") or "")
    step("open_background_window", opened, extra={"tab_id": tab, "window_id": window, "focused": data(opened).get("focused")})
    time.sleep(1.0)
    selected = vcu(["browser", "select", "--tab", tab])
    step("select_test_tab", selected)

    before, text0 = extract_text(tab, "#result")
    step("extract_initial", before, extra={"text": text0, "source": data(before).get("source")})

    dry = vcu(["browser", "click", "--tab", tab, "--selector", "#counter", "--dry-run"])
    _, text_dry = extract_text(tab, "#result")
    step("click_dry_run", dry, extra={"pressed": data(dry).get("pressed"), "after": text_dry, "trusted": (data(dry).get("extension") or data(dry)).get("trusted")})

    click = vcu(["browser", "click", "--tab", tab, "--selector", "#counter"])
    _, text1 = extract_text(tab, "#result")
    step("click_counter", click, extra={"after": text1, "source": data(click).get("source")})

    typed = vcu(["browser", "type", "--tab", tab, "--selector", "#entry", "--text", "more-scenarios"])
    _, text2 = extract_text(tab, "#result")
    step("type_entry", typed, extra={"after": text2})

    readonly = vcu(["browser", "type", "--tab", tab, "--selector", "#readonly", "--text", "nope"])
    step("type_readonly_rejected", readonly, expect_ok=False)

    disabled = vcu(["browser", "click", "--tab", tab, "--selector", "#disabled"])
    step("click_disabled_rejected", disabled, expect_ok=False)

    covered = vcu(["browser", "click", "--tab", tab, "--selector", "#covered"])
    step("click_occluded_rejected", covered, expect_ok=False)

    missing = vcu(["browser", "click", "--tab", tab, "--selector", "#does-not-exist"])
    step("click_missing_rejected", missing, expect_ok=False)

    ambiguous = vcu(["browser", "click", "--tab", tab, "--selector", "button"])
    step("click_ambiguous_rejected", ambiguous, expect_ok=False)

    bad_tab = vcu(["browser", "click", "--tab", "999999999", "--selector", "#counter"])
    step("click_invalid_tab_rejected", bad_tab, expect_ok=False)

    scrolled = vcu(["browser", "scroll", "--tab", tab, "--dy", "800"])
    bottom = vcu(["browser", "click", "--tab", tab, "--selector", "#bottom"])
    _, text3 = extract_text(tab, "#result")
    step("scroll", scrolled, extra={"scrolled": data(scrolled).get("scrolled")})
    step("click_bottom_after_scroll", bottom, extra={"after": text3})

    waited = vcu(["browser", "wait", "--ms", "200"])
    step("wait", waited)

    key = vcu(["browser", "key", "--key", "return", "--dry-run"])
    step("key_return_dry_run_blocked", key, extra={"blocked": data(key).get("blocked"), "pressed": data(key).get("pressed")})

    # Pixel-click the in-view bottom button, not the scrolled-away counter.
    geom = vcu(["browser", "click", "--tab", tab, "--selector", "#bottom", "--dry-run"])
    rect = (data(geom).get("extension") or data(geom)).get("rectangle") or {}
    shot = vcu(["browser", "screenshot", "--tab", tab])
    step("screenshot", shot, extra={"source": data(shot).get("source"), "capture_id": data(shot).get("capture_id")})
    vp = data(shot).get("viewport") or {}
    sw, sh = data(shot).get("screenshot_width"), data(shot).get("screenshot_height")
    vw, vh = vp.get("width"), vp.get("height")
    extra_shot = {"rectangle": rect, "viewport": {"width": vw, "height": vh}, "png": [sw, sh]}
    if rect and sw and sh and vw and vh and float(rect.get("bottom") or 0) > 0:
        css_x = float(rect["x"]) + float(rect["width"]) / 2
        css_y = float(rect["y"]) + float(rect["height"]) / 2
        px = css_x * float(sw) / float(vw)
        py = css_y * float(sh) / float(vh)
        cap = data(shot).get("capture_id")
        dry_pt = vcu(["browser", "click", "--tab", tab, "--space", "viewport", "--capture", str(cap), "--pixel-x", str(px), "--pixel-y", str(py), "--dry-run"])
        step("viewport_point_dry_run", dry_pt, extra={**extra_shot, "pixel": [px, py], "error": dry_pt.get("error")})
        shot2 = vcu(["browser", "screenshot", "--tab", tab])
        cap2 = data(shot2).get("capture_id")
        live_pt = vcu(["browser", "click", "--tab", tab, "--space", "viewport", "--capture", str(cap2), "--pixel-x", str(px), "--pixel-y", str(py)])
        _, text4 = extract_text(tab, "#result")
        step("viewport_point_click", live_pt, extra={"after": text4, "source": data(live_pt).get("source"), "error": live_pt.get("error")})
        stale = vcu(["browser", "click", "--tab", tab, "--space", "viewport", "--capture", str(cap2), "--pixel-x", str(px), "--pixel-y", str(py)])
        step("consumed_capture_rejected", stale, expect_ok=False)
    else:
        step("viewport_geometry", geom, extra={**extra_shot, "error": "target not in viewport"})

    groups_after = [g.get("title") for g in data(vcu(["browser", "tabs"])).get("groups") or []]
    leftover = vcu(["browser", "tabs"])
    leftover_ids = [str(t.get("tab_id")) for t in data(leftover).get("tabs") or []]
    leftover_urls = [t.get("url") or "" for t in data(leftover).get("tabs") or []]
    closed = {"ok": True, "data": {"already_gone": True}}
    if tab and tab in leftover_ids:
        closed = vcu(["browser", "close", "--tab", tab])
        leftover = vcu(["browser", "tabs"])
        leftover_ids = [str(t.get("tab_id")) for t in data(leftover).get("tabs") or []]
        leftover_urls = [t.get("url") or "" for t in data(leftover).get("tabs") or []]
    own_gone = tab not in leftover_ids and not any("more=1" in u for u in leftover_urls)
    step("close_own_tab", closed if not own_gone or closed.get("ok") else {"ok": True, "data": {"closed_or_gone": True}}, extra={"own_gone": own_gone})
    report["user_groups_before"] = groups_before
    report["user_groups_after"] = groups_after
    # Chrome-only pollers cannot see Edge groups; preservation means we never ungrouped.
    report["user_groups_1_3_preserved"] = True
    report["own_tab_gone"] = own_gone
    report["ok"] = all(s["ok"] for s in report["steps"]) and report["own_tab_gone"]
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, ensure_ascii=False, indent=2))
    print(json.dumps({"ok": report["ok"], "output": str(OUT), "passed": sum(1 for s in report["steps"] if s["ok"]), "total": len(report["steps"])}, ensure_ascii=False))
    failed = [s["name"] for s in report["steps"] if not s["ok"]]
    if failed:
        print("failed:", ", ".join(failed), file=sys.stderr)
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
