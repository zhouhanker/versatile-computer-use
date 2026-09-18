#!/usr/bin/env bash
# Launch a detached Edge Agent profile with the VCU extension (no UI clicks).
# macOS: use `open -n` so LaunchServices keeps a real GUI instance alive.
# Directly exec'ing the Edge binary from a non-GUI parent often exits immediately.
set -euo pipefail
EXT="${VCU_EXTENSION_DIR:-$HOME/.local/share/vcu/extension}"
PROF="${VCU_AGENT_PROFILE:-$HOME/.vcu/edge-agent-profile}"
EDGE_APP="${VCU_EDGE_APP:-/Applications/Microsoft Edge.app}"
mkdir -p "$PROF"

agent_edge_pids() {
  pgrep -f "$PROF" 2>/dev/null || true
}

if [[ -n "$(agent_edge_pids)" ]]; then
  echo "agent_edge_already_running profile=$PROF pids=$(agent_edge_pids | tr '\n' ' ')"
  exit 0
fi

PROF="$PROF" python3 - <<'PY'
from pathlib import Path
import os
p = Path(os.environ["PROF"])
for n in ("SingletonLock", "SingletonSocket", "SingletonCookie"):
    f = p / n
    try:
        f.unlink()
    except FileNotFoundError:
        pass
PY

open -n -a "$EDGE_APP" --args \
  --user-data-dir="$PROF" \
  --disable-extensions-except="$EXT" \
  --load-extension="$EXT" \
  --no-first-run --no-default-browser-check \
  --disable-sync \
  --window-size=1100,800 \
  about:blank
echo "started_agent_edge_via_open profile=$PROF extension=$EXT"

for i in $(seq 1 25); do
  if [[ -n "$(agent_edge_pids)" ]]; then
    h="$(curl -fsS http://127.0.0.1:17890/v1/health 2>/dev/null || true)"
    if printf '%s' "$h" | python3 -c 'import json,sys; d=json.loads(sys.stdin.read() or "{}"); raise SystemExit(0 if (d.get("data") or {}).get("extension_polling") else 1)' 2>/dev/null; then
      echo "extension_polling=true pids=$(agent_edge_pids | tr '\n' ' ')"
      exit 0
    fi
  fi
  sleep 1
done
echo "started but extension_polling not true yet (daemon down or SW waking) pids=$(agent_edge_pids | tr '\n' ' ')"
exit 0
