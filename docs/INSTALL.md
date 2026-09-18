# Install (no npm / no source tree required)

Repository: https://github.com/zhouhanker/versatile-computer-use

## macOS / Linux — curl

```bash
curl -fsSL https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh | sh
```

Installs to `~/.local/bin` (`vcu`, `vcu-daemon`, `vcu-mcp`, and on macOS `vcu-stage`) and bundles the browser extension under `~/.local/share/vcu/extension`. Windows omits `vcu-stage`.

### Local mirror (dev)

```bash
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist bash scripts/install/install.sh
```

## Windows — irm

```powershell
irm https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.ps1 | iex
```

## Release artifacts (CI)

| Asset | Platform |
| --- | --- |
| `vcu-*-macos-arm64.tar.gz` | Apple Silicon |
| `vcu-*-macos-x64.tar.gz` | Intel Mac |
| `vcu-*-windows-x64.tar.gz` | Windows x64 |
| `install.sh` / `install.ps1` | installers |

Built by `.github/workflows/ci.yml` (artifacts) and `.github/workflows/release.yml` (GitHub Release on tag `v*`).

## After install

```bash
export PATH="$HOME/.local/bin:$PATH"
vcu init
vcu daemon start --foreground
# macOS login service:
vcu service install
vcu mcp print-config --json
```

## Verify

```bash
bash scripts/poc_install_curl.sh
cargo test --workspace
```

## Update

```bash
vcu self update
# or versioned:
vcu self update --version 0.1.0
```

## Uninstall

```bash
# remove binaries + share (keeps ~/.vcu config)
vcu self uninstall --yes

# also delete config
vcu self uninstall --yes --purge-config
```

**Never** removes Codex Computer Use or other third-party computer-use apps.
