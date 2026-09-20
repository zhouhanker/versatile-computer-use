# VCU 会话交接

更新：2026-09-20。CU-D-440 真机已过。

## 本轮

- 440：`POST /v1/browser/observe` 统一 CLI 与 MCP；遍历全部 `user_browsers`；标 focused `tab_id`（`tab_id_source=extension_tabs`）。真机 tabs=6 tab_id 已出。`poc_login_state.sh` PASS；`make check` 0。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
