# VCU 会话交接

更新：2026-09-20。CU-B-011 doctor 双浏览器 lens 诊断。

## 本轮

- `lens_dual_browser`：Chrome+Edge 都装了但 lens 只轮询其中一个 → warn。
- 真机仍是 Edge-only。osascript 查 Chrome tabs 被 TCC 卡住，未再强开。
- **双浏览器 tabs 复检仍未过。** 跨源 / trusted / TC-B-040 仍排除。

## 下一刀

Chrome USER 配置里 Load unpacked 之前，不要宣称 P2 双浏览器完成。禁止改组 1/3、禁止点 Allow。
