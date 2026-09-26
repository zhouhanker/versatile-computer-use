更新：2026-09-26。WIN-VIS-013。淡阴影改画在预乘文字层。单独阴影窗仍不显示。本机红底截图胶囊四角是红底，下方有一小段略暗，不是纯黑长方形。

更新：2026-09-26。Acrylic accent 忽略圆角，胶囊后面是直角黑底。现在按圆角区域模糊，不再调用那个 accent。文字层不再画外扩黑阴影。不是 macOS 系统材质。

更新：2026-09-26。淡阴影画进文字层，窗口顶部仍在屏幕内。单独的阴影窗仍不显示。本机截图没有纯黑长方形。

更新：2026-09-26。阴影窗会在胶囊后面画出纯黑长方形，已停止显示。胶囊本身还在。不是 macOS 系统材质。

# Windows 视觉对齐

更新：2026-09-26。作者：本机真机记录。状态：WIN-VIS-001 至 012 已在本机复测。011 把胶囊内容排成 macOS 那一组：系统强调色圆点、半粗标题、常规字重的 Esc 取消，宽度随内容收缩并限制在 220 到 320。不是两端拆开的半粗字。009 在 Windows 11 上用系统 Acrylic 做胶囊，白字叠在点击穿透层。失败时仍退回采样模糊。不是 macOS `NSVisualEffectView`，也不是完整 Windows 产品 CU。没有新的 GitHub Release。

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

## WIN-VIS-009 胶囊使用系统 Acrylic

状态：**2026-09-26 本机复测通过。** 不是 `NSVisualEffectView`，也不是完整 Windows 产品 CU。

- 006 到 008 是 `CopyFromScreen` 采样后再模糊，最快大约 480ms 才跟着背后变。macOS HUD 用的是系统材质，背后一变就糊进胶囊。
- 现在 Windows 11 上胶囊走 `SetWindowCompositionAttribute` 的 `ACCENT_ENABLE_ACRYLICBLURBEHIND`，圆角仍是胶囊，不是分层窗口。白字「VCU 正在使用这台 PC」和「Esc 取消」画在单独的点击穿透层上，避免被 Acrylic 染色。
- Acrylic 调用失败时退回 008 的采样模糊。没有 SendInput，没有移动系统光标。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_vis_009_acrylic.ps1` 输出 `WIN-VIS-009 OK acrylic green=24,59,37 red=75,25,28 bright=101 cursor=1780,358`。背后从绿变红后 220ms 内胶囊跟着变，快于 480ms 采样。左侧能读出白字。测完 Abort，只关闭自建窗。
- 不做：不写成和 macOS 材质逐像素相同，也不写成完整产品 CU。没有在 100% DPI 上复测。

## WIN-VIS-010 描边、半粗标题和轻阴影

状态：**2026-09-26 本机复测通过。** 不是 `NSVisualEffectView`，也不是完整 Windows 产品 CU。

- macOS 胶囊有 0.5pt 分隔线、semibold 标题和窗口阴影。009 的 Acrylic 只有白字，没有这三样。
- 现在文字层用 Segoe UI Semibold，并画 1px 半透明白描边。胶囊外另有一层点击穿透的轻阴影，向下偏 2 个设计像素。Acrylic 失败时采样模糊路径也用同一套半粗字体。没有 SendInput。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_vis_010_hairline.ps1` 输出 `WIN-VIS-010 OK acrylic hairline=404/119 shadow=122/260 green=23,55,35 red=70,23,26 bright=139 cursor=1780,358`。描边亮于胶囊中心，阴影暗于更下方的背景，颜色仍在 220ms 内跟着变。系统光标没有动。测完 Abort，只关闭自建窗。
- 不做：不写成和 macOS 逐像素相同，也不写成完整产品 CU。没有在 100% DPI 上复测。

## WIN-VIS-011 胶囊内容按 macOS 排成一组

状态：**2026-09-26 本机复测通过。** 不是 `NSVisualEffectView`，也不是完整 Windows 产品 CU。

- macOS 胶囊是强调色圆点、半粗标题、常规字重的「Esc 取消」排在一起，宽度随内容收缩，限制在 220 到 320。010 的 Windows 胶囊把两段字都画成半粗，并拆到左右两端。
- 现在标题仍是 Segoe UI Semibold，「Esc 取消」改为 Segoe UI 常规字重。左侧圆点读 Windows 强调色，对应 macOS `controlAccentColor`。读不到或过暗时退回 `0,120,215`。宽度按内容计算，再限制在 220 到 320 设计像素。没有 SendInput，没有移动系统光标。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_vis_011_grouped.ps1` 输出 `WIN-VIS-011 OK acrylic grouped w=330 h=42 dot=80/16 text=211/43 right=0 accent=0,120,215 cursor=1780,358`。这台机器 DPI 是 144，330 是 220 设计像素的下限。圆点在白字左侧，胶囊右端没有再钉住的白字。系统光标没有动。测完 Abort，只关闭自建窗。
- 不做：不写成和 macOS 逐像素相同，也不写成完整产品 CU。没有在 100% DPI 上复测。网页指针仍是共享的 `content.js`，这轮没有改。

## WIN-VIS-012 网页指针雾心对齐 macOS 短箭

状态：**2026-09-26 本机 Edge 复测通过。** 不是逐像素复刻 Codex 裁图，也不是完整 Windows 产品 CU。

- 以前网页雾晕是 66px，偏在箭尖左上方，渐变中心约在箭尖 + (7.7, 7)。macOS 和 Windows 桌面短箭的雾心是箭尖 + (6, 6)，半径 36。
- 现在共享的 `extension/content.js` 把雾晕放在 `left/top -30px`、72px 盒子的中心就是箭尖 + (6, 6)，半径 36。内圈是 44px，对应 macOS 那个 44px 圆。两端浏览器用同一份，没有另做 Windows 指针。主机元素带 `data-vcu-fog=6,6,36`。
- 验收：单测 `windows_stage_matches_macos_capsule_and_dart` 通过。`scripts/poc_win_edge_cursor.py` 输出 `EDGE-CURSOR OK edge fog=6,6,36 hit=0->1 source=extension_dom cursor=1187,239`。只打开自建页，测完只关这个标签。没有测 Chrome，没有点允许调试，系统光标没有动。
- 不做：不把这次写成 Codex 裁图逐像素相同，也不写成完整产品 CU。
