# App Computer Use

状态：**macOS 最小可用 POC 已落地**；Windows UIA 待 CI/实机。

## 已实现（macOS）

| 能力 | 行为 |
| --- | --- |
| `vcu app windows` | 通过 System Events 列出非后台进程（allowlist 过滤） |
| `vcu app snapshot <id>` | 尝试 AX entire contents；无辅助功能权限时软降级为 process 摘要 |
| `vcu app focus` | 默认 `FocusPolicyViolation`（禁止抢焦点） |
| `vcu app invoke` | 默认 `OsCursorDenied`（禁止 OS 级触发） |

Allowlist 默认：TextEdit, Notes, Safari, Terminal, Ghostty, Finder。

## 测试

```sh
cargo test -p vcu-server app::
bash scripts/poc_app_macos.sh
```

## Windows

- 源码在 `cfg(target_os = "windows")` 下返回 `UnsupportedAppBackend`
- `.github/workflows/ci.yml` 的 `windows-latest` job 编译/测试核心与 release 二进制
- UIA adapter 后续在 Windows runner 上实现

## 安全不变量

1. 默认不抢焦点  
2. 默认不 OS 光标/SendInput  
3. allowlist  
4. 权限缺失时明确 repair（辅助功能设置）
