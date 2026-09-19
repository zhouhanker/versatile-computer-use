#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/debug:$PATH"
USER_DIR="$ROOT/.local/poc-extra-$(date +%s)"
mkdir -p "$USER_DIR"
export VCU_AUDIT=1
vcu --user-dir "$USER_DIR" init --json >/dev/null
python3 - "$USER_DIR" <<'PY'
import json, sys, time
from pathlib import Path
p = Path(sys.argv[1]) / "config.json"
c = json.loads(p.read_text())
c["daemon_port"] = 18200 + (int(time.time()) % 200)
p.write_text(json.dumps(c, indent=2))
PY
vcu-daemon --user-dir "$USER_DIR" >/tmp/vcu-extra-daemon.log 2>&1 &
DPID=$!
trap 'kill $DPID 2>/dev/null || true' EXIT
for i in $(seq 1 40); do vcu --user-dir "$USER_DIR" daemon status >/dev/null 2>&1 && break; sleep 0.1; done
SID=$(vcu --user-dir "$USER_DIR" session start --backend mock --json | jq -r .data.session_id)
vcu --user-dir "$USER_DIR" navigate --session "$SID" --url https://example.com --json >/dev/null
vcu --user-dir "$USER_DIR" scroll --session "$SID" --dy 300 --json | tee /tmp/vcu-scroll.json >/dev/null
jq -e '.ok == true and .data.detail.os_cursor_used == false' /tmp/vcu-scroll.json >/dev/null
vcu --user-dir "$USER_DIR" wait --session "$SID" --ms 50 --json | tee /tmp/vcu-wait.json >/dev/null
jq -e '.ok == true' /tmp/vcu-wait.json >/dev/null
vcu --user-dir "$USER_DIR" click --session "$SID" --ref e3 --json >/dev/null
test -f "$USER_DIR/audit.jsonl"
grep -q 'browser.click' "$USER_DIR/audit.jsonl"
set +e
printf '%s\n' '{"type":"os_cursor_move","target":{},"args":{}}' > "$USER_DIR/deny-os-cursor.json"
vcu --user-dir "$USER_DIR" act --session "$SID" --action-json "$USER_DIR/deny-os-cursor.json" --json >/tmp/vcu-os-cursor-deny.json
set -e
jq -e '.ok == false and .error.code == "OsCursorDenied"' /tmp/vcu-os-cursor-deny.json >/dev/null
vcu --user-dir "$USER_DIR" session stop "$SID" --json >/dev/null
echo "EXTRA ACTIONS POC PASSED"
