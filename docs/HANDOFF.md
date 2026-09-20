# VCU 会话交接

更新：2026-09-20。CU-D-430 真机已过。

## 本轮

- 430：login-state observe 在 Chrome/Edge AX tabs 为空时合并 `extension_tabs`；真机 `tabs=5` `tabs_source=extension_tabs` `page_url_source=extension_tabs`。不写 `source=extension_dom`，不宣称 AX 网页像素点击。`poc_login_state.sh` PASS；`make check` 0。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
