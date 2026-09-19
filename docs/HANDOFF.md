# VCU 会话交接

更新：2026-09-20。CU-D-200 CI 已绿；CU-D-210 wait miss 进行中。

## 本轮

- 200：run `35472199783` `WAIT_OK path=scene_wait found_ref=e2 os_cursor_used=False` / `CU-D-200 OK`。
- 210：错 value / 缺 ref 必须 `ActionFailed` 且文案含 `timed out` 与所等 value/ref。单测已加。待 Windows CI `WAIT_MISS_OK`。

## 下一刀

看 CI `WAIT_MISS_OK` / `WAIT_REF_MISS_OK` / `CU-D-210 OK`。未绿不得宣称 210 完成。不要 claim MAC-NEXT / FEISHU-001。
