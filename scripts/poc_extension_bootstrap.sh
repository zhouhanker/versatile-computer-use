#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
if ! curl -fsS http://127.0.0.1:17890/v1/health >/dev/null 2>&1; then
  nohup vcu-daemon >/tmp/vcu-daemon-user.log 2>&1 &
  for _ in $(seq 1 40); do
    curl -fsS http://127.0.0.1:17890/v1/health >/dev/null 2>&1 && break
    sleep 0.1
  done
fi
BOOT="$(curl -fsS -X POST http://127.0.0.1:17890/v1/extension/bootstrap \
  -H 'Content-Type: application/json' \
  -d '{"client":"vcu-extension","version":"0.1.0"}')"
echo "$BOOT"
TOKEN="$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["data"]["token"])' "$BOOT")"
HELLO="$(curl -fsS -X POST http://127.0.0.1:17890/v1/extension/hello \
  -H "X-Vcu-Token: $TOKEN" -H 'Content-Type: application/json' -d '{}')"
echo "$HELLO"
python3 -c 'import json,sys
boot=json.loads(sys.argv[1]); hello=json.loads(sys.argv[2])
assert boot["ok"] and boot["data"]["token"]
assert hello["ok"] and hello["data"]["connected"] is True
print("EXTENSION_BOOTSTRAP_POC_OK")' "$BOOT" "$HELLO"
vcu daemon start --json | tee /tmp/vcu-daemon-start-again.json
python3 -c 'import json
d=json.load(open("/tmp/vcu-daemon-start-again.json"))
assert d.get("ok") is True
print("SINGLE_INSTANCE_OK", d)'
echo "wechat_touched=false codex_cu_touched=false"
