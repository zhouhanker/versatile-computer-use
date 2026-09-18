#!/usr/bin/env bash
# Feishu Scene only. Never sends. No Stage HUD.
set -euo pipefail
export PATH="$HOME/.local/bin:$PATH"
id=$(vcu app windows --json | python3 -c 'import json,sys
d=json.load(sys.stdin)
wins=(d.get("data") or {}).get("windows") or []
for w in wins:
    if w.get("title")=="Feishu" or "Feishu" in str(w.get("bundle_or_exe")):
        print(w["id"]); break
else:
    raise SystemExit("SKIP: no Feishu window")
')
echo "feishu $id"
vcu app snapshot "$id" --pixels --selector messenger --json > /tmp/vcu-feishu-scene.json
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-feishu-scene.json").read_text())
data=d.get("data") or {}
assert data.get("webview") is True
ext=data.get("extract") or {}
assert ext.get("hud") is False
assert ext.get("count",0)>=1
png=Path.home()/".vcu/captures/feishu-latest.png"
meta=Path.home()/".vcu/captures/feishu-latest.json"
assert png.exists() and png.stat().st_size>1000, png
j=json.loads(meta.read_text())
assert j.get("screenshot_scale") in (1.0,2.0,3.0)
els=data.get("elements") or []
sendish=[e for e in els if "发送" in str(e.get("name") or "")]
assert not sendish, f"AX must not expose Send; got {sendish}"
print("PASS feishu scene", data.get("webview_ref"), "hits", ext.get("count"),
      "scale", j.get("screenshot_scale"), "png", png.stat().st_size, "hud=false ax_no_send")
INNER
