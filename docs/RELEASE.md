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

`vcu self update` 与 `curl | sh` 都指向 `releases/latest/download/`。**没有 Release 资产时两者都不可用**；仓库现已发布 `v0.2.8`（Latest）。发布后自查：

```bash
gh release list                                     # 应有 v* 资产
curl -fsI https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh
vcu self update                                     # 不带 VCU_BASE_URL 应成功
```

无 Release 期间用本地通路：`bash scripts/pack-release.sh && VCU_BASE_URL=file://$PWD/dist vcu self update`。

### 发布踩过的坑（保持修复）

- `files:` 不要用裸 `dist/*`：会带上 0 字节的 `dist/.gitkeep`，GitHub 拒绝 0 字节资产，publish job 会在**其它资产都传完之后**失败并把 Release 留在 draft。
- Windows job 的 checkout 会把 `install.sh` 变成 CRLF（`core.autocrlf`），publish 时覆盖掉 macOS 的 LF 版本，导致 `curl | sh` 报 `set: pipefail: invalid option name`。已用 `.gitattributes`（`*.sh text eol=lf`）+ publish 步骤 `tr -d '\r'` 双保险。
- draft 抢救：如果 publish 失败但资产已上传，可 `gh release edit <tag> --draft=false` 发布，再用 `gh release upload <tag> <file> --clobber` 替换坏资产（注意 CDN 可能缓存 `/latest/` 几分钟）。

修复后的管线已用临时 tag 端到端验证过（run `35511683467`，成功后已删除该 tag/Release）：publish job 绿、资产 14 个、无 0 字节文件、`install.sh` 为 LF 且与仓库文件 sha256 一致。
