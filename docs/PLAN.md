# VCU 当前计划（浏览器版）

更新：2026-09-26。本文优先于 HANDOFF 历史快照；作者 zhouhanker。Windows 产品会话证据到 CU-WIN-SESSION-056。不要把旧哈希 `e5d47df` 当成当前 HEAD。

## 当前节点

**已发布：浏览器版 Bridge 0.2.8。** GitHub Release `v0.2.8`（Latest）已上线，`curl | sh` 与 `vcu self update` 均真机验证通过。浏览器门禁仍有效。`vcu --version` 仍打印 crate `0.1.0`，不代表没更新。

**排期切片停在 CU-D-700。** CU-D-010…700 已关闭。Windows 产品会话的已复测切片见 [PLAN-EPIC-WIN-SESSION.md](PLAN-EPIC-WIN-SESSION.md)，目前到 CU-WIN-SESSION-056。这不是完整 Codex CU，也不是完整 Windows 产品 CU。CI 门禁已由 Actions run `36129175359`（代码 `9c69b8c`）恢复：`test` 三平台与 `package-macos` / `package-windows` 全绿。不要 claim `MAC-NEXT` 或 `FEISHU-001`，除非用户点名。

## 下一步大纲（未开工）

默认不新开切片，也不把停放项写成进行中。

Windows 真机缺口与修复顺序见 [PLAN-WINDOWS-FIX.md](PLAN-WINDOWS-FIX.md)。WIN-FIX-001 至 009 已在这台机器复测。009 修复 powershell 图形窗口被当成控制台粘贴。CU-D-610 的截图 dry-run 仍失败。不要把它写成完整 Windows 产品 CU。Windows 产品会话的第一扇门见 [PLAN-EPIC-WIN-SESSION.md](PLAN-EPIC-WIN-SESSION.md)。Windows 窗口外观对齐见 [PLAN-WINDOWS-VISUAL.md](PLAN-WINDOWS-VISUAL.md)。WIN-VIS-001 至 011 已复测。011 把胶囊内容排成强调色圆点、半粗标题和常规字重的 Esc 取消。008 刷新时采胶囊正后方，不是 macOS 系统材质，也不是完整产品会话。
可领取项只有停放的 `MAC-NEXT` 与 `FEISHU-001`。

若要继续产品工作，先由用户点名一条，再 claim。建议顺序：

1. **守门**：CI 或真机回归坏了再修。不扩范围，不把修复写成新产品能力。
2. **浏览器诚实缺口**（点名才开）：跨源 iframe、trusted 手势、`TC-B-040`。当前已验证的是 `source=extension_dom` 的 viewport 路线。
3. **Windows 产品会话**（点名才开）：CU-D-060…390 只是 CI 真机，不是 `vcu session` 产品路径。不得把 `WM_SETTEXT` 写成 UIA ValuePattern。
4. **`MAC-NEXT` 深 AX**（点名才开）：先过 Accessibility 门禁，再谈完整 AX 树。发布托管已由 CU-D-700 落地，不要重复做。
5. **`FEISHU-001`**（点名收信人才开）：只观察不够时才谈发送。禁止自动发送，禁止碰微信。

红线不变：不搬系统光标，不代点 Edge Allow，不自动化微信，不改 `~/.codex/computer-use/`，不动用户标签组 1/3。网页细操作必须 `source=extension_dom`。代码提交前要有足够 POC。

