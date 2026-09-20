# VCU 会话交接

更新：2026-09-20。CU-D-520 真机已过。

## 本轮

- 520：同一次 `/v1/browser/tabs` chrome=6 edge=7，`browsers_ok=["chrome","edge"]`，`browsers_failed=[]`，0.06s。Edge 需加载新 background（POST poll / hello pull / offscreen）；`install-lens --reload` 在 reload_self 超时后打开 `chrome-extension://<id>/reload.html`。`scripts/poc_cu_d_520.py` CU-D-520 OK。组 1/3 未动。无 SendInput。

## 下一刀

停放 P2：TC-B-040 / 跨源 iframe / 产品 Windows CU。不要 claim 完整 Codex CU。不要 claim MAC-NEXT / FEISHU-001。
