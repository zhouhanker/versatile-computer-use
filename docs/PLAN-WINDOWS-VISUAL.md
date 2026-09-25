# Windows 视觉对齐

更新：2026-09-26。作者：本机真机记录。状态：WIN-VIS-001 至 008 已在本机复测。008 刷新时采的是胶囊正后方，仍不是 macOS 系统材质，也不是完整 Windows 产品 CU。没有新的 GitHub Release。

## 边界

- 不移动系统光标。不调用 SendInput / SendKeys。
- 不点 Edge Allow。不自动化微信。不改 `~/.codex/computer-use/`。
- 网页指针仍是共享的 `extension/content.js` 短箭，不另做一套 Windows 网页指针。
- 不把这次外观对齐写成带 Stage 的完整产品会话，也不 claim `MAC-NEXT`。

## WIN-VIS-001 Windows Stage 对齐 macOS 胶囊和短箭

状态：**形状已对齐并在本机看到窗口。** 不是 macOS 的系统材质模糊，也没有逐像素 DPI 对齐。

- 以前：Windows HUD 是 320x32 的直角蓝条。Guide 是 48x48 橙色方块，中心对准坐标。
- 现在：HUD 宽 280、高 28，圆角胶囊，文案仍是「VCU 正在使用这台 PC」和「Esc 取消」。Guide 窗口 84，热点是箭尖 `(32, 34)`，不是窗口中心。雾晕半径 36，短箭和 macOS `helpers/vcu-stage/main.swift` 以及网页指针是同一组相对点。Guide 设置 `WS_EX_TRANSPARENT`，不挡点击。
- 没做成：先试了分层窗口的逐像素透明，本机画出来是黑的，所以实机窗口改用 WinForms Region。雾晕没有 macOS 的材质模糊。
- 验收：`VCU_STAGE_RENDER` 写出的 `hud.png` / `guide.png` 是胶囊和短箭。实机进程能起来，写入 `stop` 后退出 0。150% DPI 下，按窗口报告坐标的 1.5 倍截到圆形短箭，不是橙色方块。同进程 `CopyFromScreen` 截到黑块，不能当视觉证据。
- 未宣称：这台机器的 Stage 进程不是 DPI 感知的。窗口报告坐标和外部物理像素差 1.5 倍。没有证明 Guide 尖端和 UIA 点在每个缩放下重合。

## 网页指针

网页里的虚拟指针已经是两端共用的 `content.js` 短箭，几何和 macOS Swift Guide 相同。本条不改网页指针。若以后要改，只能改这一份共享实现，不能做 Windows 专用外观。

## CU-D-610 不是这条的范围

`poc_cu_d_610.py` 在滚动之后用滚动前的截图做 dry-run，扩展返回 `stale_viewport` / `stale viewport; recapture screenshot`。这是共享扩展的安全拒绝，不是 GBK，也不是 Windows 回归。不要为了变绿放宽 `scroll_y` 校验。

## WIN-VIS-002 逐像素透明

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

上一轮把分层窗口记成画出来是黑的。那是 150% DPI 下截图坐标看错了，不是绘制失败。独立探针在物理坐标上能看到半透明蓝胶囊。

- 实机 Stage 不再用 WinForms Region 和不透明圆。HUD 与 Guide 都走 `UpdateLayeredWindow`。
- 本机截图：胶囊条能读出「VCU 正在使用这台 PC」和「Esc 取消」，背景透出桌面。Guide 是软雾加短箭，不是橙色方块。
- Guide 仍设置点击穿透。没有 SendInput，没有移动系统光标。
- 未宣称：Stage 进程仍不是 DPI 感知的。和 UIA 坐标是否在每个缩放下重合，这轮没有新的对照证据。

## WIN-VIS-003 物理像素对齐

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

这台机器 DPI 是 144。`app snapshot` 的 Edge 窗口框是 `[-11, -11, 2182, 1390]`，和 DPI 感知的 `GetWindowRect` 一致。以前 Stage 进程不感知 DPI，把这个物理坐标当成虚拟像素，150% 下指针会偏到 1.5 倍的位置。

- Stage 启动时调用 `SetProcessDpiAwarenessContext(-4)`，失败再退到 `SetProcessDpiAwareness(2)`。本机状态：`aware=True dpi=144 scale=1.5`。
- 胶囊和短箭按 `dpi/96` 放大，坐标仍是物理像素。请求 Guide `(120, 120)` 时，DPI 感知截图在该点附近看到短箭，不是在 `(180, 180)`。
- HUD 落在物理工作区中心 `(870, 12)`，胶囊文案可读。
- 未宣称：没有在 100% 和其他缩放下复测。网页指针仍是共享的 `content.js`，这轮没有改。

