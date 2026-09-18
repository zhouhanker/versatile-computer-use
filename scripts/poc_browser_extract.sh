#!/usr/bin/env bash
# Login-state DOM extract via USER Edge extension. No navigate, no HUD, no Agent Edge.
# Honest gate: AX chrome fallback is NOT a pass.
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
vcu browser login-state --json > /tmp/vcu-extract-login.json
python3 - <<'PY'
import json, subprocess
from pathlib import Path
d=json.loads(Path("/tmp/vcu-extract-login.json").read_text())
data=d.get("data") or d
if data.get("extension_profile") != "user":
    print("SKIP: extension_profile is", data.get("extension_profile"), "need user")
    raise SystemExit(0)
ping = subprocess.run(["vcu","browser","ping","--json"], capture_output=True, text=True)
Path("/tmp/vcu-browser-ping.json").write_text(ping.stdout or ping.stderr or "")
try:
    pv=json.loads(ping.stdout)
except Exception:
    print("FAIL browser ping non-json", ping.returncode, (ping.stdout or "")[:300])
    raise SystemExit(1)
if pv.get("ok") is not True or (pv.get("data") or {}).get("pong") is not True:
    err=(pv.get("error") or {})
    print("FAIL browser ping stale or not pong", err.get("code"), err.get("message"))
    print("Reload VCU Browser Bridge on edge://extensions. Never click Allow.")
    raise SystemExit(1)
print("PASS browser ping", (pv.get("data") or {}).get("extension"))
p = subprocess.run(["vcu","browser","extract","--selector","a","--json"], capture_output=True, text=True)
Path("/tmp/vcu-browser-extract.json").write_text(p.stdout or p.stderr or "")
try:
    v=json.loads(p.stdout)
except Exception:
    print("FAIL browser extract non-json", p.returncode, (p.stdout or "")[:300])
    raise SystemExit(1)
if v.get("ok") is not True:
    err=(v.get("error") or {})
    print("FAIL browser extract", err.get("code"), err.get("message"))
    raise SystemExit(1)
data=v.get("data") or {}
assert data.get("hud") is False
assert data.get("login_state") is True
assert data.get("os_cursor_used") is False
assert data.get("source") == "extension_dom", data.get("source")
assert data.get("source") != "ax_scene_fallback"
print("PASS browser extract tab", data.get("tab_id"), "count", data.get("count"), "source", data.get("source"))
PY
