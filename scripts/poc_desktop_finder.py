#!/usr/bin/env python3
"""CU-D-040: observe a script-created Finder folder window via CG Scene.

Does not use `tell application "Finder"` (Automation TCC hangs).
Does not warp the OS cursor. Does not batch-delete user files.
Icon AXPress is not claimed: current macOS Finder AX does not expose folder icons.
"""
from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

MARKER = f"VCU-D-040-{int(time.time())}"
ROOT = Path(f"/tmp/{MARKER}")
OUT = Path(".local/desktop-cu/finder-040.json")


def run(cmd, timeout=20):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:600], "code": p.returncode}


def main() -> int:
    report = {"ok": False, "marker": MARKER, "steps": []}

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    ROOT.mkdir(parents=True, exist_ok=True)
    (ROOT / "OPENME").mkdir(exist_ok=True)
    (ROOT / "OPENME" / "inside.txt").write_text(MARKER + "\n")

    opened = run(["open", "-a", "Finder", str(ROOT)])
    step("open_finder", opened.returncode == 0, {"stderr": (opened.stderr or "")[:200]})
    time.sleep(0.6)

    pid_p = run(
        ["osascript", "-e", 'tell application "System Events" to unix id of process "Finder"']
    )
    pid = (pid_p.stdout or "").strip()
    if not step("pid", pid.isdigit(), {"pid": pid, "stderr": (pid_p.stderr or "")[:200]}):
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 2

    app_id = f"proc:Finder:{pid}"
    started = parse(run(["vcu", "session", "start", "--surface", "desktop", "--app-id", app_id, "--json"]))
    data = started.get("data") or {}
    sid = data.get("session_id")
    step(
        "session_start",
        started.get("ok") is True and data.get("stage_hud") is True,
        {"sid": sid, "stage_hud": data.get("stage_hud"), "error": started.get("error")},
    )
    if not sid:
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 3

    try:
        snap = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        sdata = snap.get("data") or {}
        refs = sdata.get("dom_refs") or []
        titles = [str(r.get("name") or "") for r in refs]
        cg = [r for r in refs if str(r.get("role") or "") == "CGWindow"]
        seen = any(MARKER in t for t in titles)
        step(
            "snapshot_lists_folder_window",
            snap.get("ok") is True and sdata.get("source") == "ax_scene" and seen,
            {
                "source": sdata.get("source"),
                "n_refs": len(refs),
                "n_cg": len(cg),
                "titles": titles[:12],
                "error": snap.get("error"),
            },
        )
        child = run(["open", "-a", "Finder", str(ROOT / "OPENME")])
        time.sleep(0.6)
        snap2 = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        titles2 = [str(r.get("name") or "") for r in (snap2.get("data") or {}).get("dom_refs") or []]
        step(
            "open_child_window",
            child.returncode == 0 and any("OPENME" in t for t in titles2),
            {"titles": titles2[:12], "stderr": (child.stderr or "")[:160]},
        )
        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = bool(
            needed.get("session_start")
            and needed.get("snapshot_lists_folder_window")
            and needed.get("open_child_window")
        )
        report["icon_axpress"] = False
        report["note"] = (
            "Finder folder windows are listed via CGWindow. "
            "AX tree does not expose icons; AXPress/Return-to-open is not claimed."
        )
    finally:
        run(["vcu", "session", "abort", str(sid), "--json"])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
