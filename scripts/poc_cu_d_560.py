#!/usr/bin/env python3
"""CU-D-560: health/init next describe observe-bind, not tabs-then-screenshot.

No browser mutations. Groups 1/3 untouched.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
_debug = ROOT / "target/debug/vcu"
VCU = os.environ.get("VCU") or (str(_debug) if _debug.exists() else "vcu")
OUT = Path(".local/desktop-cu/cu-d-560.json")


def vcu(args):
    cmd = [VCU, *args]
    if "--json" not in cmd:
        cmd.append("--json")
    p = subprocess.run(cmd, capture_output=True, text=True)
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:1200], "code": p.returncode}


def data(resp):
    return resp.get("data") if isinstance(resp.get("data"), dict) else {}


def main():
    health = vcu(["daemon", "status"]) if False else None
    # health via HTTP-equivalent CLI if any; use doctor/login-state + init
    init = vcu(["init"])
    nxt = data(init).get("next") or init.get("next") or []
    if isinstance(nxt, str):
        nxt = [nxt]
    blob = " ".join(str(x) for x in nxt)
    login = vcu(["browser", "login-state"])
    na = str(data(login).get("next_action") or "")
    # daemon health
    import urllib.request
    token = json.loads((Path.home()/".vcu/config.json").read_text())["pairing_token"]
    req = urllib.request.Request("http://127.0.0.1:17890/v1/health", headers={"X-Vcu-Token": token})
    with urllib.request.urlopen(req, timeout=5) as r:
        h = json.loads(r.read())
    hd = h.get("data") or h
    click = str(hd.get("click") or "")
    observe = str(hd.get("observe") or "")
    init_ok = (
        "observe" in blob
        and "select an exact tab" not in blob
        and "tabs --json" not in blob
    )
    health_ok = "selector" in click and "pixel-x" not in click and "observe" in observe
    login_ok = "observe" in na and "last observe" in na
    ok = init.get("ok") is not False and init_ok and health_ok and login_ok
    report = {
        "ok": ok,
        "init_next": nxt,
        "health_click": click,
        "health_observe": observe,
        "login_next_action": na[:180],
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("init_next", nxt)
    print("health_click", click)
    print("health_observe", observe)
    if ok:
        print("CU-D-560 OK")
        return 0
    print("CU-D-560 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
