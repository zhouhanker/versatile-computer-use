# VCU 会话交接

更新：2026-09-20。CU-D-460 真机已过。

## 本轮

- 460：DOM 动作按 last observe 的 Chrome/Edge 定向 lens client；tab 未在 client 登记时也不打到另一套浏览器。`browser_hint_targets_client_when_tab_unknown`。daemon 重启后 poc selector click 仍 PASS。`make check` 0。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
