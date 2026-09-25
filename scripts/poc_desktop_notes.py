#!/usr/bin/env python3
"""CU-D-023 Notes: AXPress a non-destructive button. Do not create/delete notes."""
from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

OUT = Path(".local/desktop-cu/notes-023.json")
UNSAFE = (
    "关闭",
    "最小化",
    "全屏",
    "删除",
    "新建",
    "添加",
    "文件夹",
    "共享",
    "close",
    "minimi",
    "full screen",
    "delete",
    "new note",
    "new folder",
    "folder",
    "share",
    "锁定",
    "lock",
)
PREFERRED = ("开关边栏", "搜索", "格式", "核对清单", "标记", "sidebar", "search")


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace")


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:800], "code": p.returncode}


def was_running():
    return run(["pgrep", "-x", "Notes"]).returncode == 0


def unsafe_name(name: str) -> bool:
    n = (name or "").lower()
    return any(tok.lower() in n for tok in UNSAFE)


def pick_button(refs):
    buttons = []
    for r in refs or []:
        role = str(r.get("role") or "").lower()
        name = str(r.get("name") or "").strip()
        if not name:
            continue
        if "button" in role and "menu" not in role and not unsafe_name(name):
            buttons.append(r)
    for r in buttons:
        name = str(r.get("name") or "")
        if any(tok.lower() in name.lower() for tok in PREFERRED):
            return r
    return buttons[0] if buttons else None


def main() -> int:
    report = {"ok": False, "steps": [], "refs": []}
    launched = False

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    running = was_running()
    if not running:
        launched = True
        act = run(["osascript", "-e", 'tell application "Notes" to activate'])
        if not step("launch_notes", act.returncode == 0, {"stderr": (act.stderr or "")[:200]}):
            OUT.write_text(json.dumps(report, indent=2))
            print(json.dumps(report, indent=2))
            return 2
        time.sleep(2.5)
    else:
        step("launch_notes", True, {"already_running": True})

    pid_p = run(["osascript", "-e", 'tell application "System Events" to unix id of process "Notes"'])
    pid = (pid_p.stdout or "").strip()
    if not step("pid", pid.isdigit(), {"pid": pid, "stderr": (pid_p.stderr or "")[:200]}):
        OUT.write_text(json.dumps(report, indent=2))
        return 3

    app_id = f"proc:Notes:{pid}"
    sid = None
    try:
        started = parse(run(["vcu", "session", "start", "--surface", "desktop", "--app-id", app_id, "--json"]))
        data = started.get("data") or {}
        sid = data.get("session_id")
        if not step(
            "session_start",
            started.get("ok") is True and data.get("stage_hud") is True,
            {"error": started.get("error"), "sid": sid, "active_app_id": data.get("active_app_id")},
        ) or not sid:
            report["ok"] = False
            OUT.write_text(json.dumps(report, indent=2))
            print(json.dumps(report, indent=2))
            return 4

        snap = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        sdata = snap.get("data") or {}
        refs = sdata.get("dom_refs") or []
        report["refs"] = [{"ref": r.get("ref"), "role": r.get("role"), "name": r.get("name")} for r in refs[:24]]
        step(
            "snapshot",
            snap.get("ok") is True,
            {"source": sdata.get("source"), "n_refs": len(refs), "error": snap.get("error")},
        )
        btn = pick_button(refs)
        if not btn:
            step("click", False, {"error": "no safe AXButton in scene", "n_refs": len(refs)})
        else:
            env = parse(
                run(
                    [
                        "vcu",
                        "click",
                        "--session",
                        sid,
                        "--tab",
                        app_id,
                        "--ref",
                        str(btn.get("ref")),
                        "--json",
                    ]
                )
            )
            data = env.get("data") if isinstance(env.get("data"), dict) else {}
            detail = data.get("detail") if isinstance(data.get("detail"), dict) else {}
            result = str(detail.get("result") or "")
            press_ok = "axpress:0" in result.lower() or result.lower().startswith("ok")
            step(
                "click",
                env.get("ok") is True
                and data.get("ok") is not False
                and detail.get("os_cursor_used") is not True
                and press_ok,
                {
                    "ref": btn.get("ref"),
                    "role": btn.get("role"),
                    "button_name": btn.get("name"),
                    "os_cursor_used": detail.get("os_cursor_used"),
                    "input_path": detail.get("input_path"),
                    "result": result[:200],
                    "error": env.get("error"),
                    "guide_overlay": (detail.get("guide") or {}).get("overlay")
                    if isinstance(detail.get("guide"), dict)
                    else None,
                },
            )
        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = bool(needed.get("session_start") and needed.get("snapshot") and needed.get("click"))
    finally:
        if sid:
            run(["vcu", "session", "abort", str(sid), "--json"])
        if launched:
            run(["osascript", "-e", 'tell application "Notes" to quit'])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
