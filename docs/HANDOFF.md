# VCU 会话交接

更新：2026-09-20。CU-D-060 CI 列窗+截帧已过。CU-D-070 脚本已加，待 CI。

## 本轮

- 060 证据：CI run `35462329205` @ `aa87390`：`UIA_OK` + `PRINTWINDOW_OK`。
- 070：`poc_desktop_windows.ps1` 对 Notepad 可写 ValuePattern 写入 `VCU-D-070` 并读回。无 SendInput。

## 下一刀

看 Windows CI 是否打印 `SETVALUE_OK`。未绿不得宣称 070 完成，也不得写成 Windows 产品 CU。
