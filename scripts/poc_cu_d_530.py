#!/usr/bin/env python3
"""CU-D-530: login-state observe binds the frontmost USER Chrome/Edge.

Does not open/close/group tabs. Does not touch groups 1/3.
Never Allow / OS cursor warp.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

PROTECTED = {"1", "3"}
OUT = Path(".local/desktop-cu/cu-d-530.json")
ROOT = Path(__file__).resolve().parents[1]
_debug = ROOT / "target/debug/vcu"
VCU = os.environ.get("VCU") or (str(_debug) if _debug.exists() else "vcu")


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


def frontmost():
    p = subprocess.run(
        ["osascript", "-e", 'tell application "System Events" to get name of first application process whose frontmost is true'],
        capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=4,
    )
    return (p.stdout or "").strip()


def main():
    fm = frontmost()
    tabs = vcu(["browser", "tabs"])
    prot = {}
    for g in data(tabs).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            prot[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    obs = vcu(["browser", "observe"])
    od = data(obs)
    app_id = str(od.get("app_id") or "")
    tab_id = str(od.get("tab_id") or "")
    matched = od.get("frontmost_matched")
    low = fm.lower()
    want_edge = "edge" in low
    want_chrome = "chrome" in low and "edge" not in low
    if want_edge:
        app_ok = "edge" in app_id.lower()
    elif want_chrome:
        app_ok = "chrome" in app_id.lower() and "edge" not in app_id.lower()
    else:
        print("SKIP: frontmost is not USER Chrome/Edge:", fm)
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps({"ok": True, "skip": True, "frontmost": fm}, indent=2))
        return 0
    after = vcu(["browser", "tabs"])
    prot2 = {}
    for g in data(after).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            prot2[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    groups_ok = prot == prot2
    ok = obs.get("ok") is True and app_ok and bool(tab_id) and groups_ok
    report = {
        "ok": ok,
        "frontmost": fm,
        "app_id": app_id,
        "tab_id": tab_id,
        "frontmost_app": od.get("frontmost_app"),
        "frontmost_matched": matched,
        "page_url": od.get("page_url"),
        "groups_ok": groups_ok,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("frontmost", fm)
    print("app_id", app_id, "tab_id", tab_id)
    print("FRONTMOST_MATCHED", bool(app_ok), "matched_field", matched)
    print("GROUPS_OK" if groups_ok else "GROUPS_CHANGED")
    if ok:
        print("CU-D-530 OK")
        return 0
    print("CU-D-530 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
