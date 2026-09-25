#!/usr/bin/env python3
"""CU-D-520: same tabs call includes Chrome and Edge.

If Edge worker is stale, open the unpacked reload.html in USER Edge once,
then close it. Does not touch groups titled 1 / 3. Never Allow / OS cursor.
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from collections import Counter
from pathlib import Path

PROTECTED = {"1", "3"}
OUT = Path(".local/desktop-cu/cu-d-520.json")
VCU = os.environ.get("VCU", "vcu")
EXT_ID = "cedlbclnijpladccmmfpihhgkeeldfhc"
RELOAD_URL = "chrome-extension://%s/reload.html" % EXT_ID


def vcu(args):
    cmd = [VCU, *args]
    if "--json" not in cmd:
        cmd.append("--json")
    if "--user-dir" not in cmd and os.environ.get("VCU_DIR"):
        cmd[1:1] = ["--user-dir", os.environ["VCU_DIR"]]
    p = subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")
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


def close_reload_tab():
    script = """tell application "Microsoft Edge"
  set theUrl to "%s"
  repeat with w in windows
    set tabList to tabs of w
    repeat with t in tabList
      try
        if (URL of t) starts with theUrl then
          close t
        end if
      end try
    end repeat
  end repeat
end tell
""" % RELOAD_URL
    subprocess.run(["osascript", "-e", script], capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=6)


def main():
    steps = []
    before = vcu(["browser", "tabs"])
    prot_before = protected_snapshot(before)
    counts = browsers_of(before)
    bd = data(before)
    steps.append({
        "tabs_before": True,
        "counts": dict(counts),
        "browser_count": bd.get("browser_count"),
        "browsers_ok": bd.get("browsers_ok"),
        "browsers_failed": bd.get("browsers_failed"),
        "dt_note": "first merge",
    })
    woke = False
    if counts.get("edge", 0) < 1 or counts.get("chrome", 0) < 1:
        subprocess.run(["open", "-a", "Microsoft Edge", RELOAD_URL], capture_output=True)
        woke = True
        steps.append({"page_reload": True, "url": RELOAD_URL})
        time.sleep(2)
        before = vcu(["browser", "tabs"])
        counts = browsers_of(before)
        bd = data(before)
        steps.append({
            "tabs_after_reload": True,
            "counts": dict(counts),
            "browser_count": bd.get("browser_count"),
            "browsers_ok": bd.get("browsers_ok"),
            "browsers_failed": bd.get("browsers_failed"),
        })
        close_reload_tab()
        steps.append({"reload_tab_closed": True})
    chrome_n = int(counts.get("chrome") or 0)
    edge_n = int(counts.get("edge") or 0)
    merge_ok = (
        before.get("ok") is True
        and int(bd.get("browser_count") or 0) >= 2
        and chrome_n >= 1
        and edge_n >= 1
        and not (bd.get("browsers_failed") or [])
    )
    after = vcu(["browser", "tabs"])
    prot_after = protected_snapshot(after)
    groups_ok = prot_before == prot_after
    ok = merge_ok and groups_ok
    report = {
        "ok": ok,
        "CHROME_TABS_OK": chrome_n >= 1,
        "EDGE_TABS_OK": edge_n >= 1,
        "MERGE_OK": merge_ok,
        "GROUPS_OK": groups_ok,
        "woke": woke,
        "chrome_tabs": chrome_n,
        "edge_tabs": edge_n,
        "protected": prot_after,
        "steps": steps,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(report, indent=2))
    print("CHROME_TABS_OK count=%s" % chrome_n if chrome_n >= 1 else "CHROME_TABS_MISSING")
    print("EDGE_TABS_OK count=%s" % edge_n if edge_n >= 1 else "EDGE_TABS_MISSING")
    if merge_ok:
        print("MERGE_OK browser_count=%s chrome=%s edge=%s" % (bd.get("browser_count"), chrome_n, edge_n))
    else:
        print("MERGE_FAIL", dict(counts), bd.get("browsers_failed"))
    print("GROUPS_OK" if groups_ok else "GROUPS_CHANGED %s %s" % (prot_before, prot_after))
    if ok:
        print("CU-D-520 OK")
        return 0
    print("CU-D-520 FAIL")
    return 1


if __name__ == "__main__":
    sys.exit(main())
