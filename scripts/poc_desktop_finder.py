#!/usr/bin/env python3
"""CU-D-040: list a script-created Finder window, then reveal+open a child via VCU.

Uses Launch Services (nsworkspace_reveal / nsworkspace_open). Not HID.
Does not use `tell application "Finder"`. Does not claim AXPress or Return-as-Send.
"""
from __future__ import annotations

import json
import subprocess
import tempfile
import time
from pathlib import Path

MARKER = f"VCU-D-040-{int(time.time())}"
ROOT = Path(f"/tmp/{MARKER}")
OUT = Path(".local/desktop-cu/finder-040.json")


def run(cmd, timeout=25):
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout)


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:600], "code": p.returncode}


def vcu_act(sid, tab, kind, path):
    payload = {"type": kind, "target": {"tab_id": tab}, "args": {"path": path, "tab_id": tab}}
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(payload, f)
        name = f.name
    try:
        return parse(run(["vcu", "act", "--session", sid, "--action-json", name, "--json"]))
    finally:
        Path(name).unlink(missing_ok=True)


def titles_from(snap):
    refs = (snap.get("data") or {}).get("dom_refs") or []
    return [str(r.get("name") or "") for r in refs], [r for r in refs if str(r.get("role") or "") == "CGWindow"]


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
    child = ROOT / "OPENME"
    child.mkdir(exist_ok=True)
    (child / "inside.txt").write_text(MARKER + "\n")

    pid_p = run(["osascript", "-e", 'tell application "System Events" to unix id of process "Finder"'])
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
        opened = vcu_act(sid, app_id, "open_path", str(ROOT))
        odata = opened.get("data") or {}
        detail = odata.get("detail") if isinstance(odata.get("detail"), dict) else {}
        time.sleep(0.7)
        snap = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        titles, cg = titles_from(snap)
        step(
            "open_parent_via_vcu",
            opened.get("ok") is True
            and detail.get("os_cursor_used") is not True
            and detail.get("input_path") == "nsworkspace_open"
            and any(MARKER in t for t in titles),
            {
                "input_path": detail.get("input_path"),
                "os_cursor_used": detail.get("os_cursor_used"),
                "titles": titles[:12],
                "n_cg": len(cg),
                "error": opened.get("error"),
            },
        )

        revealed = vcu_act(sid, app_id, "reveal", str(child))
        rdetail = ((revealed.get("data") or {}).get("detail") or {})
        step(
            "reveal_child",
            revealed.get("ok") is True
            and rdetail.get("input_path") == "nsworkspace_reveal"
            and rdetail.get("os_cursor_used") is not True,
            {
                "input_path": rdetail.get("input_path"),
                "os_cursor_used": rdetail.get("os_cursor_used"),
                "error": revealed.get("error"),
            },
        )

        opened_child = vcu_act(sid, app_id, "open_path", str(child))
        cdetail = ((opened_child.get("data") or {}).get("detail") or {})
        time.sleep(0.7)
        snap2 = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        titles2, _ = titles_from(snap2)
        step(
            "open_child_via_vcu",
            opened_child.get("ok") is True
            and cdetail.get("input_path") == "nsworkspace_open"
            and cdetail.get("os_cursor_used") is not True
            and any("OPENME" in t for t in titles2),
            {
                "input_path": cdetail.get("input_path"),
                "os_cursor_used": cdetail.get("os_cursor_used"),
                "titles": titles2[:12],
                "error": opened_child.get("error"),
            },
        )

        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = bool(
            needed.get("session_start")
            and needed.get("open_parent_via_vcu")
            and needed.get("reveal_child")
            and needed.get("open_child_via_vcu")
        )
        report["axpress"] = False
        report["return_used"] = False
        report["note"] = (
            "Select/open uses NSWorkspace (Launch Services). "
            "Finder AX still has no icons; Return remains Send-gated."
        )
    finally:
        run(["vcu", "session", "abort", str(sid), "--json"])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
