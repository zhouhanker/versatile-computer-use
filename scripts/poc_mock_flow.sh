#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/debug:$PATH"
USER_DIR="$ROOT/.local/poc-vcu-$(date +%s)"
mkdir -p "$USER_DIR"
export VCU_DIR="$USER_DIR"
echo "USER_DIR=$USER_DIR"

vcu --user-dir "$USER_DIR" init --json | tee /tmp/vcu-init.json >/dev/null
jq -e '.ok == true' /tmp/vcu-init.json >/dev/null

vcu-daemon --user-dir "$USER_DIR" >/tmp/vcu-daemon.log 2>&1 &
DPID=$!
cleanup() { kill "$DPID" 2>/dev/null || true; }
trap cleanup EXIT

ok=0
for i in $(seq 1 50); do
  if vcu --user-dir "$USER_DIR" daemon status >/tmp/vcu-health.json 2>/dev/null; then
    ok=1; break
  fi
  sleep 0.1
done
test "$ok" = "1"
jq -e '.ok == true' /tmp/vcu-health.json >/dev/null
echo "health ok"

vcu --user-dir "$USER_DIR" doctor | tee /tmp/vcu-doctor.json >/dev/null
jq -e '.ok == true and .data.ok == true' /tmp/vcu-doctor.json >/dev/null
echo "doctor ok"

vcu --user-dir "$USER_DIR" session start --backend mock --browser mock --json | tee /tmp/vcu-session.json >/dev/null
SID=$(jq -r '.data.session_id' /tmp/vcu-session.json)
test "$SID" != "null" && test -n "$SID"
jq -e '.data.policy.os_cursor == "deny"' /tmp/vcu-session.json >/dev/null
jq -e '.data.policy.borrow_required_for_user_tabs == true' /tmp/vcu-session.json >/dev/null
echo "session $SID"

vcu --user-dir "$USER_DIR" navigate --session "$SID" --url 'https://example.com/' --json | tee /tmp/vcu-nav.json >/dev/null
jq -e '.ok == true' /tmp/vcu-nav.json >/dev/null

vcu --user-dir "$USER_DIR" snapshot --session "$SID" --mode full --budget 5000 --json | tee /tmp/vcu-snap.json >/dev/null
jq -e '.ok == true and (.data.dom_refs|length) >= 1 and .data.vision.used == false' /tmp/vcu-snap.json >/dev/null
echo "snapshot ok"

vcu --user-dir "$USER_DIR" click --session "$SID" --ref e3 --json | tee /tmp/vcu-click.json >/dev/null
jq -e '.ok == true and .data.detail.os_cursor_used == false' /tmp/vcu-click.json >/dev/null

vcu --user-dir "$USER_DIR" type --session "$SID" --ref e4 --text 'hello-vcu' --json | tee /tmp/vcu-type.json >/dev/null
jq -e '.ok == true' /tmp/vcu-type.json >/dev/null

vcu --user-dir "$USER_DIR" extract --session "$SID" --selector button --json | tee /tmp/vcu-extract.json >/dev/null
jq -e '.data.count >= 1' /tmp/vcu-extract.json >/dev/null
echo "act/extract ok"

vcu --user-dir "$USER_DIR" tabs list --session "$SID" --json | tee /tmp/vcu-tabs.json >/dev/null
USER_TAB=$(jq -r '.data[] | select(.agent_owned == false) | .tab_id' /tmp/vcu-tabs.json | head -1)
echo "USER_TAB=$USER_TAB"
test -n "$USER_TAB"

TOKEN=$(jq -r .pairing_token "$USER_DIR/config.json")
EP=$(cat "$USER_DIR/daemon.endpoint")
CODE=$(curl -s -o /tmp/vcu-user-nav.json -w '%{http_code}' -X POST "$EP/v1/session/$SID/navigate" \
  -H "Content-Type: application/json" -H "X-Vcu-Token: $TOKEN" \
  -d "{\"url\":\"https://evil.example\",\"tab_id\":\"$USER_TAB\"}")
echo "borrow-deny HTTP $CODE"
test "$CODE" = "409"
jq -e '.ok == false and .error.code == "BorrowRequired"' /tmp/vcu-user-nav.json >/dev/null

vcu --user-dir "$USER_DIR" tabs borrow --session "$SID" --tab "$USER_TAB" --json | tee /tmp/vcu-borrow.json >/dev/null
jq -e '.ok == true' /tmp/vcu-borrow.json >/dev/null
CODE=$(curl -s -o /tmp/vcu-user-nav2.json -w '%{http_code}' -X POST "$EP/v1/session/$SID/navigate" \
  -H "Content-Type: application/json" -H "X-Vcu-Token: $TOKEN" \
  -d "{\"url\":\"https://mail.example.com/x\",\"tab_id\":\"$USER_TAB\"}")
echo "borrow-allow HTTP $CODE"
test "$CODE" = "200"
jq -e '.ok == true' /tmp/vcu-user-nav2.json >/dev/null
echo "borrow ok"

cat > /tmp/vcu-osact.json <<'JSON'
{"type":"os_cursor_move","target":{},"args":{"x":10,"y":10}}
JSON
set +e
vcu --user-dir "$USER_DIR" act --session "$SID" --action-json /tmp/vcu-osact.json >/tmp/vcu-os.json
set -e
jq -e '.ok == false and .error.code == "OsCursorDenied"' /tmp/vcu-os.json >/dev/null
echo "os_cursor deny ok"

set +e
vcu --user-dir "$USER_DIR" snapshot --session "$SID" --mode a11y --force-vision --json >/tmp/vcu-vis.json
set -e
jq -e '.ok == false and .error.code == "VisionProviderRequired"' /tmp/vcu-vis.json >/dev/null
echo "vision required ok"

vcu --user-dir "$USER_DIR" model set vision --base-url http://127.0.0.1:9 --model dummy --api-key-env VCU_VISION_API_KEY --json >/tmp/vcu-model.json
vcu --user-dir "$USER_DIR" model set-policy --mode dom_only --json >/tmp/vcu-policy.json
jq -e '.ok == true' /tmp/vcu-policy.json >/dev/null

vcu --user-dir "$USER_DIR" agent blackboard --session "$SID" --json >/tmp/vcu-bb.json
vcu --user-dir "$USER_DIR" session checkpoint "$SID" --json >/tmp/vcu-cp.json
jq -e '.ok == true' /tmp/vcu-cp.json >/dev/null
test -f "$USER_DIR/sessions/$SID/blackboard.json"

vcu --user-dir "$USER_DIR" session stop "$SID" --json >/tmp/vcu-stop.json
jq -e '.ok == true' /tmp/vcu-stop.json >/dev/null

vcu --user-dir "$USER_DIR" install-skill --harness generic --dest "$USER_DIR/skills/vcu" --json >/tmp/vcu-skill.json
test -f "$USER_DIR/skills/vcu/SKILL.md"

vcu --user-dir "$USER_DIR" mcp print-config --json >/tmp/vcu-mcp.json
jq -e '.ok == true' /tmp/vcu-mcp.json >/dev/null

echo "ALL POC CHECKS PASSED"
