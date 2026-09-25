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

`vcu self update` 走 `releases/latest/download/install.sh`（`VCU_BASE_URL` 可覆盖）。仓库已发布 `v0.2.8`（Latest），直接跑即可。若换成没有资产的镜像会失败，此时错误里会带上 base URL、installer 的 stderr 摘要与下面的本地通路提示。

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

## Browser extension

The runtime tarball on GitHub Releases already contains `extension/`. A separate `vcu-lens-extension.zip` is built by the release workflow for the next tag. It is not on the existing `v0.2.8` asset list.

Windows, without cloning and without clicking the debugging consent dialog:

```powershell
irm https://raw.githubusercontent.com/zhouhanker/versatile-computer-use/main/scripts/install/install-lens.ps1 | iex
```

The script tries `vcu-lens-extension.zip` first, then falls back to `vcu-latest-windows-x64.tar.gz`. It copies the extension to `%USERPROFILE%\.vcu\lens-extension`. Then in Edge: `edge://extensions`, Developer mode, Load unpacked. Do not start CDP. A checkout can still run `powershell -File scripts/install/install-lens.ps1 -FromRelease -Open`.

From a source checkout, omit `-FromRelease` to copy `extension/` directly.