**已关闭切片流水（至 CU-D-700，不是进行中史诗）：** 对标 Codex Computer Use **含桌面**。阶段 1–5 已过。CU-D-060 CI 真机列窗+PrintWindow 已过（run `35462329205`）。CU-D-070 CI `SETVALUE_OK path=wm_settext`（非 ValuePattern）。CU-D-080 单测已过。CU-D-090 CI 已过。CU-D-100 CI 已过。CU-D-110 CI 已过。CU-D-120 CI 已过。CU-D-130 CI 已过。CU-D-140 CI 已过（`TYPE_OK path=clipboard_paste` + `NEWLINE_DENIED`）。未宣称完整 Windows 产品 CU。CU-D-140/150/160/170/180/190/200/210/220/230 已过。CU-D-230 CI `TYPE_OK path=clipboard_paste` + `NEWLINE_DENIED` run `35473547564`。CU-D-240 CI `ABORT_OK hud=false` / `HUD_GONE` run `35473997888`。CU-D-250 CI `HOVER_OK path=guide_hover` run `35474340766`。CU-D-260 CI `HOVER_OK` then `INVOKE_OK path=bm_click` run `35474689948`。CU-D-270 CI `SCOPE_OK` / `BACKEND_OK` run `35475108584`。CU-D-280 CI `TOOLS_OK` / `CU-D-280 OK` run `35476413803`。CU-D-290 CI `HOVER_OK path=guide_hover` / `ABORT_OK via=mcp_tools_call` run `35477727787`。CU-D-300 CI `INVOKE_OK path=bm_click` / `CU-D-300 OK` run `35478128021`。CU-D-310 CI `WAIT_OK path=scene_wait` / `CU-D-310 OK` run `35478526783`。CU-D-320 CI `TYPE_OK path=wm_settext` / `CU-D-320 OK` run `35479050203`。CU-D-330 CI `NEWLINE_DENIED via=mcp_tools_call` / `CU-D-330 OK` run `35479929439`。CU-D-340 CI `SCROLL_OK path=wm_vscroll` / `CU-D-340 OK` run `35480404968`。CU-D-350 CI `EXTRACT_OK count=1` / `CU-D-350 OK` run `35481212302`。CU-D-360 CI `SHOT_OK mime=image/png` / `CU-D-360 OK` run `35481626341`。CU-D-370 CI `KEY_DENIED via=mcp_tools_call` / `CU-D-370 OK` run `35482034139`。CU-D-380 CI `WAIT_MISS_OK` / `WAIT_REF_MISS_OK` / `CU-D-380 OK` run `35482614685`。CU-D-390 CI `SCOPE_OK` / `BACKEND_OK` / `STAGE_OK via=mcp_tools_call` / `CU-D-390 OK` run `35483037087`。CU-D-400 真机 `CHROME_HELLO_OK` / `EDGE_HELLO_OK` / `MERGE_OK browser_count=2` / `CU-D-400 OK`（Chrome 5 + Edge 13 tabs）。未宣称完整 Windows 产品 CU / 完整 Codex CU。CU-D-410 真机 health `extension_browsers=["chrome","edge"]` count=2，无占位 `browser`。CU-D-420 真机 observe 进程名 `Chrome` `target.allowed=true`，无 AppDenied。CU-D-430 真机 observe `tabs_source=extension_tabs`。CU-D-440 真机 `/v1/browser/observe` 统一 CLI/MCP。CU-D-450 真机 last_observe 绑定。CU-D-460 lens 按 observe 浏览器定向。CU-D-470 screenshot 绑 last observe。CU-D-480 open 绑 observe 浏览器。CU-D-490 desktop.scene 附 browser_tabs。CU-D-500 MCP initialize 已改。CU-D-510 `login-state`/`next` 在 lens polling 时 next_action 为 ping→observe→60s last observe；CLI help 同步。CU-D-520 真机同一次 tabs chrome+edge。CU-D-530 真机 observe 绑前台 USER 浏览器。CU-D-540 真机 login-state 前台优先。CU-D-550 真机 extract 绑 last observe。CU-D-560 health/init observe-bind 文案。CU-D-570 lens observe 不走 AX 全树。CU-D-580 observe capture 可 viewport dry-run click。CU-D-590 真机 `observe --tab` 抛页 live viewport click 0→1。CU-D-600 无 `--tab` 时前台非 USER Chrome/Edge 则诚实失败。CU-D-610 真机 observe 后 live type/scroll 绑 last observe。CU-D-620 真机 hover + DOM wait。CU-D-630 真机 observe --browser 定向 Chrome/Edge。CU-D-640 真机 close --browser 定向。CU-D-650 真机 extract/type/click --browser。CU-D-660 真机 hover/scroll/wait --browser。CU-D-670 真机 open --browser 与 group 同浏览器。CU-D-680 `group-update --browser`、CU-D-690 原生 `<select>`、CU-D-700 Release 托管 + `vcu self update` 均已真机/发布验证。`make check` 0。其后停放 P2。未宣称完整 Windows 产品 CU / 完整 Codex CU。总览与编排见 [`ROADMAP-CU.md`](ROADMAP-CU.md)，测试见 [`testing/DESKTOP_CU_TEST_PLAN.md`](testing/DESKTOP_CU_TEST_PLAN.md)。排期桌面切片已关闭，仍不得把 VCU 写成完整桌面 CU 或完整 Windows 产品 CU。

