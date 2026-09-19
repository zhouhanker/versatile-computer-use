# VCU 会话交接

更新：2026-09-20。桌面史诗进行中。

## 本轮

- CU-D-042 **观察通过、未发送**：`proc:Feishu:11666` Scene `CGWindow` 标题「飞书」；无 AX 发送控件；Return `FocusPolicyViolation`。聊天气泡不在 AX（Electron webview）。
- 单测：Feishu 快照后点 `e_send`（发送）→ FocusPolicyViolation。

## 下一刀

CU-D-043 系统设置：只读观察；辅助功能 repair 只给 hint，不自动改权限。
