#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/debug:$PATH"
USER_DIR="$ROOT/.local/poc-app-$(date +%s)"
mkdir -p "$USER_DIR"
vcu --user-dir "$USER_DIR" init --json >/dev/null
python3 - <<PY
import json
from pathlib import Path
p=Path("$USER_DIR")/"config.json"
c=json.loads(p.read_text())
c["daemon_port"]=18100+(int(__import__('time').time())%200)
p.write_text(json.dumps(c,indent=2))
PY
vcu-daemon --user-dir "$USER_DIR" >/tmp/vcu-app-daemon.log 2>&1 &
DPID=$!
trap 'kill $DPID 2>/dev/null || true' EXIT
for i in $(seq 1 40); do
  vcu --user-dir "$USER_DIR" daemon status >/dev/null 2>&1 && break
  sleep 0.1
done

echo "== app windows =="
vcu --user-dir "$USER_DIR" app windows --json | tee /tmp/vcu-app-windows.json
jq -e '.ok == true' /tmp/vcu-app-windows.json >/dev/null
PLATFORM=$(jq -r '.data.platform' /tmp/vcu-app-windows.json)
echo "platform=$PLATFORM"
COUNT=$(jq -r '.data.windows|length' /tmp/vcu-app-windows.json)
echo "windows=$COUNT"

# pick first window if any
ID=$(jq -r '.data.windows[0].id // empty' /tmp/vcu-app-windows.json)
if [[ -n "$ID" ]]; then
  echo "== snapshot $ID =="
  vcu --user-dir "$USER_DIR" app snapshot "$ID" --json | tee /tmp/vcu-app-snap.json
  jq -e '.ok == true' /tmp/vcu-app-snap.json >/dev/null
else
  echo "NOTE: no allowlisted windows visible; list still succeeded"
fi

echo "== focus denied by default =="
if [[ -n "$ID" ]]; then
  set +e
  vcu --user-dir "$USER_DIR" app focus "$ID" --json >/tmp/vcu-app-focus.json
  set -e
  jq -e '.ok == false and .error.code == "FocusPolicyViolation"' /tmp/vcu-app-focus.json >/dev/null
fi

echo "== invoke denied =="
if [[ -n "$ID" ]]; then
  set +e
  vcu --user-dir "$USER_DIR" app invoke "$ID" --ref e1 --json >/tmp/vcu-app-invoke.json
  set -e
  jq -e '.ok == false and .error.code == "OsCursorDenied"' /tmp/vcu-app-invoke.json >/dev/null
fi

echo "APP MACOS POC PASSED"
