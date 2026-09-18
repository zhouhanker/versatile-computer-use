#!/usr/bin/env bash
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
bash "$ROOT/scripts/pack-release.sh"
PREFIX="$ROOT/.local/curl-install-prefix"
rm -rf "$PREFIX"
mkdir -p "$PREFIX"
export VCU_BASE_URL="file://$ROOT/dist"
export VCU_PREFIX="$PREFIX"
bash "$ROOT/scripts/install/install.sh"
test -x "$PREFIX/bin/vcu"
test -x "$PREFIX/bin/vcu-daemon"
test -x "$PREFIX/bin/vcu-mcp"
if [[ "$(uname -s)" == "Darwin" ]]; then
  test -x "$PREFIX/bin/vcu-stage"
fi
"$PREFIX/bin/vcu" --version | grep -q vcu
export PATH="$PREFIX/bin:$PATH"
USER_DIR="$PREFIX/vcu-home"
mkdir -p "$USER_DIR"
vcu --user-dir "$USER_DIR" init --json >/dev/null
python3 - "$USER_DIR" <<'PY'
import json,sys,time
from pathlib import Path
p=Path(sys.argv[1])/"config.json"
c=json.loads(p.read_text()); c["daemon_port"]=18600+(int(time.time())%200)
p.write_text(json.dumps(c,indent=2))
PY
vcu-daemon --user-dir "$USER_DIR" >/tmp/vcu-curl-daemon.log 2>&1 &
DPID=$!
trap 'kill $DPID 2>/dev/null || true' EXIT
for i in $(seq 1 50); do vcu --user-dir "$USER_DIR" daemon status >/dev/null 2>&1 && break; sleep 0.1; done
SID=$(vcu --user-dir "$USER_DIR" session start --backend mock --json | jq -r .data.session_id)
vcu --user-dir "$USER_DIR" navigate --session "$SID" --url https://example.com --json | jq -e .ok >/dev/null
vcu --user-dir "$USER_DIR" snapshot --session "$SID" --mode a11y --json | jq -e .ok >/dev/null
# MCP initialize via installed binary
python3 - "$USER_DIR" "$PREFIX/bin/vcu-mcp" <<'PY'
import json,sys,subprocess,re
user, binpath = sys.argv[1], sys.argv[2]
p=subprocess.Popen([binpath,"--user-dir",user], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
def send(obj):
    body=json.dumps(obj).encode()
    p.stdin.write(f"Content-Length: {len(body)}\r\n\r\n".encode()+body); p.stdin.flush()
def read():
    headers=b""
    while True:
        line=b""
        while not line.endswith(b"\n"):
            b=p.stdout.read(1)
            assert b
            line+=b
        if line in (b"\r\n", b"\n"):
            break
        headers+=line
    m=re.search(br"Content-Length:\s*(\d+)", headers, re.I)
    n=int(m.group(1))
    return json.loads(p.stdout.read(n))
send({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"t","version":"0"}}})
r=read()
assert r["result"]["serverInfo"]["name"]=="vcu-mcp", r
send({"jsonrpc":"2.0","id":2,"method":"tools/list"})
r=read()
names=[t["name"] for t in r["result"]["tools"]]
assert "vcu_session_start" in names
p.kill()
print("mcp-ok")
PY
echo "CURL INSTALL POC PASSED"
