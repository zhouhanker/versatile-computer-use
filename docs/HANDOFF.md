# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机仍未跑。

## 本轮

- CU-D-060：`uia_set_value_script`（ValuePattern，无 SendInput）。Windows 上 set_value 走 UIA；非 Windows 仍 NotImplemented。
- `scripts/poc_desktop_windows.sh` 在 Darwin 上 SKIP（exit 0）。

## 下一刀

Windows 主机跑 Notepad：list / UIA 树 / InvokePattern / ValuePattern / PrintWindow。不要在 macOS 上宣称 Windows CU 已可用。
