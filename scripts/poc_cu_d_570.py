#!/usr/bin/env python3
"""CU-D-570: login-state observe via lens is fast (no full AX walk).

No click. Groups 1/3 untouched.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

PROTECTED = {"1", "3"}
ROOT = Path(__file__).resolve().parents[1]
_debug = ROOT / "target/debug/vcu"
VCU = os.environ.get("VCU") or (str(_debug) if _debug.exists() else "vcu")
OUT = Path(".local/desktop-cu/cu-d-570.json")


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


def protected(tabs):
    out = {}
    for g in data(tabs).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            out[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    return out


def main():
    prot0 = protected(vcu(["browser", "tabs"]))
    t0 = time.time()
    obs = vcu(["browser", "observe"])
    dt = time.time() - t0
    od = data(obs)
    snap = od.get("snapshot") or {}
    elements = snap.get("elements") if isinstance(snap, dict) else None
    n_el = len(elements) if isinstance(elements, list) else 0
    fast = dt < 2.5
    has_tab = bool(od.get("tab_id"))
    src = str(od.get("tabs_source") or "")
    png = bool(od.get("screenshot_path") or (od.get("vision_handoff") or {}).get("must_view") or snap.get("screenshot_path"))
    no_ax_dump = n_el == 0
    prot1 = protected(vcu(["browser", "tabs"]))
    ok = obs.get("ok") is True and fast and has_tab and src == "extension_tabs" and png and no_ax_dump and prot0 == prot1
    report = {
        "ok": ok,
        "dt": round(dt, 3),
        "tab_id": od.get("tab_id"),
        "tabs_source": src,
        "elements": n_el,
        "app_id": od.get("app_id"),
        "png": png,
        "groups_ok": prot0 == prot1,
        "error": obs.get("error"),
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("dt", round(dt, 3), "tab", od.get("tab_id"), "tabs_source", src, "elements", n_el)
    if ok:
        print("CU-D-570 OK")
        return 0
    print("CU-D-570 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
