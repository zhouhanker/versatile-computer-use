#!/usr/bin/env bash
# Virtual cursor hover — no OS cursor, no UI clicks.
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
vcu daemon start --json >/tmp/vcu-daemon-start.json || true
python3 - <<'PY'
import json, subprocess, urllib.request
from pathlib import Path

h=json.loads(urllib.request.urlopen("http://127.0.0.1:17890/v1/health", timeout=2).read())
assert (h.get("data") or {}).get("extension_polling") is True, h
print("POLLING_OK")

def vcu(args):
    r=subprocess.run(["vcu", *args], capture_output=True, text=True)
    return json.loads(r.stdout or "{}")

st=vcu(["session","start","--backend","extension","--browser","edge","--json"])
sid=(st.get("data") or {}).get("session_id")
assert sid, st
vcu(["navigate","--session",sid,"--url","https://example.com/","--json"])
vcu(["wait","--session",sid,"--ms","500","--json"])
snap=vcu(["snapshot","--session",sid,"--mode","full","--budget","1500","--json"])
refs=((snap.get("data") or {}).get("dom_refs") or [])
ref=(refs[0] or {}).get("ref") if refs else None
assert ref, snap
act={"type":"hover","target":{"ref":ref},"args":{}}
Path("/tmp/vcu-hover.json").write_text(json.dumps(act))
out=vcu(["act","--session",sid,"--action-json","/tmp/vcu-hover.json"])
assert out.get("ok") is True, out
detail=((out.get("data") or {}).get("detail") or {})
assert detail.get("os_cursor_used") is False, out
ex=vcu(["extract","--session",sid,"--selector","#vcu-virtual-cursor","--json"])
n=((ex.get("data") or {}).get("count") or 0)
assert n>=1, ex
vcu(["session","stop",sid,"--json"])
print("HOVER_CURSOR_OK", "ref", ref)
print("wechat_touched=false codex_cu_touched=false")
PY
