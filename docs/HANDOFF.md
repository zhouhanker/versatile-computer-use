# VCU 会话交接

更新：2026-09-20。CU-D-140 CI 红因 empty cmd scene（`MainWindowHandle=0` / `raw=MISSING`）。已改 HWND 解析与控制台 paste，待新 CI。

## 本轮

- 失败 run `35468406114` job `windows vcu desktop cmd type (CU-D-140)`：`elements=0 note=uia_tree raw=MISSING`。
- cmd 窗口在 conhost 上；`Get-Process.MainWindowHandle` 为 0 不能当 HWND。
- 现用 EnumWindows / AttachConsole+GetConsoleWindow；控制台 type 走 clipboard+WM_PASTE，不把 WM_SETTEXT 当成功。
- POC：conhost 拉起 `cmd /k title VCU-D-140`，snapshot 重试。无 SendInput。

## 下一刀

看新 CI 的 `SNAP_OK` + `TYPE_OK` + `NEWLINE_DENIED`。未绿不得宣称 140 完成。禁止执行 cmd 命令。
