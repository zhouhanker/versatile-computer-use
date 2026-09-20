# VCU 会话交接

更新：2026-09-20。CU-D-620 真机已过。

## 本轮

- 620：`observe --tab` 后 live `hover --selector` 绑 last observe（hovered=1）。新增 DOM `wait --selector [--text]`，等到 extract 命中；miss 诚实超时。`scripts/poc_cu_d_620.py` CU-D-620 OK。组 1/3 未动。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 无 `--tab` 的 observe 仅当前台是 USER Chrome/Edge；否则必须 `--tab`
- click/type/scroll/hover/extract/wait(selector) 无 `--tab` 时绑 60s last observe

## 下一刀

CU-D-630 候选：wait 文本尚未出现时的负向真机（已有 HTTP miss）；或双浏览器 `observe --tab` 定向 Chrome vs Edge。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
