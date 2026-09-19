#!/usr/bin/env python3
"""CU-D-023: type a marker into a throwaway TextEdit document via desktop surface."""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

MARKER = f"VCU-D-023-{int(time.time())}"
OUT = Path(".local/desktop-cu/textedit-023.json")


def run(cmd):
    p = subprocess.run(cmd, capture_output=True, text=True)
    return p


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:500], "code": p.returncode}


def main() -> int:
    report = {"ok": False, "marker": MARKER, "steps": []}

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    opened = run(["osascript", "-e", 'tell application "TextEdit" to make new document'])
    step("open_textedit", opened.returncode == 0, {"stderr": (opened.stderr or "")[:240]})
    pid_p = run(["osascript", "-e", 'tell application "System Events" to unix id of process "TextEdit"'])
    pid = (pid_p.stdout or "").strip()
    if not step("pid", pid.isdigit(), {"pid": pid, "stderr": (pid_p.stderr or "")[:200]}):
        OUT.write_text(json.dumps(report, indent=2))
        return 2

    app_id = f"proc:TextEdit:{pid}"
    started = parse(run(["vcu", "session", "start", "--surface", "desktop", "--app-id", app_id, "--json"]))
    data = started.get("data") or {}
    sid = data.get("session_id")
    step(
        "session_start",
        started.get("ok") is True and data.get("stage_hud") is True,
        {"error": started.get("error"), "stage_hud": data.get("stage_hud"), "sid": sid},
    )
    if not sid:
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 3

    try:
        snap = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        sdata = snap.get("data") or {}
        step(
            "snapshot",
            snap.get("ok") is True,
            {
                "kind": sdata.get("kind"),
                "surface": sdata.get("surface"),
                "source": sdata.get("source"),
                "error": snap.get("error"),
            },
        )
        refs = sdata.get("dom_refs") or []
        text_ref = None
        for r in refs:
            role = str(r.get("role") or "").lower()
            if "text" in role or "area" in role or r.get("value") is not None:
                text_ref = r.get("ref")
                break
        if not text_ref and refs:
            text_ref = refs[0].get("ref")
        typed = {"ok": False}
        if text_ref:
            typed = parse(
                run(["vcu", "type", "--session", sid, "--ref", str(text_ref), "--text", MARKER, "--json"])
            )
        tdata = typed.get("data") or {}
        step(
            "type",
            typed.get("ok") is True and tdata.get("os_cursor_used") is not True,
            {
                "ref": text_ref,
                "os_cursor_used": tdata.get("os_cursor_used"),
                "input_path": tdata.get("input_path"),
                "error": typed.get("error"),
            },
        )
        time.sleep(0.5)
        verify = run(["osascript", "-e", 'tell application "TextEdit" to get text of document 1'])
        body = verify.stdout or ""
        step("verify_text", MARKER in body, {"excerpt": body[:160], "stderr": (verify.stderr or "")[:200]})
        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = bool(needed.get("session_start") and needed.get("type") and needed.get("verify_text"))
    finally:
        run(["vcu", "session", "abort", "--id", str(sid), "--json"])
        run(["osascript", "-e", 'tell application "TextEdit" to close front document saving no'])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
