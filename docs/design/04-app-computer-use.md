# App / 桌面 Computer Use

状态：macOS **最小 POC 已有**（列窗、AX 软降级）。  
**主路径设计**见 `docs/design/06-stage-steward.md`（Stage + Steward）。确认前不把 POC 扩成 overlay 产品。

## 设计主路径（未实现）

| 能力 | 行为 |
| --- | --- |
| Steward | `vcu-daemon` 长驻，一次辅助功能 |
| Scene | AX 树 refs + 窗口截帧 |
| Actuator | AXPress / AXSetValue / key；不 warp 用户鼠标 |
| Stage | Banner「VCU 正在使用这台 Mac」+ Guide 虚拟指针 |
| Abort | 热键结束会话并拆 Stage |

## 已实现（POC，将降为 desktop 的探测层）

| 能力 | 行为 |
| --- | --- |
| `vcu app windows` | System Events 列窗（allowlist） |
| `vcu app snapshot <id>` | AX entire contents；无权限时软降级 |
| `vcu app focus` | 默认 `FocusPolicyViolation` |
| `vcu app invoke` | 默认 `OsCursorDenied` |

Allowlist 默认：Edge, Safari, Feishu/Lark, TextEdit, Notes, Terminal, Ghostty, Finder。  
Denylist：**微信 / WeChat**。

## 测试

```sh
cargo test -p vcu-server app::
bash scripts/poc_app_macos.sh
```

## 安全不变量

1. 不把用户物理光标当主路径（Guide ≠ OS cursor）
2. 会话必须可见 Stage
3. allowlist + 微信拒绝
4. 缺辅助功能时 repair 指向系统设置，而不是 Edge Allow
5. 不碰 `~/.codex/computer-use/`
