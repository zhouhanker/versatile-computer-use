#!/usr/bin/env bash
# Login-state Etherscan L1/L2/L3 via USER Edge extension extract.
# CDP / Allow / Agent Edge are abandoned this version.
set -euo pipefail
command -v vcu >/dev/null || export PATH="$HOME/.local/bin:$PATH"
OUT_DIR="${VCU_ETHERSCAN_OUT:-$HOME/vcu-etherscan-labels}"
mkdir -p "$OUT_DIR"

vcu browser login-state --json | tee "$OUT_DIR/login-state.json" >/dev/null
python3 - <<'PY' "$OUT_DIR"
import json, subprocess, sys
from pathlib import Path
out = Path(sys.argv[1])
login = json.loads((out/"login-state.json").read_text())
data = login.get("data") or login
if data.get("extension_profile") != "user":
    print("SKIP: extension_profile is", data.get("extension_profile"), "need user")
    (out/"mode.txt").write_text("SKIP_NO_USER_EXTENSION\n")
    (out/"labels_tree.json").write_text(json.dumps({
        "blocker": "USER Edge extension_profile is not user",
        "never": ["CDP", "click Allow", "Agent Edge"],
    }, indent=2))
    raise SystemExit(0)
ping = subprocess.run(["vcu","browser","ping","--json"], capture_output=True, text=True)
(out/"ping.json").write_text(ping.stdout or ping.stderr or "")
try:
    pv = json.loads(ping.stdout)
except Exception:
    print("FAIL ping non-json")
    raise SystemExit(1)
if pv.get("ok") is not True or (pv.get("data") or {}).get("pong") is not True:
    err = pv.get("error") or {}
    print("SKIP: extension SW stale", err.get("message"))
    (out/"mode.txt").write_text("SKIP_STALE_SW\n")
    (out/"labels_tree.json").write_text(json.dumps({
        "blocker": "stale SW; Reload VCU Browser Bridge on edge://extensions",
        "error": err,
        "never": ["CDP", "click Allow", "WeChat"],
    }, indent=2, ensure_ascii=False))
    raise SystemExit(0)
typ = subprocess.run(["vcu","browser","type","--dry-run"], capture_output=True, text=True)
(out/"type.json").write_text(typ.stdout or "")
tv = json.loads(typ.stdout)
td = tv.get("data") or {}
url = str(td.get("field_value") or "")
if "etherscan.io" not in url.lower():
    print("SKIP: USER Edge is not on etherscan.io, url=", url)
    (out/"mode.txt").write_text("SKIP_NOT_ETHERSCAN\n")
    (out/"labels_tree.json").write_text(json.dumps({
        "blocker": "open https://etherscan.io/labelcloud in USER Edge then retry",
        "page_url": url,
        "never": ["CDP", "click Allow", "navigate Agent Edge"],
    }, indent=2))
    raise SystemExit(0)
ext = subprocess.run(["vcu","browser","extract","--selector","a","--json"], capture_output=True, text=True)
(out/"extract.json").write_text(ext.stdout or ext.stderr or "")
ev = json.loads(ext.stdout)
if ev.get("ok") is not True:
    print("FAIL extract", ev.get("error"))
    raise SystemExit(1)
ed = ev.get("data") or {}
if ed.get("source") != "extension_dom":
    print("FAIL extract source", ed.get("source"), "want extension_dom")
    raise SystemExit(1)
matches = ed.get("matches") or []
hrefs = []
for m in matches:
    href = (m.get("href") or "") if isinstance(m, dict) else ""
    text = (m.get("text") or m.get("name") or "") if isinstance(m, dict) else ""
    if href or text:
        hrefs.append({"text": str(text)[:160], "href": href})
tree = {
    "source": "extension_dom",
    "login_state": True,
    "hud": False,
    "page_url": url,
    "count": ed.get("count"),
    "l1": [h for h in hrefs if "labelcloud" in (h.get("href") or "").lower() or "label" in (h.get("href") or "").lower()][:80],
    "sample": hrefs[:80],
    "never": ["CDP", "click Allow", "Agent Edge", "WeChat"],
}
(out/"labels_tree.json").write_text(json.dumps(tree, indent=2, ensure_ascii=False))
(out/"mode.txt").write_text("LOGIN_STATE_EXTENSION_DOM\n")
print("PASS etherscan extract", ed.get("count"), "source", ed.get("source"))
PY
