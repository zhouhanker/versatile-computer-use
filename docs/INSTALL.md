# Install (no npm / no source tree required)

## macOS / Linux — curl

```bash
# After you host dist/ + install.sh on a release URL:
curl -fsSL https://<host>/install.sh | sh

# Local mirror (pack first: bash scripts/pack-release.sh)
curl -fsSL file:///path/to/repo/scripts/install/install.sh | VCU_BASE_URL=file:///path/to/repo/dist sh
# or simply:
VCU_BASE_URL=file:///path/to/repo/dist VCU_PREFIX=~/.local bash scripts/install/install.sh
```

Installs prebuilt binaries to `~/.local/bin` and extension under `~/.local/share/vcu/extension`.

## Windows — irm

```powershell
irm https://<host>/install.ps1 | iex
# local:
$env:VCU_BASE_URL = 'file:///C:/path/to/dist'
# then run scripts/install/install.ps1
```

## Maintainers: pack binaries

```bash
bash scripts/pack-release.sh
# dist/vcu-latest-macos-arm64.tar.gz + .sha256
# publish install.sh / install.ps1 beside archives
```

## Verify installer

```bash
bash scripts/poc_install_curl.sh
```

## macOS daemon at login

```bash
bash scripts/macos/install-launch-agent.sh
```

## MCP (agent integration)

See `docs/design/05-agent-integration.md`.

```bash
vcu init
vcu daemon start --foreground
vcu mcp print-config --json
```
