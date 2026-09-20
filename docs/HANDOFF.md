# VCU 会话交接

更新：2026-09-20。CU-D-400 真机已过。

## 本轮

- 400：`scripts/poc_cu_d_400.py` `CHROME_HELLO_OK` / `EDGE_HELLO_OK` / `MERGE_OK browser_count=2` / `CU-D-400 OK`。tabs chrome=5 edge=13。根因：Chrome/Edge unpacked 共用同一个 `chrome.runtime.id`，clients map 互相覆盖；key 改为 `browser:id`，poll 带 `browser=`。组 1/3 未动。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
