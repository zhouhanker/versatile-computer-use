# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机仍未跑。

## 本轮

- CU-D-060 接上 `uia_capture_script`（PrintWindow → PNG base64）+ `parse_uia_capture_output`。单测：脚本含 PrintWindow，不含 SendInput/CopyFromScreen；1×1 PNG 能解析。
- 真机 UIA/截图仍需 Windows 主机。

## 下一刀

在 Windows 上对 Notepad 跑 list / UIA 树 / InvokePattern / PrintWindow。不要在 macOS 上宣称 Windows CU 已可用。
