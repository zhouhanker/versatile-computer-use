# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机仍未跑。

## 本轮

- CU-D-060 补了 `uia_tree_script` / `uia_invoke_script` / `parse_uia_element_lines`。Invoke 在 Windows 上走 InvokePattern，非 HID。单测锁脚本不含 SendInput。
- 截图仍未实现。真机 UIA 需要 Windows 主机。

## 下一刀

Windows 主机：Notepad 列树 + InvokePattern。或接 PrintWindow 截图。不要在 macOS 上宣称 Windows CU 已可用。
