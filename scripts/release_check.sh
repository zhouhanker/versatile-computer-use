#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"
echo "== unit/integration =="
cargo test --workspace
echo "== mock poc =="
bash scripts/poc_mock_flow.sh
echo "== login-state poc =="
bash scripts/poc_login_state.sh
echo "== app/feishu/cdp skipped this version =="
echo "SKIP poc_app_macos / poc_feishu_scene / poc_cdp — browser-only version"
echo "== release build =="
cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp
echo "== windows portability =="
echo "Windows verification is covered by GitHub Actions windows-latest job (.github/workflows/ci.yml)."
echo "Local mingw cross-link is optional; core crates avoid macOS-only APIs outside cfg(target_os=macos)."
echo "RELEASE CHECK OK"