## WIN-VIS-004 胶囊文案不重叠，网页指针已在 Edge 出现

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 144 DPI 下把字号再乘 `dpi/96` 会和系统字号叠乘，标题和「Esc 取消」挤在一起。现在字号用像素单位，按设计尺寸乘缩放，左右留白。实机胶囊上两段文字分开，能读出「VCU 正在使用这台 PC」和「Esc 取消」。
- 网页指针没有另做 Windows 版本。本机 USER Edge 对 `127.0.0.1` 抛页做了一次真点击：按钮从 0 变成 1，`#vcu-virtual-cursor` 出现在页面里，`source=extension_dom`，`os_cursor_used=false`。没有点 Allow，没有移动系统光标。标签随即关闭。

## WIN-VIS-005 桌面雾色和网页指针同一组色标

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 网页指针用 `rgba(148,168,188,.50)`、`rgba(170,184,200,.26)`、`rgba(206,212,222,.11)`。macOS Swift 第一档是 `0.58, 0.66, 0.74`，也就是 148,168,188。Windows Guide 以前用另一组灰 `168,182,196`。
- 现在 Windows 雾晕按同一组色标插值，半径仍是 36，短箭几何不变。单测要求 Stage 脚本和 `extension/content.js` 都带这三档。
- 本机物理坐标 `(120, 120)` 上仍能看到软雾和短箭，不是硬环，也不是橙色方块。没有移动系统光标。

## WIN-VIS-006 胶囊采样桌面并做模糊

状态：**2026-09-26 本机复测通过。** 不是 `NSVisualEffectView`，不是持续更新的 DWM Acrylic，也不是完整 Windows 产品 CU。

- macOS HUD 用 `NSVisualEffectView.Material.hudWindow`，背后内容会糊进胶囊。Windows 以前是不透明海军蓝 `FromArgb(245, 18, 46, 107)`。
- 现在 Stage 在升起窗口之前，按物理像素 `CopyFromScreen` 采样胶囊位置，缩小再放大当作模糊，再盖一层半透明石墨 `FromArgb(150, 24, 28, 34)`。边缘仍走 `UpdateLayeredWindow`。没有 SendInput。
- 这是显示瞬间的采样，窗口后面的内容之后再变，胶囊不会跟着更新。透明度效果关闭时也不等于系统材质。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_vis_006_material.ps1` 在 HUD 槽放一块 `255,0,180` 的自建窗，会话拉起 Stage 后，胶囊左侧像素平均是 `125,26,105`，不是纯品红，也不是旧的不透明海军蓝。原点 `(870, 8)`。测完 Abort，只关闭这个自建窗。
- 不做：不把这次写成和 macOS 材质逐像素相同，也不写成完整产品 CU。

## WIN-VIS-007 模糊跟着背后变化

状态：**2026-09-26 本机复测通过。** 不是 `NSVisualEffectView`，不是 DWM Acrylic，也不是完整 Windows 产品 CU。

- 006 只在 Stage 升起前采样一次。背后变了，胶囊还是旧颜色。
- 现在大约每 480ms 采样胶囊正下方 8 像素的一条，颜色变了就重画。不把胶囊自己采进去，所以截图仍能看见胶囊。没有 SendInput，也没有把窗口排除出截图。
- 这条采样的是胶囊下方，不是正后方的每一个像素。桌面内容不均匀时，只是邻近色的近似。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_vis_007_refresh.ps1` 先放 `255,0,180`，胶囊像素 `119,16,99`；不重启 Stage，把自建窗改成 `0,255,40` 后像素变成 `14,121,38`。测完 Abort，只关闭这个自建窗。
- 不做：不写成和 macOS 材质相同，也不写成持续的系统模糊。

## WIN-VIS-008 刷新时采正后方，不是只采下方

状态：**2026-09-26 本机复测通过。** 不是 `NSVisualEffectView`，不是 DWM Acrylic，也不是完整 Windows 产品 CU。

- 007 刷新时只采胶囊正下方 8 像素。正后方换成别的颜色，胶囊仍跟着下方走。
- 现在刷新前短暂把胶囊排除出截图（`WDA_EXCLUDEFROMCAPTURE`），`CopyFromScreen` 采到正后方，随即恢复。平时截图仍能看见胶囊。失败时退回下方那一条。没有 SendInput。
- 采样大约每 480ms 一次。排除截图只在这一小段，不是一直隐藏。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_vis_008_behind.ps1` 先让正后方全蓝，胶囊像素 `14,49,125`；不重启 Stage，只把正后方改成绿、下方仍是蓝，像素变成 `14,121,36`。这不是纯绿，说明排除截图已经恢复。测完 Abort，只关闭这个自建窗。
- 不做：不写成系统材质，也不写成每一帧都在更新。

