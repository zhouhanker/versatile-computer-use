# VCU 会话交接

更新：2026-09-20。CU-D-660 真机已过。

## 本轮

- 660：hover/scroll/wait/screenshot 接受 `--browser`。真机 Chrome hover+scroll；Edge wait `#who=edge`；hover 错浏览器拒绝。viewport screenshot 在窗口被挡时可能 image readback failed，不作为本切片必过项。`scripts/poc_cu_d_660.py` CU-D-660 OK。组 1/3 未动。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- tab_id 撞号时 observe/close/select 与 DOM 动作须 `--browser`

## 下一刀

CU-D-670 候选：group/ungroup 按浏览器隔离；或 open `--browser`。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
