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
python3 - <<'GATE'
import json, urllib.request
from pathlib import Path
token=json.loads((Path.home()/".vcu/config.json").read_text())["pairing_token"]
req=urllib.request.Request(
    "http://127.0.0.1:17890/v1/browser/observe",
    data=json.dumps({"pixels": False, "budget": 800}).encode(),
    method="POST",
    headers={"X-Vcu-Token": token, "Content-Type": "application/json"},
)
with urllib.request.urlopen(req, timeout=20) as r:
    Path("/tmp/vcu-observe.json").write_bytes(r.read())
d=json.loads(Path("/tmp/vcu-observe.json").read_text())
data=d.get("data") or d
tid=data.get("tab_id") or (data.get("snapshot") or {}).get("tab_id")
raise SystemExit(0 if tid else 1)
GATE
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
obs_tab=str((json.loads(Path("/tmp/vcu-observe.json").read_text()).get("data") or {}).get("tab_id") or "")
shot_tab=str(sdata.get("tab_id") or "")
if obs_tab:
    assert shot_tab==obs_tab, (shot_tab, obs_tab, sdata.get("tab_id_source"))
    assert sdata.get("tab_id_source")=="last_observe"
    print("screenshot tab_id", shot_tab, "source", sdata.get("tab_id_source"))
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
assert tab_id, "observe must stamp focused tab_id"
if tabs_source=="extension_tabs":
    assert tabs, "login-state observe must merge extension tabs when AX tree is empty"
    assert url and str(url).startswith("http"), url
elif tabs_source:
    assert tabs_source in ("ax_scene", "extension_tabs"), tabs_source
print("tab_id", tab_id, "tab_id_source", snap.get("tab_id_source") or obs.get("tab_id_source"), "tabs_source", tabs_source, "ext", obs.get("extension_profile"))
print("PASS login-state observe hud=false scale", scale, "png_bytes", png.stat().st_size, "source", sdata.get("source"))
INNER

python3 - <<'INNER'
import json, subprocess, os
from pathlib import Path
obs=json.loads(Path("/tmp/vcu-observe.json").read_text())
odata=obs.get("data") or obs
want=str(odata.get("tab_id") or (odata.get("snapshot") or {}).get("tab_id") or "")
url=str(odata.get("page_url") or (odata.get("snapshot") or {}).get("page_url") or "")
if "127.0.0.1" in url or "localhost" in url or "example.com" in url:
    subprocess.check_call(["vcu","browser","click","--selector","body","--dry-run","--json"], stdout=open("/tmp/vcu-click-obs.json","w"))
    d=json.loads(Path("/tmp/vcu-click-obs.json").read_text())
    data=d.get("data") or d
    assert data.get("source")=="extension_dom"
    assert data.get("dry_run") is True
    assert data.get("pressed") is False
    got=str(data.get("tab_id") or "")
    assert got==want, (got, want, data.get("tab_id_source"))
    assert data.get("tab_id_source")=="last_observe"
    print("PASS login-state click selector bound to observe tab", got, "source", data.get("tab_id_source"))
else:
    print("SKIP login-state click on user page", url, "tab", want)
INNER

vcu browser open --url https://example.com/ --background --json > /tmp/vcu-open-obs.json
python3 - <<'INNER'
import json, subprocess
from pathlib import Path
obs=json.loads(Path("/tmp/vcu-observe.json").read_text())
odata=obs.get("data") or obs
want_browser=(odata.get("login") or {}).get("name") or ""
d=json.loads(Path("/tmp/vcu-open-obs.json").read_text())
data=d.get("data") or d
assert d.get("ok") is True or data.get("ok") is True, d
opened=None
for t in data.get("tabs") or []:
    if str(t.get("url") or "").startswith("https://example.com"):
        opened=t
        break
tab_id=str((opened or {}).get("tab_id") or data.get("tab_id") or "")
assert tab_id, data
browser=str((opened or {}).get("browser") or data.get("browser") or "")
if "Chrome" in want_browser:
    assert browser.lower()=="chrome" or not browser, (browser, want_browser)
if "Edge" in want_browser:
    assert browser.lower()=="edge" or not browser, (browser, want_browser)
print("PASS login-state open bound to observe browser", browser or want_browser, "tab", tab_id)
subprocess.check_call(["vcu","browser","close","--tab",tab_id])
print("PASS login-state closed throwaway tab", tab_id)
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
vcu browser type --dry-run > /tmp/vcu-type.json || true
python3 - <<'INNER'
import json
from pathlib import Path
d=json.loads(Path("/tmp/vcu-type.json").read_text())
data=d.get("data") or d
if d.get("ok") is False:
    print("SKIP login-state type", (d.get("error") or {}).get("message"))
else:
    assert data.get("hud") is False
    assert data.get("typed") is False
    assert data.get("field_name") or data.get("ref")
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
fr=meta.get("webview_screenshot_frame")
sc=meta.get("webview_screenshot_scale")
if not fr or sc is None:
    print("SKIP login-state center pixel; lens observe has no AX webview frame")
else:
    sc=float(sc)
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
