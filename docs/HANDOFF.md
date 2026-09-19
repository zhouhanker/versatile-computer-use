# VCU 会话交接

更新：2026-09-20。CU-D-280 MCP desktop hover/wait/abort 进行中。

## 本轮

- 280：宿主 MCP 必须有 `vcu_hover`；`vcu_wait` 转发 `args.value`；`vcu_session_abort` 走 abort 不是 stop。

## 下一刀

看 CI `TOOLS_OK` / `CU-D-280 OK`。未绿不得宣称 280 完成。不要 claim MAC-NEXT / FEISHU-001。
