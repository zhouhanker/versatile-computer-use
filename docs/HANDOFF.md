# VCU 会话交接

更新：2026-09-20。CU-D-610 真机已过。

## 本轮

- 610：`observe --tab` 后，无 `--tab` 的 live `type --selector` / `scroll` 绑 `tab_id_source=last_observe`。抛页输入 `vcu-d-610`，scrollY 0→900。CLI `--pixels false` 可关 PNG（遮挡时 capture 可能 image readback failed）。`scripts/poc_cu_d_610.py` CU-D-610 OK。组 1/3 未动。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 无 `--tab` 的 observe 仅当前台是 USER Chrome/Edge；否则必须 `--tab`
- type/scroll 无 `--tab` 时绑 60s last observe

## 下一刀

CU-D-620 候选：同一 observe 环 live hover，或 wait 到抛页 DOM 值。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
