#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "== unit/integration =="
cargo test --workspace
echo "== mock poc =="
bash scripts/poc_mock_flow.sh
echo "== cdp poc (auto chrome if needed) =="
bash scripts/poc_cdp_smoke.sh
echo "== app poc (macos) =="
if [[ "$(uname -s)" == "Darwin" ]]; then bash scripts/poc_app_macos.sh; else echo SKIP app poc; fi
echo "== release build =="
cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp
echo "== windows portability =="
echo "Windows verification is covered by GitHub Actions windows-latest job (.github/workflows/ci.yml)."
echo "Local mingw cross-link is optional; core crates avoid macOS-only APIs outside cfg(target_os=macos)."
echo "RELEASE CHECK OK"
