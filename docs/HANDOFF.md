# VCU 会话交接

更新：2026-09-20。CU-D-520 进行中，未宣称完成。

## 本轮

- 真机缺口：health 显示 chrome+edge polling，但 `vcu browser tabs` 经常只有 Chrome。Edge MV3 多半只 hello、不挂 poll；定向 `edge:<id>` 的 list_tabs/reload_self 会超时。Chrome 单独 6 tabs、Wake 后 Edge 单独 9 tabs 都见过，同一次 merge 双浏览器仍未绿。
- 已落地（单测绿，不是 CU-D-520 完成）：lens offscreen 在 getContexts 抛错时仍 create；offscreen 长连接；content/SW heartbeat；hello `command_pull`；poll 支持 POST；list_tabs 并行；client 存活 45s；list_tabs/reload 等 35s；无 client_id 的 Edge poll 仍可接 edge 定向命令；一侧失败时 `browsers_failed` 诚实留下另一侧。`extension_bridge` 18 测、Node background 22 测通过。`make check` 0。组 1/3 未动。无 SendInput。
- 不是 TC-B-040 / 产品 Windows CU / 完整 Codex CU。

## 下一刀

CU-D-520 真机：同一次 `/v1/browser/tabs` 要 chrome≥1 且 edge≥1。Edge SW 需要跑到新 background（POST poll / hello pull），不要再发明 last_observe 文档刀。P2 仍停放。
