# Release & CI packaging

Repo: https://github.com/zhouhanker/versatile-computer-use

## Packages (minimum)

- **macos-arm64**
- **macos-x64** (cross-built on macos-14)
- **windows-x64**

## Workflows

| File | When | Output |
| --- | --- | --- |
| `ci.yml` | push/PR | tests + upload-artifact packages |
| `release.yml` | tag `v0.1.0` etc. | GitHub Release + install.sh/ps1 |

## Publish a release

```bash
git tag v0.1.0
git push origin v0.1.0
# release.yml attaches macOS arm64/x64 + Windows x64 tarballs
```

## Local

```bash
bash scripts/pack-release.sh
```

## `vcu self update` 依赖

`vcu self update` 与 `curl | sh` 都指向 `releases/latest/download/`，**没有 Release 资产时两者都不可用**（当前仓库即为此状态，跟踪项 CU-D-700 / 台账 `MAC-NEXT`）。发布后自查：

```bash
gh release list                                     # 应有 v* 资产
curl -fsI https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh
vcu self update                                     # 不带 VCU_BASE_URL 应成功
```

无 Release 期间用本地通路：`bash scripts/pack-release.sh && VCU_BASE_URL=file://$PWD/dist vcu self update`。
