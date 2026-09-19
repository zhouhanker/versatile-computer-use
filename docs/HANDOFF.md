# VCU 会话交接

更新：2026-09-20。CU-D-140 脚本已加，待 CI。

## 本轮

- 控制台 type 含换行 → FocusPolicyViolation。
- 无 ValuePattern 时 clipboard + WM_PASTE（非 SendInput）。

## 下一刀

看 CI `TYPE_OK` + `NEWLINE_DENIED`。未绿不得宣称 140 完成。禁止执行 cmd 命令。
