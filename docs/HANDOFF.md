# VCU 会话交接

更新：2026-09-20。桌面史诗进行中。

## 本轮

- CU-D-040 **真机通过**（诚实路径）：`act type=open_path` / `reveal` 走 NSWorkspace；Scene 列出 `VCU-D-040-*` 与 `OPENME`；`os_cursor_used=false`。不是 AXPress，不是 Return。
- 不要 `tell application "Finder"`。不要 HID。

## 下一刀

CU-D-041 Terminal / Ghostty：聚焦、输入、**禁盲目 Return**。只动脚本新开的 Terminal 窗，不要往用户正在用的 Ghostty/agent 终端打字。
