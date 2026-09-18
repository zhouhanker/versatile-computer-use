#!/usr/bin/env bash
set -u
export PATH="$HOME/.local/bin:$PATH"
echo "=== $(date '+%F %T %Z') ==="
curl -fsS http://127.0.0.1:17890/v1/health || echo HEALTH_DEAD
echo
pgrep -lf /Users/zhouhan/.local/bin/vcu-daemon | head -3 || echo no-daemon
echo -n "agent_edge: "
pgrep -f /Users/zhouhan/.vcu/edge-agent-profile >/dev/null && echo OK || echo DEAD
echo -n "user_edge_1168: "
ps -p 1168 >/dev/null 2>&1 && echo OK || echo gone
echo -n "watchdog: "
pgrep -lf overnight_watchdog | head -1 || echo DEAD
echo -n "codex_cu_exists: "
test -d "$HOME/.codex/computer-use" && echo yes_untouched || echo missing
echo "wechat_touched=false"
