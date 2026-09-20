# VCU 会话交接

更新：2026-09-20。CU-D-490 已过。

## 本轮

- 490：`vcu session` desktop.scene 对 Chrome/Edge 附 `browser_tabs`（不把 AX 写成 extension_dom）。select/close/group 按 last observe 定向，open 后能关上抛页。`desktop_scene_attaches_extension_tabs_for_edge`；`poc_login_state.sh` PASS；`make check` 0。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
