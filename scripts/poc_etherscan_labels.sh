#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
OUT_DIR="${VCU_ETHERSCAN_OUT:-$HOME/vcu-etherscan-labels}"
mkdir -p "$OUT_DIR"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

pkill -f '[v]cu-daemon' 2>/dev/null || true
sleep 0.3
nohup vcu-daemon >/tmp/vcu-daemon-user.log 2>&1 &
sleep 0.6

vcu browser discover --json | tee "$OUT_DIR/discover.json"
COUNT=$(jq -r '.data.count // 0' "$OUT_DIR/discover.json")
MODE="NEW_HEADLESS_NO_LOGIN"
CDP=""
if [[ "${COUNT}" -gt 0 ]]; then
  CDP=$(jq -r '.data.found[0].endpoint' "$OUT_DIR/discover.json")
  MODE="TAKEOVER_CDP"
else
  PORT=9340
  PROFILE="$ROOT/.local/edge-etherscan-temp"
  mkdir -p "$PROFILE"
  pkill -f "remote-debugging-port=${PORT}" 2>/dev/null || true
  sleep 0.2
  "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" \
    --remote-debugging-port=$PORT --remote-debugging-address=127.0.0.1 \
    --user-data-dir="$PROFILE" --no-first-run --disable-background-networking \
    "https://etherscan.io/labelcloud" >/tmp/vcu-edge-eth.log 2>&1 &
  echo $! > /tmp/vcu-edge-eth.pid
  ok=0
  for i in $(seq 1 60); do
    if curl -fsS "http://127.0.0.1:$PORT/json/version" >/tmp/vcu-eth-ver.json 2>/dev/null; then ok=1; break; fi
    sleep 0.25
  done
  if [[ "$ok" != 1 ]]; then
    echo "Edge CDP failed"; cat /tmp/vcu-edge-eth.log | tail -20
    echo "$MODE" > "$OUT_DIR/mode.txt"
    exit 1
  fi
  CDP="http://127.0.0.1:$PORT"
fi
echo "MODE=$MODE" | tee "$OUT_DIR/mode.txt"
echo "CDP=$CDP" | tee "$OUT_DIR/cdp.txt"
vcu config set-cdp "$CDP" --json >/dev/null

SID=$(vcu session start --backend cdp --browser edge --json | tee "$OUT_DIR/session.json" | jq -r .data.session_id)
echo "SID=$SID"
vcu navigate --session "$SID" --url 'https://etherscan.io/labelcloud' --json | tee "$OUT_DIR/nav.json"
vcu wait --session "$SID" --ms 2500 --json >/dev/null || true
vcu snapshot --session "$SID" --mode full --budget 15000 --json | tee "$OUT_DIR/snap.json" >/dev/null
vcu extract --session "$SID" --selector 'a,button,div,span,li,h1,h2,h3' --json | tee "$OUT_DIR/extract.json" >/dev/null || true

python3 - <<'PY' "$OUT_DIR"
import json,re,sys
from pathlib import Path
out=Path(sys.argv[1])
snap=json.loads((out/"snap.json").read_text())
data=snap.get("data") or {}
text=data.get("text_excerpt") or ""
a11y=data.get("a11y_summary") or ""
blob=(text+" "+a11y).lower()
login=any(k in blob for k in ["sign in","log in","login","sign-in","connect wallet","captcha","verify you are human"])
mode=(out/"mode.txt").read_text().strip()
report={
  "mode": mode,
  "likely_login_required_or_wall": login,
  "takeover": "TAKEOVER" in mode,
  "new_browser_no_user_cookies": "NEW_" in mode or mode.endswith("NO_LOGIN"),
  "text_len": len(text),
  "dom_ref_count": len(data.get("dom_refs") or []),
}
(out/"login_wall.json").write_text(json.dumps(report,indent=2))
labels=[]
for line in text.splitlines():
    s=line.strip()
    if 2<=len(s)<=60:
        labels.append(s)
for r in data.get("dom_refs") or []:
    n=(r.get("name") or "").strip()
    if n and n not in labels:
        labels.append(n)
# try nested structure heuristic: keep unique
uniq=[]
for x in labels:
    if x not in uniq:
        uniq.append(x)
(out/"labels_flat.json").write_text(json.dumps({"count":len(uniq),"labels":uniq[:1000]},indent=2,ensure_ascii=False))
# placeholder hierarchy file
(out/"labels_tree.json").write_text(json.dumps({
  "note": "Hierarchical L1/L2/L3 requires logged-in labelcloud UI; flat extraction saved in labels_flat.json",
  "login_wall": report,
  "sample": uniq[:50]
},indent=2,ensure_ascii=False))
print(json.dumps(report,indent=2))
print("labels", len(uniq))
PY

vcu session stop "$SID" --json >/dev/null || true
# Open same URL in user's existing Edge for differential check (does not use WeChat; does not touch Codex CU)
osascript -e 'tell application "Microsoft Edge" to open location "https://etherscan.io/labelcloud"' 2>/dev/null || true
if [[ -f /tmp/vcu-edge-eth.pid ]]; then kill "$(cat /tmp/vcu-edge-eth.pid)" 2>/dev/null || true; fi
echo "ETHERSCAN_POC_DONE out=$OUT_DIR"
