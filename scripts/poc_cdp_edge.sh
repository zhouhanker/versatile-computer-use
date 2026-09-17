#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/debug:$PATH"
PORT="${VCU_CDP_PORT:-9334}"
EDGE="/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
if [[ ! -x "$EDGE" ]]; then
  echo "SKIP: Microsoft Edge not installed"
  exit 0
fi
PROFILE="$ROOT/.local/edge-cdp-$PORT"
mkdir -p "$PROFILE"
pkill -f "remote-debugging-port=$PORT" 2>/dev/null || true
sleep 0.2
"$EDGE" --remote-debugging-port="$PORT" --remote-debugging-address=127.0.0.1 \
  --user-data-dir="$PROFILE" --no-first-run --no-default-browser-check \
  --headless=new about:blank >/tmp/vcu-edge.log 2>&1 &
EPID=$!
trap 'kill $EPID 2>/dev/null || true; kill ${DPID:-} 2>/dev/null || true' EXIT
for i in $(seq 1 40); do
  curl -fsS "http://127.0.0.1:$PORT/json/version" >/tmp/vcu-edge-ver.json 2>/dev/null && break
  sleep 0.2
done
curl -fsS "http://127.0.0.1:$PORT/json/version" >/tmp/vcu-edge-ver.json
echo "EDGE: $(jq -r .Browser /tmp/vcu-edge-ver.json)"
USER_DIR="$ROOT/.local/poc-edge-$(date +%s)"
mkdir -p "$USER_DIR"
vcu --user-dir "$USER_DIR" init --json >/dev/null
python3 - "$USER_DIR" "$PORT" <<'PY'
import json,sys,time
from pathlib import Path
p=Path(sys.argv[1])/"config.json"
c=json.loads(p.read_text())
c["cdp_url"]=f"http://127.0.0.1:{sys.argv[2]}"
c["daemon_port"]=18400+(int(time.time())%200)
p.write_text(json.dumps(c,indent=2))
PY
vcu-daemon --user-dir "$USER_DIR" >/tmp/vcu-edge-daemon.log 2>&1 &
DPID=$!
for i in $(seq 1 40); do vcu --user-dir "$USER_DIR" daemon status >/dev/null 2>&1 && break; sleep 0.1; done
SID=$(vcu --user-dir "$USER_DIR" session start --backend cdp --browser edge --json | jq -r .data.session_id)
vcu --user-dir "$USER_DIR" navigate --session "$SID" --url https://example.com --json | jq -e .ok >/dev/null
vcu --user-dir "$USER_DIR" snapshot --session "$SID" --mode a11y --json | tee /tmp/vcu-edge-snap.json >/dev/null
jq -e '.ok==true and (.data.a11y_summary|test("Example Domain"))' /tmp/vcu-edge-snap.json >/dev/null
vcu --user-dir "$USER_DIR" session stop "$SID" --json >/dev/null
echo "EDGE CDP SMOKE PASSED"
