#!/usr/bin/env python3
"""CU-D-550: extract without tab_id stamps last_observe.

Does not click. Does not touch groups 1/3. Never Allow / OS cursor.
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
OUT = Path(".local/desktop-cu/cu-d-550.json")


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


def protected(tabs):
    out = {}
    for g in data(tabs).get("groups") or []:
        title = str(g.get("title") or "")
        if title in PROTECTED:
            out[title] = sorted(str(x) for x in (g.get("tab_ids") or g.get("tabs") or []))
    return out


def main():
    prot0 = protected(vcu(["browser", "tabs"]))
    obs = vcu(["browser", "observe"])
    od = data(obs)
    want = str(od.get("tab_id") or "")
    ext = vcu(["browser", "extract", "--selector", "title"])
    ed = data(ext)
    got = str(ed.get("tab_id") or "")
    src = str(ed.get("tab_id_source") or "")
    extract_ok = (
        ext.get("ok") is True
        and ed.get("source") == "extension_dom"
        and want
        and got == want
        and src == "last_observe"
    )
    prot1 = protected(vcu(["browser", "tabs"]))
    groups_ok = prot0 == prot1
    ok = obs.get("ok") is True and extract_ok and groups_ok
    report = {
        "ok": ok,
        "observe_tab": want,
        "extract_tab": got,
        "tab_id_source": src,
        "source": ed.get("source"),
        "groups_ok": groups_ok,
        "error": ext.get("error"),
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("observe_tab", want)
    print("extract_tab", got, "source", src, ed.get("source"))
    print("GROUPS_OK" if groups_ok else "GROUPS_CHANGED")
    if ok:
        print("CU-D-550 OK")
        return 0
    print("CU-D-550 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
