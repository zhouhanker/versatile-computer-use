# VCU 静态资源

只放 **VCU 自有**、要进 git / 随安装包发布的素材。不要提交：

- `~/.codex/computer-use/` 或其它产品安装目录里的文件
- `~/.vcu/captures/`、`.local/` 里的运行时截图与 POC 证据

## 目录

| 路径 | 用途 | 谁用 |
| --- | --- | --- |
| `assets/cursor/` | 虚拟指针（SVG/PNG，短三角+柔光） | 扩展 content、Guide |
| `assets/hud/` | Stage 胶囊 HUD 图/色板 | `vcu-stage` |
| `assets/extension/` | 扩展图标（16/32/48/128） | `extension/manifest.json` |

网页光标若继续用内联 SVG，也可把源文件放在 `assets/cursor/` 再同步进 `extension/content.js`。

## 不要放这里

| 路径 | 用途 |
| --- | --- |
| `extension/` | 扩展代码；图标从 `assets/extension/` 拷贝或引用 |
| `helpers/vcu-stage/` | Swift 源码；打包时再把 `assets/hud`、`assets/cursor` 编进 bundle |
| `.local/` | 本机对照证据，不进仓库 |
| `~/.vcu/captures/` | daemon 运行时截图 |

源文件：`assets/extension/icons01-vcu-lens.png`。扩展实际加载的是 `extension/icons/icon-{16,32,48,128}.png`（由源图缩放）。改源图后需重新生成这些尺寸并 `vcu browser install-lens --reload`。
