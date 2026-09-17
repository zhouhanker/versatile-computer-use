#!/usr/bin/env bash
set -euo pipefail
PREFIX="${VCU_PREFIX:-$HOME/.local}"
BIN="${VCU_BIN_DIR:-$PREFIX/bin}"
LABEL="com.vcu.daemon"
PLIST="$HOME/Library/LaunchAgents/${LABEL}.plist"
VCU_DIR="${VCU_DIR:-$HOME/.vcu}"
mkdir -p "$(dirname "$PLIST")" "$VCU_DIR" "$HOME/Library/Logs/vcu"
test -x "$BIN/vcu-daemon" || { echo "missing $BIN/vcu-daemon — install first" >&2; exit 1; }
cat > "$PLIST" <<PL
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>${LABEL}</string>
  <key>ProgramArguments</key>
  <array>
    <string>${BIN}/vcu-daemon</string>
    <string>--user-dir</string>
    <string>${VCU_DIR}</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>${HOME}/Library/Logs/vcu/daemon.out.log</string>
  <key>StandardErrorPath</key><string>${HOME}/Library/Logs/vcu/daemon.err.log</string>
  <key>EnvironmentVariables</key>
  <dict>
    <key>PATH</key><string>${BIN}:/usr/bin:/bin</string>
    <key>VCU_DIR</key><string>${VCU_DIR}</string>
  </dict>
</dict></plist>
PL
launchctl bootout "gui/$(id -u)/${LABEL}" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$PLIST"
launchctl kickstart -k "gui/$(id -u)/${LABEL}" 2>/dev/null || true
echo "Installed LaunchAgent $PLIST"
