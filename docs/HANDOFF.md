# VCU 会话交接

更新：2026-09-20。CU-D-600 真机已过。

## 本轮

- 600：无 `--tab` 的 `observe` 在前台不是 USER Chrome/Edge 时返回 `InvalidInput`（`frontmost is not USER Chrome/Edge; pass observe --tab`），不静默绑到另一浏览器的 active tab。`--tab` 路径不变。真机前台 WeChat 时 default observe 失败、`observe --tab` 抛页成功。`scripts/poc_cu_d_600.py` CU-D-600 OK。组 1/3 未动。无 SendInput。未操作微信。`make check` 0。

## 测试边界

- 允许：抛页 `127.0.0.1`、dry-run、自己创建的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 无 `--tab` 的 observe 只绑前台 Chrome/Edge 的 focused tab；否则必须 `--tab`

## 下一刀

CU-D-610 候选：同一 observe+capture 环上的 live type / scroll（仅抛页）。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
