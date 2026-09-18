#!/usr/bin/env bash
set -uo pipefail
export PATH="$HOME/.local/bin:$PATH"
vcu daemon start --json >/tmp/vcu-daemon-start.json || true
python3 - <<'PY'
import json, urllib.request
h = json.loads(urllib.request.urlopen("http://127.0.0.1:17890/v1/health", timeout=2).read())
assert (h.get("data") or {}).get("extension_polling") is True, h
print("POLLING_OK")
PY
SID=""
for _ in 1 2 3; do
  vcu session start --backend extension --browser edge --json > /tmp/vcu-ext-session.json || true
  SID="$(python3 -c 'import json; d=json.load(open("/tmp/vcu-ext-session.json")); print((d.get("data") or {}).get("session_id") or "")')"
  if [ -n "$SID" ]; then
    break
  fi
  sleep 1
done
test -n "$SID"
echo "SID=$SID"
vcu navigate --session "$SID" --url "https://example.com/" --json
sleep 2
ok_snap=0
for _ in 1 2 3; do
  if vcu snapshot --session "$SID" --mode full --budget 2000 --json > /tmp/vcu-ext-snap.json; then
    if python3 -c "import json; d=json.load(open(\"/tmp/vcu-ext-snap.json\")); raise SystemExit(0 if d.get(\"ok\") else 1)"; then
      ok_snap=1
      break
    fi
  fi
  sleep 2
done
test "$ok_snap" = 1
python3 - <<'PY'
import json
d = json.load(open("/tmp/vcu-ext-snap.json"))
assert d.get("ok") is True, d
text = ((d.get("data") or {}).get("text_excerpt") or "")
print("SNAP_OK", "len", len(text))
PY
echo "wechat_touched=false codex_cu_touched=false"
