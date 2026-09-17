#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/debug:$PATH"
PORT="${VCU_CDP_PORT:-9333}"
CDP_URL="${VCU_CDP_URL:-http://127.0.0.1:$PORT}"
STARTED_CHROME=0
CHROME_PID=""

cleanup() {
  if [[ "$STARTED_CHROME" == "1" && -n "${CHROME_PID}" ]]; then
    kill "$CHROME_PID" 2>/dev/null || true
  fi
  if [[ -n "${DPID:-}" ]]; then
    kill "$DPID" 2>/dev/null || true
  fi
}
trap cleanup EXIT

ensure_cdp() {
  if curl -fsS "$CDP_URL/json/version" >/tmp/vcu-cdp-ver.json 2>/dev/null; then
    return 0
  fi
  local chrome=""
  if [[ -x "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome" ]]; then
    chrome="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
  elif [[ -x "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" ]]; then
    chrome="/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
  elif command -v google-chrome >/dev/null 2>&1; then
    chrome="$(command -v google-chrome)"
  elif command -v chromium >/dev/null 2>&1; then
    chrome="$(command -v chromium)"
  else
    echo "SKIP: no Chrome/Edge binary and no CDP at $CDP_URL"
    exit 0
  fi
  local profile="$ROOT/.local/chrome-cdp-auto-$PORT"
  mkdir -p "$profile"
  "$chrome" \
    --remote-debugging-port="$PORT" \
    --remote-debugging-address=127.0.0.1 \
    --user-data-dir="$profile" \
    --no-first-run \
    --no-default-browser-check \
    --disable-background-networking \
    --headless=new \
    about:blank >/tmp/vcu-chrome-auto.log 2>&1 &
  CHROME_PID=$!
  STARTED_CHROME=1
  for i in $(seq 1 50); do
    if curl -fsS "$CDP_URL/json/version" >/tmp/vcu-cdp-ver.json 2>/dev/null; then
      return 0
    fi
    sleep 0.2
  done
  echo "FAIL: started chrome but CDP not ready"; tail -30 /tmp/vcu-chrome-auto.log || true
  exit 1
}

ensure_cdp
echo "CDP: $(jq -r .Browser /tmp/vcu-cdp-ver.json)"

USER_DIR="$ROOT/.local/poc-cdp-$(date +%s)"
mkdir -p "$USER_DIR"
vcu --user-dir "$USER_DIR" init --json >/dev/null
python3 - <<PY
import json
from pathlib import Path
p = Path("$USER_DIR") / "config.json"
c = json.loads(p.read_text())
c["cdp_url"] = "$CDP_URL"
# unique daemon port to avoid collisions
c["daemon_port"] = 17900 + (int(__import__('time').time()) % 200)
p.write_text(json.dumps(c, indent=2))
print("daemon_port", c["daemon_port"])
PY

vcu-daemon --user-dir "$USER_DIR" >/tmp/vcu-cdp-daemon.log 2>&1 &
DPID=$!
for i in $(seq 1 50); do
  vcu --user-dir "$USER_DIR" daemon status >/dev/null 2>&1 && break
  sleep 0.1
done

vcu --user-dir "$USER_DIR" session start --backend cdp --browser chrome --json | tee /tmp/vcu-cdp-session.json
SID=$(jq -r .data.session_id /tmp/vcu-cdp-session.json)
test -n "$SID" && test "$SID" != null
jq -e '.data.policy.os_cursor == "deny"' /tmp/vcu-cdp-session.json >/dev/null

# Ensure we are not on browser_ui
TAB=$(jq -r .data.active_tab_id /tmp/vcu-cdp-session.json)
echo "active_tab=$TAB"

vcu --user-dir "$USER_DIR" navigate --session "$SID" --url 'https://example.com' --json | tee /tmp/vcu-cdp-nav.json
jq -e '.ok == true' /tmp/vcu-cdp-nav.json >/dev/null

vcu --user-dir "$USER_DIR" snapshot --session "$SID" --mode full --budget 5000 --json | tee /tmp/vcu-cdp-snap.json
jq -e '.ok == true' /tmp/vcu-cdp-snap.json >/dev/null
jq -e '.data.a11y_summary | test("Example Domain")' /tmp/vcu-cdp-snap.json >/dev/null
REF=$(jq -r '.data.dom_refs[] | select(.name|test("Learn more|More information";"i")) | .ref' /tmp/vcu-cdp-snap.json | head -1)
echo "link_ref=$REF"
test -n "$REF" && test "$REF" != null

vcu --user-dir "$USER_DIR" extract --session "$SID" --selector 'a,h1' --json | tee /tmp/vcu-cdp-extract.json
jq -e '.ok == true and .data.count >= 1' /tmp/vcu-cdp-extract.json >/dev/null

vcu --user-dir "$USER_DIR" click --session "$SID" --ref "$REF" --json | tee /tmp/vcu-cdp-click.json
jq -e '.ok == true and .data.detail.os_cursor_used == false' /tmp/vcu-cdp-click.json >/dev/null

# After navigation click, page should still be snapshot-able
vcu --user-dir "$USER_DIR" snapshot --session "$SID" --mode a11y --budget 3000 --json | tee /tmp/vcu-cdp-snap2.json
jq -e '.ok == true' /tmp/vcu-cdp-snap2.json >/dev/null

# screenshot
vcu --user-dir "$USER_DIR" screenshot --session "$SID" --json | tee /tmp/vcu-cdp-shot.json
jq -e '.ok == true and .data.bytes > 100' /tmp/vcu-cdp-shot.json >/dev/null

# os cursor still denied on cdp
cat > /tmp/vcu-cdp-os.json <<'JSON'
{"type":"os_cursor_move","target":{},"args":{"x":1,"y":1}}
JSON
set +e
vcu --user-dir "$USER_DIR" act --session "$SID" --action-json /tmp/vcu-cdp-os.json >/tmp/vcu-cdp-os-out.json
set -e
jq -e '.ok == false and .error.code == "OsCursorDenied"' /tmp/vcu-cdp-os-out.json >/dev/null

vcu --user-dir "$USER_DIR" session stop "$SID" --json >/dev/null
echo "CDP SMOKE PASSED (real browser)"
