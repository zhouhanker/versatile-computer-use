#!/usr/bin/env python3
"""CU-D-043: observe System Settings read-only. Do not toggle TCC."""
from __future__ import annotations

import json
import subprocess
import time
from pathlib import Path

OUT = Path(".local/desktop-cu/settings-043.json")


def run(cmd, timeout=25):
    return subprocess.run(cmd, capture_output=True, text=True, encoding="utf-8", errors="replace", timeout=timeout)


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:600]}


def main() -> int:
    report = {"ok": False, "steps": [], "toggled_tcc": False}

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    launched = run(["pgrep", "-x", "System Settings"]).returncode != 0
    if launched:
        run(["open", "-a", "System Settings"])
        time.sleep(1.2)
    step("launch_or_attach", True, {"launched": launched})

    pid_p = run(
        [
            "osascript",
            "-e",
            'tell application "System Events" to unix id of process "System Settings"',
        ]
    )
    pid = (pid_p.stdout or "").strip()
    if not pid.isdigit():
        pid_p = run(
            [
                "osascript",
                "-e",
                'tell application "System Events" to unix id of process "系统设置"',
            ]
        )
        pid = (pid_p.stdout or "").strip()
    if not step("pid", pid.isdigit(), {"pid": pid, "stderr": (pid_p.stderr or "")[:200]}):
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 2

    # id uses spaces -> underscores in list_windows
    app_id = f"proc:System_Settings:{pid}"
    started = parse(
        run(["vcu", "session", "start", "--surface", "desktop", "--app-id", app_id, "--json"])
    )
    data = started.get("data") or {}
    sid = data.get("session_id")
    if not sid:
        # try Chinese process id
        app_id = f"proc:系统设置:{pid}"
        started = parse(
            run(["vcu", "session", "start", "--surface", "desktop", "--app-id", app_id, "--json"])
        )
        data = started.get("data") or {}
        sid = data.get("session_id")
    step(
        "session_start",
        started.get("ok") is True and data.get("stage_hud") is True,
        {"sid": sid, "app_id": app_id, "error": started.get("error")},
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
        step(
            "observe",
            snap.get("ok") is True and sdata.get("source") == "ax_scene",
            {
                "n_refs": len(refs),
                "titles": titles[:8],
                "source": sdata.get("source"),
                "error": snap.get("error"),
            },
        )
        ref = (refs[0].get("ref") if refs else "w1")
        clicked = parse(
            run(["vcu", "click", "--session", sid, "--tab", app_id, "--ref", str(ref), "--json"])
        )
        err = clicked.get("error") or {}
        step(
            "click_denied",
            clicked.get("ok") is False and err.get("code") == "FocusPolicyViolation",
            {"error": err},
        )
        doc = parse(run(["vcu", "doctor", "--json"]))
        checks = ((doc.get("data") or {}).get("checks") or [])
        ax = next((c for c in checks if c.get("name") == "macos_accessibility"), {})
        hint = str(ax.get("hint") or "")
        blob = json.dumps(doc, ensure_ascii=False)
        step(
            "doctor_hint",
            ax.get("status") in ("pass", "fail")
            and "tccutil" not in blob.lower()
            and "x-apple.systempreferences" not in blob,
            {"ax_status": ax.get("status"), "ax_hint": hint[:180]},
        )
        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = all(
            needed.get(n)
            for n in ("session_start", "observe", "click_denied", "doctor_hint")
        )
    finally:
        run(["vcu", "session", "abort", str(sid), "--json"])
        if launched:
            run(["killall", "System Settings"])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
