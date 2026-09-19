#!/usr/bin/env python3
"""CU-D-023: type a marker into a throwaway TextEdit document via desktop surface.

Does not touch USER Edge tab groups. Raises Stage HUD briefly; always abort + close.
"""
from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

MARKER = f"VCU-D-023-{int(time.time())}"
OUT = Path(".local/desktop-cu/textedit-023.json")


def run(cmd):
    return subprocess.run(cmd, capture_output=True, text=True)


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {
            "ok": False,
            "raw": (p.stdout or p.stderr or "")[:800],
            "code": p.returncode,
        }


def action_ok(env):
    data = env.get("data") if isinstance(env, dict) else None
    if not isinstance(data, dict):
        data = {}
    detail = data.get("detail") if isinstance(data.get("detail"), dict) else {}
    envelope_ok = env.get("ok") is True
    inner_ok = data.get("ok")
    if inner_ok is False:
        return False, data, detail
    return envelope_ok, data, detail


def is_editable_role(role: str) -> bool:
    r = (role or "").lower()
    if "scroll" in r or "static" in r or "webarea" in r:
        return False
    return any(
        tok in r
        for tok in (
            "textarea",
            "text area",
            "textfield",
            "text field",
            "searchfield",
            "combo",
            "axtext",
        )
    ) or r == "text"


def pick_text_refs(refs):
    scored = []
    for r in refs or []:
        role = str(r.get("role") or "")
        if is_editable_role(role):
            # Prefer text areas over fields.
            score = 0 if ("area" in role.lower() or "textarea" in role.lower()) else 1
            scored.append((score, r))
    scored.sort(key=lambda x: (x[0], str(x[1].get("ref") or "")))
    return [r for _, r in scored]


def main() -> int:
    report = {"ok": False, "marker": MARKER, "steps": [], "refs": []}

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    opened = run(["osascript", "-e", 'tell application "TextEdit" to make new document'])
    step("open_textedit", opened.returncode == 0, {"stderr": (opened.stderr or "")[:240]})
    pid_p = run(
        ["osascript", "-e", 'tell application "System Events" to unix id of process "TextEdit"']
    )
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
        {
            "error": started.get("error"),
            "stage_hud": data.get("stage_hud"),
            "sid": sid,
            "active_app_id": data.get("active_app_id"),
        },
    )
    if not sid:
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 3

    try:
        snap = parse(run(["vcu", "snapshot", "--session", sid, "--tab", app_id, "--json"]))
        sdata = snap.get("data") or {}
        refs = sdata.get("dom_refs") or []
        report["refs"] = [
            {"ref": r.get("ref"), "role": r.get("role"), "name": r.get("name")}
            for r in refs[:24]
        ]
        step(
            "snapshot",
            snap.get("ok") is True,
            {
                "kind": sdata.get("kind"),
                "surface": sdata.get("surface"),
                "source": sdata.get("source"),
                "n_refs": len(refs),
                "error": snap.get("error"),
            },
        )
        candidates = pick_text_refs(refs)
        if not candidates and refs:
            # Last resort: try every ref until verify, but record that none looked like text.
            candidates = list(refs)
        typed_ok = False
        last_type = None
        chosen = None
        for cand in candidates:
            ref = cand.get("ref")
            if not ref:
                continue
            env = parse(
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
                        MARKER,
                        "--json",
                    ]
                )
            )
            ok, tdata, detail = action_ok(env)
            last_type = {
                "ref": ref,
                "role": cand.get("role"),
                "envelope_ok": env.get("ok"),
                "data_ok": tdata.get("ok"),
                "os_cursor_used": detail.get("os_cursor_used"),
                "input_path": detail.get("input_path"),
                "process": detail.get("process"),
                "tab_id": detail.get("tab_id"),
                "result": detail.get("result"),
                "error": env.get("error"),
            }
            time.sleep(0.4)
            verify = run(
                ["osascript", "-e", 'tell application "TextEdit" to get text of document 1']
            )
            body = verify.stdout or ""
            last_type["excerpt"] = body[:200]
            if (
                ok
                and detail.get("os_cursor_used") is not True
                and MARKER in body
            ):
                typed_ok = True
                chosen = last_type
                break
        step("type", typed_ok, chosen or last_type or {"ref": None, "n_candidates": len(candidates)})
        body = ""
        verify = run(["osascript", "-e", 'tell application "TextEdit" to get text of document 1'])
        body = verify.stdout or ""
        step(
            "verify_text",
            MARKER in body,
            {"excerpt": body[:200], "stderr": (verify.stderr or "")[:200]},
        )
        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = bool(
            needed.get("session_start") and needed.get("type") and needed.get("verify_text")
        )
    finally:
        run(["vcu", "session", "abort", str(sid), "--json"])
        run(["osascript", "-e", 'tell application "TextEdit" to close front document saving no'])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
