#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="$ROOT/target/debug:$HOME/.local/bin:$PATH"
if ! command -v vcu >/dev/null; then
  echo "vcu not on PATH" >&2
  exit 1
fi
vcu browser login-state > /tmp/vcu-login-state.json
python3 - <<'INNER'
import json, sys
from pathlib import Path
d=json.loads(Path("/tmp/vcu-login-state.json").read_text())
data=d.get("data") or d
users=data.get("user_browsers") or []
if not users:
    print("SKIP: no user Chrome/Edge")
    raise SystemExit(0)
print("user_pid", users[0].get("pid"), "ext", data.get("extension_profile"))
print("infobar", data.get("automation_infobar"), "allow_dialog", data.get("allow_dialog_visible"))
print("next_action", data.get("next_action"))
na = str(data.get("next_action") or "")
assert "click Allow debugging" not in na
assert "User: click Allow" not in na
assert data.get("never_click_allow") is True
assert data.get("never_os_cursor") is True
assert data.get("never_wechat") is True
INNER
vcu browser observe --selector "*" --json > /tmp/vcu-observe.json
vcu browser screenshot --json > /tmp/vcu-observe-shot.json
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-observe.json").read_text())
data=d.get("data") or d
assert data.get("hud") is False
assert data.get("login_state") is True
shot=json.loads(Path("/tmp/vcu-observe-shot.json").read_text())
sdata=shot.get("data") or shot
assert shot.get("ok") is True or sdata.get("ok") is True
assert sdata.get("source") == "extension_viewport" or sdata.get("screenshot_path")
vh=(data.get("snapshot") or {}).get("vision_handoff") or data.get("vision_handoff") or sdata.get("vision_handoff") or {}
must=vh.get("must_view") or []
png_path=sdata.get("screenshot_path")
if not must and png_path:
    must=[png_path]
assert must, "observe/screenshot must provide a PNG"
assert any(str(p).endswith(".png") for p in must)
png=Path.home()/".vcu/captures/login-latest.png"
meta=Path.home()/".vcu/captures/login-latest.json"
if not (png.exists() and png.stat().st_size>1000):
    png=Path(str(must[0]))
assert png.exists() and png.stat().st_size>1000
scale=None
if meta.exists():
    j=json.loads(meta.read_text())
    scale=j.get("screenshot_scale")
if scale is None:
    scale=sdata.get("screenshot_scale") or 1.0
obs=data
snap=obs.get("snapshot") or {}
url=obs.get("page_url") or snap.get("page_url") or sdata.get("page_url")
tabs=obs.get("tabs") or snap.get("tabs") or []
tabs_source=snap.get("tabs_source") or obs.get("tabs_source")
print("page_url", url, "tabs", len(tabs), "tabs_source", tabs_source, "page_url_source", snap.get("page_url_source"))
tab_id=obs.get("tab_id") or snap.get("tab_id")
if tabs_source=="extension_tabs" or obs.get("extension_profile")=="user":
    assert tabs_source=="extension_tabs", tabs_source
    assert tabs, "login-state observe must merge extension tabs when AX tree is empty"
    assert url and str(url).startswith("http"), url
    assert tab_id, "observe must stamp focused extension tab_id"
    print("tab_id", tab_id, "tab_id_source", snap.get("tab_id_source") or obs.get("tab_id_source"), "ext", obs.get("extension_profile"))
print("PASS login-state observe hud=false scale", scale, "png_bytes", png.stat().st_size, "source", sdata.get("source"))
INNER

vcu browser click --pixel-x 0 --pixel-y 0 --space webview --dry-run --guide > /tmp/vcu-click-map.json
python3 - <<'INNER'
import json, subprocess
from pathlib import Path
d=json.loads(Path("/tmp/vcu-click-map.json").read_text())
data=d.get("data") or d
assert data.get("hud") is False
assert data.get("dry_run") is True
assert data.get("pressed") is False
ax=data.get("ax_point") or {}
assert "x" in ax and "y" in ax
g=data.get("guide") or {}
assert g.get("overlay") is True
assert g.get("hud") is False
left=subprocess.run(["pgrep","-x","vcu-stage"], capture_output=True)
assert left.returncode != 0, "vcu-stage leftover after guide flash"
print("PASS login-state click dry-run+guide ax", ax, "hit", data.get("hit_ref"), "guide", g.get("x"), g.get("y"))
INNER
vcu browser type --dry-run > /tmp/vcu-type.json
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-type.json").read_text())
data=d.get("data") or d
assert data.get("hud") is False
assert data.get("typed") is False
assert data.get("field_name") or data.get("ref")
assert data.get("typed") is False
meta=json.loads((Path.home()/".vcu/captures/login-latest.json").read_text())
url=data.get("field_value") or ""
if str(url).startswith("http"):
    assert data.get("login_latest_url_merged") is True
    assert meta.get("page_url", "").startswith("http")
print("PASS login-state type dry-run field", data.get("field_name") or data.get("ref"), "sidecar", meta.get("page_url"))
INNER
vcu browser scroll --dry-run --dy 600 > /tmp/vcu-scroll.json
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-scroll.json").read_text())
data=d.get("data") or d
assert data.get("hud") is False
assert data.get("scrolled") is False
print("PASS login-state scroll dry-run")
INNER

vcu browser wait --role AXWebArea --ms 3000 > /tmp/vcu-wait.json
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-wait.json").read_text())
data=d.get("data") or d
assert data.get("hud") is False
assert data.get("found_ref")
print("PASS login-state wait AXWebArea", data.get("found_ref"), "ms", data.get("waited_ms"), "fast", data.get("fast"))
INNER
python3 - <<'INNER'
import json, subprocess, os
from pathlib import Path
meta=json.loads((Path.home()/".vcu/captures/login-latest.json").read_text())
fr=meta["webview_screenshot_frame"]
sc=float(meta["webview_screenshot_scale"])
px, py = fr[2]*sc/2, fr[3]*sc/2
exp_x, exp_y = fr[0]+fr[2]/2, fr[1]+fr[3]/2
out=subprocess.check_output(["vcu","browser","click","--pixel-x",str(px),"--pixel-y",str(py),"--space","webview","--dry-run"], text=True)
d=json.loads(out)
data=d.get("data") or d
ax=data.get("ax_point") or {}
assert data.get("hud") is False
assert abs(ax["x"]-exp_x)<0.6 and abs(ax["y"]-exp_y)<0.6
print("PASS login-state center pixel ax", ax, "expected", exp_x, exp_y)
INNER

vcu browser key --key return --dry-run > /tmp/vcu-key.json
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-key.json").read_text())
data=d.get("data") or d
assert data.get("pressed") is False
assert data.get("hid_injected") is False
assert data.get("blocked") is True
print("PASS login-state key return dry-run blocked", data.get("code"))
INNER
