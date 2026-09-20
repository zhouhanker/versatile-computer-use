# VCU 会话交接

更新：2026-09-20。CU-D-450 真机已过。

## 本轮

- 450：observe 后 60s 内，无 tab_id 的 DOM click/type/hover/scroll/extract 绑 last observe tab（`tab_id_source=last_observe`）。显式 tab_id 优先。真机 `body` dry-run 与 observe tab 一致。`poc_login_state.sh` PASS；`make check` 0。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
