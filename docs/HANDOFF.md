# VCU 会话交接

更新：2026-09-20。CU-D-670 真机已过。

## 本轮

- 670：`open --browser chrome|edge` 覆盖 last observe。group/ungroup 解析 tab_ids，跨 Chrome/Edge 拒绝；撞号须 `--browser`。真机 Chrome 两抛页 group/ungroup，Edge 另开一页。组 1/3 未动。`scripts/poc_cu_d_670.py` CU-D-670 OK。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 原生标签组只属于一个浏览器窗口

## 下一刀

CU-D-680 候选：group-update `--browser`；或 MCP 文案与 CLI help 对齐全部 --browser。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
