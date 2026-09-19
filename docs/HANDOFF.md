# VCU 会话交接

更新：2026-09-20。CU-D-060/070 CI 过；CU-D-080 单测过。

## 本轮

- 070 CI：`SETVALUE_OK path=wm_settext`（非 ValuePattern）
- 080：`uia_set_value_script` 回退 `WM_SETTEXT`；单测解析 `ok:wm_settext`

## 下一刀

CU-D-090：Windows 上经 `vcu` 的 desktop/list/set_value（仍不要 Stage 产品宣称）。禁止 SendInput / 微信 / Allow。
