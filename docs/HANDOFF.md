# VCU 会话交接

更新：2026-09-20。CU-D-210 CI 已绿；CU-D-220 Notepad key return 拒绝进行中。

## 本轮

- 210：run `35472851832` `WAIT_MISS_OK` / `WAIT_REF_MISS_OK` / `CU-D-210 OK`。
- 220：Notepad `act type=key return` 无 confirm_send 必须 FocusPolicyViolation。待 CI `KEY_DENIED`。

## 下一刀

看 CI `KEY_DENIED` / `CU-D-220 OK`。未绿不得宣称 220 完成。不要 claim MAC-NEXT / FEISHU-001。
