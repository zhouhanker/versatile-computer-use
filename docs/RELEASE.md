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