本地对照实现在 gitignore 的 `/reference/computer-use/`：可反编译作结构参考，笔记进 `docs/research/`，自有代码进仓库；`.app` / 反编译源码 / 官方素材不上传。

0.2.8 仍是登录态浏览器操作层；桌面工作不得回退 `make check` / 扩展测试，不得 warp OS 光标、代点 Allow、自动化微信、修改 Codex CU 安装。

额度规则：用户已重置总额度。旧“周额度不足10%”提示无效；今后按重置后的**总额度剩10%**要求写交接，不能把goal token计数当账户余额。

## 目标与范围

对比Codex Computer Use，改进浏览器操作与虚拟光标，采用参考图中的原生彩色可折叠标签组，持续修复问题，每阶段留下可复核记录。

- 主路径：USER Edge/Chrome + Browser Bridge，保留登录态。宿主已有视觉时不要求`vcu init model`。
- 网页：明确tab、DOM selector、绑定截图的viewport坐标点击、输入/滚动、原生标签组。
- 浏览器整窗：macOS窗口ID截图与Guide；AXPress失败必须诚实报错。
- 0.2.8 冻结：不做桌面 App 产品路径。微信自动化、CDP Allow、OS cursor warp、修改 Codex CU 安装在桌面史诗中也仍然禁止。桌面编排见 ROADMAP-CU。

## 阶段清单

| ID | 阶段 | 当前状态 | 完成证据 / 剩余项 |
| --- | --- | --- | --- |
| PARITY-001 | Codex比较与阶段设计 | 完成 | `docs/design/09-browser-interaction-parity.md` |
| PARITY-002 | 精确tab与DOM动作 | 完成 | 失效ID不fallback、唯一/可编辑/无遮挡目标、无mutation重放；真机DOM通过 |
| PARITY-003 | 原生标签组与网页选择 | 完成 | 命名、折叠/展开、选择自动展开、解除分组；CLI/MCP和真实窗口POC |
| PARITY-004 | Codex光标外观对齐 | 同背景终验已做并修 halo | 浅色同页对照 native/DOM/Guide；放大光晕至约66px 圆雾。不宣称逐像素动画或官方资源复刻 |
| PARITY-005 | 最终整体验收 | Chrome 真机 + 原生 popup 已过 | 40 Node；Chrome extract/click/type source=extension_dom，counter 0→1；原生 popup 跨窗禁选。双扩展错路由会 retryable |
| PARITY-006 | 绑定截图的网页坐标点击 | 完成 | capture绑定文档/布局、60秒过期、一次消费；真实点选counter0→1 |
| PARITY-007 | 多窗口与面板约束 | **本节点完成** | 后台开窗不抢焦点、跨窗拒绝无副作用、组显式保留所属窗口；面板按窗口分区/跨窗禁选 |
| PARITY-008 | 布局变化与截图可靠性 | **本节点完成** | CSSOM移动/遮挡、input事件、JSON排序往返、截图频率控制、大PNG回执；正向/反向测试均通过 |

