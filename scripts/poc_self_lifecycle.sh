#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/release:$PATH"
bash "$ROOT/scripts/pack-release.sh" >/tmp/vcu-pack-life.log
PREF="$ROOT/.local/life-prefix-$(date +%s)"
mkdir -p "$PREF"
VCU_BASE_URL="file://$ROOT/dist" VCU_PREFIX="$PREF" bash "$ROOT/scripts/install/install.sh" >/tmp/vcu-life-inst.log
export PATH="$PREF/bin:$PATH"
vcu self info --json | jq -e '.data.bins_present.vcu==true'
if [[ "$(uname -s)" == "Darwin" ]]; then
  vcu self info --json | jq -e '.data.bins_present["vcu-stage"]==true'
fi
# update from same pack
vcu self update --prefix "$PREF" --base-url "file://$ROOT/dist" --json | jq -e '.data.updated==true'
vcu self uninstall --prefix "$PREF" --yes --json | jq -e '.data.uninstalled==true'
test ! -x "$PREF/bin/vcu"
test ! -e "$PREF/bin/vcu-stage"
# ensure codex path still exists
test -d "$HOME/.codex/computer-use/Codex Computer Use.app"
echo "SELF_LIFECYCLE_POC_PASSED codex_untouched=true"
