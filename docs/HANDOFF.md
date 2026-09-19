# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机待 CI。

## 本轮

- `poc_desktop_windows.ps1` 增加 PrintWindow PNG 魔数校验（无 SendInput）。
- Darwin 仍 SKIP。CI windows-latest 跑该脚本后才能标 060 真机。

## 下一刀

看 Windows CI 是否打印 UIA_OK 与 PRINTWINDOW_OK。未绿不得宣称 Windows CU。
