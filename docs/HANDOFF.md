# VCU 会话交接

更新：2026-09-20。macOS 第一版门禁已齐。Windows 真机仍未跑。

## 本轮

- CU-D-060：把 Windows 成功路径抽成 `snapshot_from_uia` / `invoke_from_uia_output` / `set_value_from_uia_output`，在 macOS 上也能单测（ok 与失败）。交叉编译缺 mingw gcc，未做 windows-gnu 链接。
- 真机 UIA 仍需 Windows 主机。

## 下一刀

Windows 主机跑 Notepad 真机。不要宣称已有 Windows 桌面 CU。
