# VCU 会话交接

更新：2026-09-20。CU-D-650 真机已过。

## 本轮

- 650：extract/type/click 接受 `--browser`；显式 `--tab` 撞号须带 `--browser`。observe `--browser chrome` 不再把 last_observe `app_id` 回落到 Edge 进程。真机 Chrome `#who=chrome` + type；Edge `#who=edge` + click dry-run。`scripts/poc_cu_d_650.py` CU-D-650 OK。组 1/3 未动。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- tab_id 撞号时 observe/close/select/extract/type/click 须 `--browser`

## 下一刀

CU-D-660 候选：hover/scroll/wait/screenshot 同样 `--browser`；或 group 按浏览器隔离。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
