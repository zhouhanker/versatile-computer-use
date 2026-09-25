#!/usr/bin/env python3
"""CU-D-580: observe capture_id can dry-run viewport click.

Dry-run only. Groups 1/3 untouched. Never Allow / OS cursor.
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
OUT = Path(".local/desktop-cu/cu-d-580.json")


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
    obs = vcu(["browser", "observe"])
    od = data(obs)
    cid = str(od.get("capture_id") or (od.get("snapshot") or {}).get("capture_id") or "")
    click = vcu([
        "browser", "click",
        "--space", "viewport",
        "--capture", cid,
        "--pixel-x", "1",
        "--pixel-y", "1",
        "--dry-run",
    ]) if cid else {"ok": False, "error": {"message": "no capture_id"}}
    cd = data(click)
    click_ok = (
        click.get("ok") is True
        and cd.get("dry_run") is True
        and cd.get("source") == "extension_dom"
        and cd.get("os_cursor_used") is False
    )
    prot1 = protected(vcu(["browser", "tabs"]))
    ok = obs.get("ok") is True and bool(cid) and cid in str(od.get("capture_id") or "") and click_ok and prot0 == prot1
    report = {
        "ok": ok,
        "capture_id": cid,
        "observe_source": od.get("source"),
        "click_ok": click.get("ok"),
        "click_source": cd.get("source"),
        "tab_id": od.get("tab_id"),
        "groups_ok": prot0 == prot1,
        "error": click.get("error"),
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("capture_id", cid)
    print("observe_source", od.get("source"))
    print("click", click.get("ok"), cd.get("source"), cd.get("dry_run"), click.get("error"))
    if ok:
        print("CU-D-580 OK")
        return 0
    print("CU-D-580 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
