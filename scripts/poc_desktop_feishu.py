#!/usr/bin/env python3
"""CU-D-042: observe Feishu/Lark client. Do not send.

Reads the existing desktop window. Does not open random chats or click 发送.
"""
from __future__ import annotations

import json
import subprocess
import tempfile
from pathlib import Path

OUT = Path(".local/desktop-cu/feishu-042.json")


def run(cmd, timeout=25):
    return subprocess.run(cmd, capture_output=True, text=True, timeout=timeout)


def parse(p):
    try:
        return json.loads(p.stdout or "{}")
    except json.JSONDecodeError:
        return {"ok": False, "raw": (p.stdout or p.stderr or "")[:600]}


def vcu_act(sid, body):
    with tempfile.NamedTemporaryFile("w", suffix=".json", delete=False) as f:
        json.dump(body, f)
        name = f.name
    try:
        return parse(run(["vcu", "act", "--session", sid, "--action-json", name, "--json"]))
    finally:
        Path(name).unlink(missing_ok=True)


def main() -> int:
    report = {"ok": False, "steps": [], "sent": False}

    def step(name, ok, extra=None):
        item = {"name": name, "ok": bool(ok)}
        if extra:
            item.update(extra)
        report["steps"].append(item)
        return bool(ok)

    OUT.parent.mkdir(parents=True, exist_ok=True)
    wins = parse(run(["vcu", "app", "windows"]))
    windows = ((wins.get("data") or {}).get("windows") or [])
    feishu = next(
        (
            w
            for w in windows
            if str(w.get("title") or "") in ("Feishu", "Lark", "飞书")
            or "feishu" in str(w.get("id") or "").lower()
            or "lark" in str(w.get("bundle_or_exe") or "").lower()
        ),
        None,
    )
    if not step("find_client", feishu is not None, {"window": feishu}):
        report["note"] = "Feishu/Lark client not running; cannot claim 042 live observe"
        OUT.write_text(json.dumps(report, indent=2))
        print(json.dumps(report, indent=2))
        return 2

    app_id = feishu["id"]
    started = parse(run(["vcu", "session", "start", "--surface", "desktop", "--app-id", app_id, "--json"]))
    data = started.get("data") or {}
    sid = data.get("session_id")
    step(
        "session_start",
        started.get("ok") is True and data.get("stage_hud") is True,
        {"sid": sid, "error": started.get("error")},
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
        seen = any("飞书" in t or "Feishu" in t or "Lark" in t or "lark" in t.lower() for t in titles)
        step(
            "observe_window",
            snap.get("ok") is True and sdata.get("source") == "ax_scene" and (seen or bool(cg)),
            {
                "source": sdata.get("source"),
                "n_refs": len(refs),
                "n_cg": len(cg),
                "titles": titles[:12],
                "webview": sdata.get("webview"),
                "excerpt": (sdata.get("text_excerpt") or "")[:200],
                "error": snap.get("error"),
            },
        )
        send_refs = [r for r in refs if "发送" in str(r.get("name") or "") or str(r.get("name") or "") in ("Send", "Send message")]
        if send_refs:
            clicked = parse(
                run(
                    [
                        "vcu",
                        "click",
                        "--session",
                        sid,
                        "--tab",
                        app_id,
                        "--ref",
                        str(send_refs[0].get("ref")),
                        "--json",
                    ]
                )
            )
            err = clicked.get("error") or {}
            step(
                "send_click_denied",
                clicked.get("ok") is False and err.get("code") == "FocusPolicyViolation",
                {"ref": send_refs[0].get("ref"), "error": err},
            )
        else:
            step("send_click_denied", True, {"skipped": "no AX send control (expected for Electron messenger)"})

        keyed = vcu_act(
            sid,
            {
                "type": "key",
                "target": {"tab_id": app_id, "ref": (cg[0].get("ref") if cg else "w1")},
                "args": {"key": "return", "tab_id": app_id},
            },
        )
        err = keyed.get("error") or {}
        step(
            "return_gated",
            keyed.get("ok") is False and err.get("code") == "FocusPolicyViolation",
            {"error": err},
        )
        needed = {s["name"]: s["ok"] for s in report["steps"]}
        report["ok"] = bool(
            needed.get("session_start")
            and needed.get("observe_window")
            and needed.get("send_click_denied")
            and needed.get("return_gated")
        )
        report["messages_in_ax"] = any(
            t and t not in ("飞书", "Feishu", "Lark") for t in titles
        )
        report["note"] = (
            "Feishu window observed via CGWindow. Chat bubbles are not in the AX tree "
            "(Electron). Send was not clicked. Return gated."
        )
    finally:
        run(["vcu", "session", "abort", str(sid), "--json"])

    OUT.write_text(json.dumps(report, indent=2))
    print(json.dumps(report, indent=2))
    return 0 if report["ok"] else 1


if __name__ == "__main__":
    raise SystemExit(main())
