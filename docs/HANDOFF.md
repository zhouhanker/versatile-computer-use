# VCU 会话交接

更新：2026-09-20。CU-D-240 Windows Abort 拆 HUD 进行中。

## 本轮

- 240：`session abort` 必须 `hud=false`；list 无 sid；后续 snapshot `SessionNotFound`；`vcu-stage-*.ps1` teardown。无 SendInput。

## 下一刀

看 CI `ABORT_OK` / `HUD_GONE` / `CU-D-240 OK`。未绿不得宣称 240 完成。不要 claim MAC-NEXT / FEISHU-001。
