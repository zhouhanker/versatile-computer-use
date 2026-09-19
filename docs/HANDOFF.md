# VCU 会话交接

更新：2026-09-20。桌面史诗进行中。

## 本轮

- CU-D-023 真机通过 `2b12a1c`。
- CU-D-032 真机通过 `e197160`：ping 0.2.8、现窗新标签、screenshot capture dry-run、throwaway DOM click。组 1/3 未改。
- CU-D-040 **未开始实现**。探针：`tell application "Finder"` 会超时（可能缺 Automation TCC，不要去点系统对话框）。`open -a Finder <path>` 与 System Events 取 pid 正常。desktop snapshot `proc:Finder:731` 得到 0 refs：`cannot get window 1 of process Finder`。默认不抢焦点。

## 下一刀

CU-D-040 Finder：脚本自建临时目录，用 `open -a Finder` 开窗（不要 `tell application "Finder"`）。先让 Scene 看到图标，再 AXPress 打开子项。禁止批量删除、禁止 Return 盲发。不要碰 Edge 组 1/3。
