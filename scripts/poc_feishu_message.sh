#!/usr/bin/env bash
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
vcu config set-app-allowlist 'Feishu,Lark,TextEdit,Ghostty,Finder,Safari,Microsoft Edge,Google Chrome,Code' --json >/dev/null || true
pkill -f '[v]cu-daemon' 2>/dev/null || true
sleep 0.3
nohup vcu-daemon >/tmp/vcu-daemon-user.log 2>&1 &
sleep 0.6
vcu app windows --json | tee /tmp/vcu-feishu-windows.json >/dev/null
pick_id() {
python3 - <<'PY'
import json
d=json.load(open("/tmp/vcu-feishu-windows.json"))
for w in (d.get("data") or {}).get("windows") or []:
    blob=f"{w.get('title','')} {w.get('bundle_or_exe','')}"
    if any(k.lower() in blob.lower() for k in ["feishu","lark","飞书"]):
        print(w.get("id",""))
        break
PY
}
FID="$(pick_id || true)"
if [[ -z "${FID}" ]]; then
  open -a Lark 2>/dev/null || open -a Feishu 2>/dev/null || true
  sleep 2
  vcu app windows --json | tee /tmp/vcu-feishu-windows.json >/dev/null
  FID="$(pick_id || true)"
fi
echo "feishu_id=${FID:-none}"
if [[ -n "${FID}" ]]; then
  vcu app snapshot "$FID" --budget 8000 --json >/tmp/vcu-feishu-snap.json || true
fi
# Accessibility keystrokes into Feishu only (NOT WeChat / NOT Codex CU)
osascript > /tmp/vcu-feishu-osa.out 2>/tmp/vcu-feishu-osa.err <<'APPLESCRIPT' || true
tell application "Lark" to activate
delay 1.2
tell application "System Events"
  set pname to "Feishu"
  if not (exists process "Feishu") then set pname to "Lark"
  tell process pname
    set frontmost to true
    delay 0.4
    keystroke "k" using {command down}
    delay 0.8
    keystroke "张北北"
    delay 1.5
    key code 36
    delay 1.0
    keystroke "Test"
    delay 0.4
    key code 36
  end tell
end tell
return "ok"
APPLESCRIPT
echo "osa_out=$(cat /tmp/vcu-feishu-osa.out 2>/dev/null || true)"
echo "osa_err=$(cat /tmp/vcu-feishu-osa.err 2>/dev/null || true)"
echo "FEISHU_POC_DONE — please confirm 张北北 got message Test"
echo "wechat_touched=false codex_cu_touched=false"
