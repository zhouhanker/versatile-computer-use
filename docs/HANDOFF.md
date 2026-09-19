# VCU 会话交接

更新时间：2026-09-20。作者zhouhanker。

桌面史诗进行中。浏览器 0.2.8 冻结有效。

## 本轮

- CU-D-011：`POST /v1/session/{id}/abort` + `vcu session abort`；HTTP 测会话消失。
- CU-D-020：AXPress 必须 `axpress:0`；去掉 `click el` 回退。
- CU-D-021/022/024：既有单测 + 像素路径不得报 `extension_dom`。
- CU-D-023 真机 TextEdit **未做**。

`cargo test --workspace --offline` 绿。

## 下一刀

CU-D-023 受控 TextEdit 真机，或阶段 3 CU-D-030 Observation 契约。不要碰用户 Edge 组 1/3。
