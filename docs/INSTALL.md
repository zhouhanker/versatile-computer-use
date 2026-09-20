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

`vcu self update` 走 `releases/latest/download/install.sh`（`VCU_BASE_URL` 可覆盖）。**仓库还没发布 Release 资产时它会失败**，只报 `update installer exited non-zero`（不透传 installer stderr）。

### Local mirror（无 Release 时）

```bash
bash scripts/pack-release.sh                       # 产出 dist/vcu-latest-*.tar.gz + .sha256
VCU_BASE_URL=file://$PWD/dist vcu self update      # 等价于 curl|sh 的本地通路
```

更新后重启 daemon 才会用上新二进制：`vcu daemon stop && vcu daemon start`。

注意 `vcu --version` 打印的是 crate 版本（当前 `0.1.0`），浏览器桥版本另算（当前 `0.2.8`），所以版本号不变不代表没更新。

## Uninstall

```bash
# remove binaries + share (keeps ~/.vcu config)
vcu self uninstall --yes

# also delete config
vcu self uninstall --yes --purge-config
```

**Never** removes Codex Computer Use or other third-party computer-use apps.
