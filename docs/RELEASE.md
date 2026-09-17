# Release & CI packaging

Repository: https://github.com/zhouhanker/versatile-computer-use

## Automated packages (GitHub Actions)

| Workflow | Trigger | Artifacts |
| --- | --- | --- |
| `ci.yml` | push/PR | test + **package** job uploads `vcu-macos-arm64`, `vcu-macos-x64`, `vcu-windows-x64` |
| `release.yml` | tag `v*` or manual | GitHub Release assets + `install.sh` / `install.ps1` |

### Platforms (minimum)

- **macos-arm64** (macos-14 runner)
- **macos-x64** (macos-13 runner)
- **windows-x64** (windows-latest)

### User install after release

```bash
curl -fsSL https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh | sh
```

```powershell
irm https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.ps1 | iex
```

### Local pack

```bash
bash scripts/pack-release.sh
```
