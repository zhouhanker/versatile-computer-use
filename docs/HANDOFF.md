# VCU 会话交接

更新：2026-09-20。CU-D-590 真机已过。

## 本轮

- 590：`vcu browser observe --tab` 指定抛页；后台 tab 只 `tabs.update(active)`，不 `windows.update(focused)`。observe `capture_id` live viewport click `#hit` 0→1 后关闭。`scripts/poc_cu_d_590.py` CU-D-590 OK。组 1/3 未动。无 SendInput。不是 TC-B-040。`make check` 0。

## 下一刀

CU-D-600 候选：无 `--tab` 的 observe 在前台不是 USER Chrome/Edge 时要诚实失败（或要求 `--tab`），禁止静默绑到另一浏览器的 active tab。停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
