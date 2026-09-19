# VCU 会话交接

更新：2026-09-20。桌面史诗进行中。

## 本轮

- CU-D-041 **真机通过**（Terminal.app，非 Ghostty）：Scene `CGWindow`；`vcu type` → `ax_menu_paste`；剪贴板读回标记；含换行的 type 与无 confirm_send 的 Return 均为 `FocusPolicyViolation`。
- 终端 AX 无文本区，输入不是 AXSetValue，也不是 HID keystroke。

## 下一刀

CU-D-042 飞书 / Lark **客户端**：打开已有会话、读可见消息。禁止自动发送。不要碰 Edge 组 1/3。
