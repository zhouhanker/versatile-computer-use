#!/usr/bin/env python3
"""CU-D-041: type into a throwaway Terminal.app window without Return.

Never targets Ghostty. Refuses newlines. Return without confirm_send must fail.
"""
from __future__ import annotations

import json
import subprocess
import tempfile
import time
from pathlib import Path

MARKER = f"VCU-D-041-{int(time.time())}"
OUT = Path(".local/desktop-cu/terminal-041.json")


def run(cmd, timeout=25):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:600], "code": p.returncode}


def vcu_act(sid, body):
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(body, f)
        name = f.name
    try:
        return parse(run(["vcu", "act", "--session", sid, "--action-json", name, "--json"]))
    finally:
        Path(name).unlink(missing_ok=True)


def terminal_copy():
    r = run(
        [
            "osascript",
            "-e",
            'tell application "System Events" to tell process "Terminal" to click menu item "全选" of menu "编辑" of menu bar 1',
            "-e",
            "delay 0.15",
            "-e",
            'tell application "System Events" to tell process "Terminal" to click menu item "拷贝" of menu "编辑" of menu bar 1',
        ]
    )
    time.sleep(0.2)
    clip = run(["pbpaste"]).stdout or ""
    return r.returncode == 0, clip


def main() -> int:
    report = {"ok": False, "marker": MARKER, "steps": []}
    launched = run(["pgrep", "-x", "Terminal"]).returncode != 0

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    if launched:
        run(["open", "-a", "Terminal"])
        time.sleep(1.0)
    step("launch_or_attach", True, {"launched": launched})

    pid_p = run(["osascript", "-e", 'tell application "System Events" to unix id of process "Terminal"'])
    pid = (pid_p.stdout or "").strip()
    if not step("pid", pid.isdigit(), {"pid": pid, "stderr": (pid_p.stderr or "")[:200]}):
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 2

    app_id = f"proc:Terminal:{pid}"
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
        refs = (snap.get("data") or {}).get("dom_refs") or []
        cg = [r for r in refs if str(r.get("role") or "") == "CGWindow"]
        step(
            "snapshot_cg_window",
            snap.get("ok") is True and bool(cg),
            {
                "source": (snap.get("data") or {}).get("source"),
                "n_cg": len(cg),
                "titles": [r.get("name") for r in cg[:6]],
                "error": snap.get("error"),
            },
        )
        ref = (cg[0].get("ref") if cg else None) or "w1"
        typed = parse(
            run(["vcu", "type", "--session", sid, "--tab", app_id, "--ref", str(ref), "--text", MARKER, "--json"])
        )
        tdata = typed.get("data") or {}
        detail = tdata.get("detail") if isinstance(tdata.get("detail"), dict) else {}
        step(
            "type_no_return",
            typed.get("ok") is True
            and tdata.get("ok") is not False
            and detail.get("os_cursor_used") is not True
            and detail.get("input_path") == "ax_menu_paste",
            {
                "input_path": detail.get("input_path"),
                "os_cursor_used": detail.get("os_cursor_used"),
                "result": detail.get("result"),
                "error": typed.get("error"),
            },
        )
        time.sleep(0.3)
        copied, clip = terminal_copy()
        step("verify_paste", copied and MARKER in clip, {"excerpt": clip[:160]})

        nl = parse(
            run(
                [
                    "vcu",
                    "type",
                    "--session",
                    sid,
                    "--tab",
                    app_id,
                    "--ref",
                    str(ref),
                    "--text",
                    MARKER + "\n",
                    "--json",
                ]
            )
        )
        step(
            "newline_rejected",
            nl.get("ok") is False,
            {"error": nl.get("error")},
        )

        keyed = vcu_act(
            sid,
            {
                "type": "key",
                "target": {"tab_id": app_id, "ref": str(ref)},
                "args": {"key": "return", "tab_id": app_id},
            },
        )
        err = keyed.get("error") or {}
        code = err.get("code") if isinstance(err, dict) else None
        step(
            "return_gated",
            keyed.get("ok") is False and code == "FocusPolicyViolation",
            {"error": err},
        )

        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = all(
            needed.get(n)
            for n in (
                "session_start",
                "snapshot_cg_window",
                "type_no_return",
                "verify_paste",
                "newline_rejected",
                "return_gated",
            )
        )
        report["ghostty_touched"] = False
        report["return_used"] = False
    finally:
        run(["vcu", "session", "abort", str(sid), "--json"])
        if launched:
            run(["killall", "Terminal"])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
