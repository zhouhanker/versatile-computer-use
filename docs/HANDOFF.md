# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机仍待 CI/主机。

## 本轮

- CU-D-060：`scripts/poc_desktop_windows.ps1` 在 Windows 上开 Notepad，用 UIAutomation 找窗（无 SendInput）。Darwin 上 sh 诚实 SKIP。
- CI `windows-latest` 增加 `windows UIA notepad smoke` 步骤。本机未跑该 ps1。

## 下一刀

等 Windows CI 或真机跑通 `poc_desktop_windows.ps1` 后再把 CU-D-060 标真机完成。不要提前宣称 Windows CU 可用。