以前的TEST/EXTRACT/ETH阶段属于基线。ETH只是L1/L2/L3样本，不是全站抓取；FEISHU-001停放。旧MAC-NEXT深AX不是本版本下一步。TC-B-040通用AX网页像素真点仍不能当成已通过，当前已验证的是extension DOM viewport路线。

## 本节点验收

- `rtk proxy make check` exit **0**：**102 Rust +35 Node**；mock/extra/login-state、release打包、checksum/curl-install/MCP smoke全部通过。
- `scripts/poc_browser_parity.py --live`：**32项通过**，测试页安全清理。包含稳定截图允许point dry-run，以及CSS/输入变化必须拒绝；不再只有负向测试。
- 真实看图点选：PNG像素(120,318) → CSS(75,198.705)，counter **0→1**，只发生一次。
- 布局JSON经过Rust重排键后仍可用；完整几何留在sidecar，模型只接收摘要。
- 截图统一排队以满足浏览器每秒2次限制；只读截图可做一次限流恢复，点击/输入等mutation不自动重放。
- AX先启用再枚举窗口，跳过小控制浮窗；截图失败不包装成成功，不使用被遮挡的屏幕区域冒充浏览器图像。

完整结果：`docs/testing/BROWSER_PARITY_RESULTS.md`；机器可读索引：`docs/testing/BROWSER_PARITY_NODE_REPORT.json`。门禁在定版提交前的工作树通过；定版 SHA 为 `d8ee9ad`。

## 下一阶段（按优先级）

