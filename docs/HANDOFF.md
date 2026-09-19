# VCU 会话交接

更新：2026-09-20。桌面史诗进行中。

## 本轮

- CU-D-040 **未完成**。已做：`vcu-stage --list-windows` + Finder Scene 合并 `role=CGWindow`。真机 `scripts/poc_desktop_finder.py` 看到 `/tmp/VCU-D-040-*` 与 `OPENME` 窗口标题。
- 不要用 `tell application "Finder"`（超时）。`open -a Finder <path>` 可用。
- Finder AX 不暴露文件夹窗口/图标：System Events `windows=0`；AXWindows 是嵌套 AXApplication + 桌面 AXScrollArea；hit-test 文件夹窗只返回 `AXApplication 访达`。因此 **选图标 / AXPress / 回车打开不得标过**。禁止 HID。

## 下一刀

继续 CU-D-040：在不 warp 光标、不点 TCC 的前提下，找到诚实的选图标/打开路径（或把该条明确记为 OS 限制并拆出 040b）。未完成前不要领 CU-D-041。不要碰 Edge 组 1/3。
