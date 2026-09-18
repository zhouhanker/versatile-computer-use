#!/usr/bin/env bash
# Keep daemon + Agent Edge polling alive until the overnight cutoff. No UI clicks.
set -u
export PATH="$HOME/.local/bin:$PATH"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LOG="${VCU_WATCHDOG_LOG:-/tmp/vcu-overnight-watchdog.log}"
CUTOFF="${VCU_OVERNIGHT_CUTOFF:-2026-09-18 12:00:00}"
echo "watchdog start $(date) cutoff=$CUTOFF pid=$$" >>"$LOG"
while true; do
  now="$(date '+%Y-%m-%d %H:%M:%S')"
  if [[ "$now" > "$CUTOFF" ]]; then
    echo "watchdog stop $now" >>"$LOG"
    exit 0
  fi
  if curl -fsS http://127.0.0.1:17890/v1/health >/tmp/vcu-health-now.json 2>/dev/null; then
    :
  else
    echo "$now daemon_down_restart" >>"$LOG"
    echo '{"ok":false}' >/tmp/vcu-health-now.json
    vcu daemon start --json >>"$LOG" 2>&1 || true
    sleep 2
    curl -fsS http://127.0.0.1:17890/v1/health >/tmp/vcu-health-now.json 2>/dev/null || echo '{"ok":false}' >/tmp/vcu-health-now.json
  fi
  poll="$(python3 -c 'import json
try:
 d=json.load(open("/tmp/vcu-health-now.json"))
 print((d.get("data") or {}).get("extension_polling"))
except Exception:
 print("false")' 2>/dev/null || echo false)"
  if [[ "$poll" != "True" && "$poll" != "true" ]]; then
    echo "$now polling_false_restart_agent_edge poll=$poll" >>"$LOG"
    bash "$ROOT/scripts/start_agent_edge.sh" >>"$LOG" 2>&1 || true
  else
    echo "$now ok $(tr -d '\n' </tmp/vcu-health-now.json)" >>"$LOG"
  fi
  sleep 45
done
