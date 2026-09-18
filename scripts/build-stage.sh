#!/usr/bin/env bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
out="${1:-"$root/target/release/vcu-stage"}"
target="${2:-}"
mkdir -p "$(dirname "$out")"
if [[ -n "$target" ]]; then
  swiftc -O -target "$target" -framework AppKit -o "$out" "$root/helpers/vcu-stage/main.swift"
else
  swiftc -O -framework AppKit -o "$out" "$root/helpers/vcu-stage/main.swift"
fi
echo "wrote $out"
