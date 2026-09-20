# VCU 会话交接

更新：2026-09-20。CU-D-640 真机已过。

## 本轮

- 640：`close`/`select` 解析 merged tabs，`--browser chrome|edge` 定向 lens；撞号无 `--browser` 为 InvalidInput。真机 close 错浏览器 `tab not found`，`--browser chrome` 只关 Chrome 抛页，Edge 抛页仍在。未跑 live select（会抢 OS 窗口焦点）。`scripts/poc_cu_d_640.py` CU-D-640 OK。组 1/3 未动。无 SendInput。`make check` 0。

## 测试边界

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- live `select` 会 `windows.update(focused)`，本切片不真机 select
- tab_id 撞号时 observe/close/select 须 `--browser`

## 下一刀

CU-D-650 候选：extract/type/click 同样接受 `--browser`；或 group 命令按窗口/浏览器隔离。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
