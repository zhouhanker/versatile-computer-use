#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PREFIX="${VCU_PREFIX:-$HOME/.local}"
BIN_DIR="$PREFIX/bin"
mkdir -p "$BIN_DIR"
cd "$ROOT"
cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp
install -m 755 target/release/vcu "$BIN_DIR/vcu"
install -m 755 target/release/vcu-daemon "$BIN_DIR/vcu-daemon"
install -m 755 target/release/vcu-mcp "$BIN_DIR/vcu-mcp"
echo "Installed to $BIN_DIR"
echo "Ensure $BIN_DIR is on PATH"
"$BIN_DIR/vcu" --version
"$BIN_DIR/vcu" init --json || true
