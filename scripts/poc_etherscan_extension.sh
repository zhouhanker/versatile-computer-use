#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
vcu daemon start --json >/tmp/vcu-daemon-start.json || true
exec python3 "$ROOT/scripts/poc_etherscan_extension.py" "$@"