- [x] **P1 / PARITY-004**：同背景浅色页对照 native/DOM/Guide 的 idle/click/move；DOM 按 tab zoom 逆缩放。发现 halo 过小过淡后已加大圆雾并统一 Guide。约束仍有效：不再向用户索要截图；不退回长箭尾/硬圆环；仅 mismatch 时改代码；可参考开源/公开技术；不修改私有安装。
- [x] **P1 / PARITY-005**：Chrome 真机 DOM extract/click/type 已过（counter 0→1，input=chrome-live，遮挡拒绝）。原生 popup 已过。用户 1/3 组未改。
- [x] **P2 / PARITY-005（定版提交）**：工作树已定版提交 `d8ee9ad` 并准备推送 origin/main。AWR 证据绑定该 SHA；光标终验与 Chrome 真机仍未完成。
- [x] **P2 / 0.2.7 体验补丁**：光标朝向与点击压缩、viewport 截图保留光标、合并 Edge+Chrome tabs、CLI/MCP hover、同源 iframe 内层点选、canvas 合成点击。
- [x] **P2 / 0.2.8**：`install-lens --reload` 热更新已连接 Edge/Chrome SW；默认 `open` 在现有 USER 窗口开新标签。
- [ ] **P2 后续（浏览器）**：跨源 iframe / trusted 手势 / TC-B-040 仍不在范围内。双浏览器 live 已过（CU-D-400）。
- [x] **CU-B-010**：tabs/health 报告 `browser_count`/`browsers`（单测）。真机 Chrome 未连。
- [x] **CU-B-011**：doctor `lens_dual_browser` 在双浏览器已装但只连一个时 warn（单测）。真机仍 Edge-only。
- [x] **CU-D-010**：无 Stage HUD 则 desktop session 失败（`StageRequired`）；HTTP `stage_hud=true`。
- [x] **CU-D-011**：`vcu session abort` / HTTP abort 拆会话（单测）。未弹用户 HUD 做目视 Escape。
- [x] **CU-D-012**：微信 denylist + 覆盖点拒绝（单测）。
- [x] **CU-D-020/021/022/024**：AXPress 非零失败、Return 门禁、Guide overlay、像素命中不报 extension_dom（单测）。
- [x] **CU-D-023**：TextEdit 真机 `e8` AXTextArea 写入标记，`os_cursor_used=false`；Notes AXPress `axpress:0`。CLI session `type --tab`；`resolve_tab` 优先 active app。
- [x] **CU-D-032**：浏览器回归 ping 0.2.8 / tabs / 现窗新标签 / screenshot capture dry-run / throwaway DOM click。组 1/3 未动。
- [x] **CU-D-040**：Finder CG 列窗 + VCU `reveal`/`open_path`（nsworkspace）真机打开 `OPENME`。非 AXPress、非 Return。
- [x] **CU-D-041**：Terminal.app `ax_menu_paste` 真机输入；换行/Return 拒绝。未碰 Ghostty。
- [x] **CU-D-042**：飞书客户端 CG 窗观察；无发送；气泡不在 AX。
- [x] **CU-D-030/031/033**：Observation surface/source；USER Edge AXWebArea 不得假绿；`playbooks/desktop.md`。
- [x] **CU-D-050/051/052**：HUD 文案、Guide 短三角、`VCU_AUDIT=1`。
- [x] **CU-D-060**：CI run `35462329205` Notepad `UIA_OK` + `PRINTWINDOW_OK`。非 `vcu session` 产品路径。
- [x] **CU-D-070**：CI `SETVALUE_OK path=wm_settext`（Edit 无 ValuePattern）。
- [x] **CU-D-080**：backend set_value 回退 WM_SETTEXT（单测）。
- [x] **CU-D-090**：CI `STAGE_OK`/`SNAP_OK`/`TYPE_OK path=wm_settext`。
- [x] **CU-D-100**：CI `INVOKE_OK path=bm_click`（非 InvokePattern）。
- [x] **CU-D-110**：CI `SHOT_OK` 768x519 PNG（PrintWindow）。
- [x] **CU-D-120**：CI `OPEN_OK path=explorer_open`。
- [x] **CU-D-130**：CI `REVEAL_OK path=explorer_reveal`。
- [x] **CU-D-140**：CI `SNAP_OK`/`TYPE_OK path=clipboard_paste`/`NEWLINE_DENIED` run `35469208163`。无 SendInput；未执行命令。
- [x] **CU-D-150**：`playbooks/desktop.md` Windows 最短环（cmd `clipboard_paste`，禁换行）。
- [x] **README**：Windows CI 覆盖（cmd `clipboard_paste` / Explorer / WinForms Stage）；明确非产品会话。
- [x] **CU-D-160**：CI `SNAP_OK ref=e14` / `INVOKE_OK path=bm_click` run `35470610934`（win32calc）。无 SendInput。
- [x] **CU-D-170**：CI `SNAP_OK` / `CLICK_DENIED` / `TYPE_DENIED` run `35471043411`（SystemSettings）。无 SendInput；未改设置。
- [x] **CU-D-180**：CI `SCROLL_OK path=wm_vscroll` run `35471497693`。无 SendInput / mouse_event。
- [x] **CU-D-190**：CI `EXTRACT_OK count=1` run `35471859281`。无 SendInput。
- [x] **CU-D-200**：CI `WAIT_OK path=scene_wait found_ref=e2 os_cursor_used=False` / `CU-D-200 OK` run `35472199783`。无 SendInput。
- [x] **CU-D-210**：CI `WAIT_MISS_OK` / `WAIT_REF_MISS_OK` / `CU-D-210 OK` run `35472851832`。`ActionFailed` timed out 含 value/ref。无 SendInput。
- [x] **CU-D-220**：CI `KEY_DENIED` / `CU-D-220 OK` run `35473210457`。`FocusPolicyViolation` Return gated。无 SendInput。
- [x] **CU-D-230**：CI `TYPE_OK path=clipboard_paste` / `NEWLINE_DENIED` / `CU-D-230 OK` run `35473547564` tab `win:powershell:6700`。无 SendInput；未执行命令。
- [x] **CU-D-240**：CI `ABORT_OK hud=false` / `HUD_GONE` / `ACT_DENIED` / `CU-D-240 OK` run `35473997888`。无 SendInput。
- [x] **CU-D-250**：CI `HOVER_OK path=guide_hover overlay=True os_cursor_used=False` / `GUIDE_FILE_OK x=462 y=347.5` / `CU-D-250 OK` run `35474340766`。无 SendInput。
- [x] **CU-D-260**：CI `HOVER_OK path=guide_hover` / `INVOKE_OK path=bm_click os_cursor_used=False` / `CU-D-260 OK` run `35474689948`。无 SendInput。
- [x] **CU-D-270**：CI `SCOPE_OK` / `BACKEND_OK` / `STAGE_OK` / `CU-D-270 OK` run `35475108584`。doctor warn 不是产品 Windows CU；无 AXPress 假绿。
- [x] **CU-D-280**：CI `TOOLS_OK hover=vcu_hover abort=vcu_session_abort wait.value=true` / `CU-D-280 OK` run `35476413803`。tools/list only。无 SendInput。
- [x] **CU-D-290**：CI `HOVER_OK path=guide_hover os_cursor_used=False via=mcp_tools_call` / `ABORT_OK via=mcp_tools_call` / `CU-D-290 OK` run `35477727787`。无 SendInput。
- [x] **CU-D-300**：CI `INVOKE_OK path=bm_click os_cursor_used=False via=mcp_tools_call` / `CU-D-300 OK` run `35478128021`。无 SendInput。
- [x] **CU-D-310**：CI `WAIT_OK path=scene_wait via=mcp_tools_call` / `CU-D-310 OK` run `35478526783`。无 SendInput。
- [x] **CU-D-320**：CI `TYPE_OK path=wm_settext os_cursor_used=False via=mcp_tools_call` / `CU-D-320 OK` run `35479050203`。无 SendInput。
- [x] **CU-D-330**：CI `NEWLINE_DENIED via=mcp_tools_call` / `CU-D-330 OK` run `35479929439`。无 SendInput。
- [x] **CU-D-340**：CI `SCROLL_OK path=wm_vscroll os_cursor_used=False via=mcp_tools_call` / `CU-D-340 OK` run `35480404968`。无 SendInput。
- [x] **CU-D-350**：CI `EXTRACT_OK count=1 via=mcp_tools_call` / `CU-D-350 OK` run `35481212302`。无 SendInput。
- [x] **CU-D-360**：CI `SHOT_OK mime=image/png via=mcp_tools_call` / `CU-D-360 OK` run `35481626341`。无 SendInput。
- [x] **CU-D-370**：CI `KEY_DENIED via=mcp_tools_call` / `CU-D-370 OK` run `35482034139`。无 SendInput。
- [x] **CU-D-380**：CI `WAIT_MISS_OK via=mcp_tools_call` / `WAIT_REF_MISS_OK via=mcp_tools_call` / `CU-D-380 OK` run `35482614685`。无 SendInput。
- [x] **CU-D-390**：CI `SCOPE_OK via=mcp_tools_call` / `BACKEND_OK via=mcp_tools_call` / `STAGE_OK via=mcp_tools_call` / `CU-D-390 OK` run `35483037087`。无 SendInput。
- [x] **CU-D-400**：真机 `CHROME_HELLO_OK` / `EDGE_HELLO_OK` / `MERGE_OK browser_count=2` / `CU-D-400 OK`。同路径 unpacked 共用 runtime id，client key 为 `browser:id`。组 1/3 未动。无 SendInput。
- [x] **CU-D-410**：真机 health `extension_browsers=["chrome","edge"]` `extension_browser_count=2`；doctor `lens polling chrome+edge`。无占位 client。
- [x] **CU-D-420**：真机 allowlist 含 `Chrome`/`Google Chrome`；observe `app_id=proc:Chrome:*` `allowed=true` 无 AppDenied；`poc_login_state.sh` PASS hud=false；`make check` 0。无 SendInput。
- [x] **CU-D-430**：login-state observe 合并 extension tabs/url；真机 `tabs_source=extension_tabs` tabs=5 `page_url_source=extension_tabs`；不写 `source=extension_dom`；不宣称 AX 网页像素点击。`make check` 0。无 SendInput。
- [x] **CU-D-440**：`POST /v1/browser/observe` 为 CLI/MCP 唯一路径；遍历 user_browsers；标 `tab_id`/`tab_id_source=extension_tabs`；真机 `tab_id` 非空 tabs=6。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-450**：DOM click/type/hover/scroll/extract 无 tab_id 时绑 60s 内 last observe；显式 tab_id 优先；`tab_id_source=last_observe`。真机 body dry-run 绑到 observe tab。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-460**：DOM 动作 `call_timeout_hinted` 按 last observe 的 Chrome/Edge 定向 lens client；tab 未登记时不落到另一浏览器。`browser_hint_targets_client_when_tab_unknown`。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-470**：`browser screenshot`/`capture_tab` 无 tab_id 时绑 last observe；`tab_id_source=last_observe`。真机 screenshot tab 与 observe 相同。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-480**：`open_tab` 按 last observe 的 Chrome/Edge 定向现有窗口；真机 example.com 后台开在 Chrome 后关闭。组 1/3 未动。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-490**：desktop.scene 快照附 `browser_tabs`/`page_url`，不改 `source=ax_scene`、不替换 AX targets；select/close/group 也按 last observe 定向。`desktop_scene_attaches_extension_tabs_for_edge`。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-500**：MCP initialize instructions 改为 ping→observe→60s last observe（click/screenshot/open）；Never CDP Allow。`mcp_initialize_and_session_tools` 断言。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-510**：polling lens 视为 user；next_action 含 observe / last observe；CLI observe/screenshot/open/click help 同步。真机 login-state PASS。不是 TC-B-040。`make check` 0。无 SendInput。
- [x] **CU-D-520**：同一次 tabs chrome=6 edge=7，`browsers_failed=[]`。Edge 新 SW 挂 POST poll；`--reload` 超时则打开 `reload.html`。`poc_cu_d_520.py` OK。不是 TC-B-040。无 SendInput。
- [x] **CU-D-530**：observe 前台 USER Chrome/Edge。真机前台 Edge 时 `app_id=proc:Microsoft_Edge:60318` `frontmost_matched=true`。`poc_cu_d_530.py` OK。不是 TC-B-040。无 SendInput。
- [x] **CU-D-540**：login-state `user_browsers[0]` 为前台浏览器。真机 Edge pid 60318。`poc_cu_d_540.py` OK。不是 TC-B-040。无 SendInput。
- [x] **CU-D-550**：extract 无 tab 时 `tab_id_source=last_observe`。真机 tab 与 observe 相同。`poc_cu_d_550.py` OK。不是 TC-B-040。无 SendInput。
- [x] **CU-D-560**：health click 与 `vcu init` next 为 ping→observe→last observe，不再 tabs-then-screenshot / pixel-only。`poc_cu_d_560.py` OK。无 SendInput。
- [x] **CU-D-570**：lens polling 时 observe 用 extension viewport，不扫 AX。真机 0.49s。`poc_cu_d_570.py` OK。不是 TC-B-040。无 SendInput。
- [x] **CU-D-580**：observe `capture_id` 可 viewport dry-run click。`poc_cu_d_580.py` OK。不是 TC-B-040。无 SendInput。
- [x] **CU-D-590**：`observe --tab` 指定抛页，不抢 OS 前台；live viewport click `#hit` 0→1 后关闭。`poc_cu_d_590.py` OK。组 1/3 未动。不是 TC-B-040。无 SendInput。
- [x] **CU-D-600**：无 `--tab` 的 observe 在前台不是 USER Chrome/Edge 时 `InvalidInput`，要求 `--tab`；不静默绑另一浏览器。`poc_cu_d_600.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-610**：observe `--tab` 后无 `--tab` 的 live type/scroll `tab_id_source=last_observe`，输入可见、scrollY 变化。`poc_cu_d_610.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-620**：observe 后 live hover `hovered=1`；`wait --selector` 等到 DOM 文本，miss 诚实超时。`poc_cu_d_620.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-630**：`observe --browser chrome|edge` 定向 last observe；tab_id 双浏览器撞号须带 `--browser`。`poc_cu_d_630.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-640**：`close`/`select` 接受 `--browser`；撞号无 `--browser` 拒绝；错浏览器 close 为 tab not found。`poc_cu_d_640.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-650**：extract/type/click `--browser` 定向；observe 不再把 Chrome tab 的 last_observe 写成 Edge 进程。`poc_cu_d_650.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-660**：hover/scroll/wait/screenshot `--browser`；撞号无 `--browser` 拒绝。`poc_cu_d_660.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-670**：`open --browser` 定向 Chrome/Edge；group/ungroup 拒绝跨浏览器 tab。`poc_cu_d_670.py` OK。组 1/3 未动。无 SendInput。
- [x] **CU-D-680**：`group-update --browser` 真机通过。`scripts/poc_cu_d_680.py` CU-D-680 OK：组列表带 `browser` 标记、唯一 id 无 `--browser` 正确解析、`--browser` 定向只改对应浏览器组、错浏览器/未知 id 诚实失败且不误改、组清理干净、组 1/3 未动。撞号分支由 `app_http` `upd_amb` 单测覆盖。无 SendInput。同时删除仓库残留 `extension/background.js.bak`。
- [x] **CU-D-690**：extension DOM 支持原生 `<select>`。`type --selector` 接受 `<select>`：按 option value/可见文本匹配，设置后派发 `input`+`change`，回执 `input_path=dom_select`；匹配不到诚实报错且不改值，文本输入仍 `dom_type`。`scripts/poc_cu_d_690.py` CU-D-690 OK；真机验收＝用 VCU 在 GitHub Support 工单页选中「Type of Issue」并提交成功（`.local/desktop-cu/cu-d-690-github-ticket.png`）。
- [x] **CU-D-700**：Release 托管 + `vcu self update` 可用性。已发布 `v0.2.8`（Latest）：`curl | sh` 装到临时前缀成功，`vcu self update`（无 `VCU_BASE_URL`）`updated: true` 且与 Release 包 sha256 一致；失败路径输出 installer stderr + 本地 `file://` 提示。同时修复发布管线两个缺陷（0 字节 `dist/.gitkeep`、Windows 侧 CRLF `install.sh`）。
- [x] **清理**：删除被 git 跟踪的残留备份 `extension/background.js.bak`（2026-09-17 旧 service worker，2460B）。


## 操作入口

```sh
vcu browser ping --json
vcu browser observe --json          # 看 PNG；60s 内无 --tab 的动作绑这次 observe
vcu browser observe --tab <id> --json  # 指定标签，不抢 OS 前台
vcu browser screenshot --json       # 无 --tab 时截 last observe 页
vcu browser click --selector body --dry-run --json
vcu browser open --url https://example.com --background
vcu browser close --tab <id>
```

`source=extension_dom`用于网页动作，`extension_tabs`用于标签管理，`extension_viewport`用于网页截图。DOM是synthetic事件，`trusted=false`；iframe/canvas等需要原生手势的点目标不假报成功。布局绑定上限1000个viewport可见交互目标、5000候选扫描，超限明确拒绝。细节见`playbooks/user-browser.md`。
