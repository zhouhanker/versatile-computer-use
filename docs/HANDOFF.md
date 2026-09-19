# VCU 会话交接

更新：2026-09-20。CU-D-100 CI 已过。

## 本轮

- CU-D-100 run `35466230837` @ `f6bcb28`：
  - `SNAP_OK ref=e2` (VcuCount, ControlType.Pane)
  - `INVOKE_OK path=bm_click os_cursor_used=False`
  - `CU-D-100 OK`
- **不是** UIA InvokePattern；WinForms 按钮无 InvokePattern，走 BM_CLICK (0xF5)。无 SendInput。

## 下一刀

不要宣称完整 Windows CU。可开 CU-D-110 浏览器 P2 或更多 App。禁止微信 / HID。
