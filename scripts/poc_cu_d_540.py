#!/usr/bin/env python3
"""CU-D-540: login-state user_browsers lists the frontmost USER browser first.

Does not open/close/group. Does not touch groups 1/3. Never Allow / OS cursor.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

PROTECTED = {"1", "3"}
ROOT = Path(__file__).resolve().parents[1]
_debug = ROOT / "target/debug/vcu"
VCU = os.environ.get("VCU") or (str(_debug) if _debug.exists() else "vcu")
OUT = Path(".local/desktop-cu/cu-d-540.json")


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


def protected(tabs):
    out = {}
    for g in data(tabs).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            out[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    return out


def main():
    fm = frontmost()
    low = fm.lower()
    before = vcu(["browser", "tabs"])
    prot0 = protected(before)
    login = vcu(["browser", "login-state"])
    users = data(login).get("user_browsers") or []
    first = users[0] if users else {}
    name = str(first.get("name") or "")
    obs = vcu(["browser", "observe"])
    od = data(obs)
    app_id = str(od.get("app_id") or "")
    if "edge" in low:
        login_ok = "edge" in name.lower()
        obs_ok = "edge" in app_id.lower()
    elif "chrome" in low:
        login_ok = "chrome" in name.lower() and "edge" not in name.lower()
        obs_ok = "chrome" in app_id.lower() and "edge" not in app_id.lower()
    else:
        print("SKIP: frontmost is not USER Chrome/Edge:", fm)
        OUT.parent.mkdir(parents=True, exist_ok=True)
        OUT.write_text(json.dumps({"ok": True, "skip": True, "frontmost": fm}, indent=2))
        return 0
    prot1 = protected(vcu(["browser", "tabs"]))
    groups_ok = prot0 == prot1
    ok = login.get("ok") is True and login_ok and obs.get("ok") is True and obs_ok and groups_ok
    report = {
        "ok": ok,
        "frontmost": fm,
        "login_first": {"name": name, "pid": first.get("pid")},
        "observe_app_id": app_id,
        "observe_tab_id": od.get("tab_id"),
        "groups_ok": groups_ok,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("frontmost", fm)
    print("login_first", name, first.get("pid"))
    print("observe", app_id, od.get("tab_id"))
    print("GROUPS_OK" if groups_ok else "GROUPS_CHANGED")
    if ok:
        print("CU-D-540 OK")
        return 0
    print("CU-D-540 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
