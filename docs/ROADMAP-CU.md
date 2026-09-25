# VCU 对标 Codex Computer Use（含桌面）路线图

更新：2026-09-25。作者 zhouhanker。HEAD `e5d47df`。

**排期切片已停在 CU-D-700，下一史诗未开。** 本文保留已关闭切片的执行记录，不是进行中工单。已发布的浏览器 Bridge **0.2.8** 冻结仍有效，见 [`PLAN.md`](PLAN.md)。桌面工作不得回退浏览器门禁，不得把未测桌面能力写成已完成。

测试细则：[`testing/DESKTOP_CU_TEST_PLAN.md`](testing/DESKTOP_CU_TEST_PLAN.md)。结构参照：[`design/06-stage-steward.md`](design/06-stage-steward.md)、[`design/08-codex-cu-parity.md`](design/08-codex-cu-parity.md)。

## 1. 总览

### 要对齐什么

Codex Computer Use 是「长驻助手 + 真窗口 + 辅助功能观察 + 虚拟指针 + 可见叠加层」。VCU 做 **对照官方公开行为的独立实现**（reference implementation）：可以读公开文档、开源实现、以及本机运行时的可见行为，用自有代码/资源复现能力。

这不是把 `~/.codex/computer-use/` 里的私有包（二进制、Lens PNG、AppInstructions 原文）提交进本仓库。那是别人的安装物，不是「参考实现」。

VCU 仍然是 **宿主 / 模型无关** 的本机运行时（CLI / daemon / MCP）。浏览器路径（USER Edge/Chrome + lens）已经可用，作为网页 HTML 的主路径保留。桌面路径补上「任意允许名单内的真窗口」。

目标产品形态：

```text
宿主（Codex CLI / Claude / Cursor / 自建 Agent，Grok 等）
        │  MCP 或 vcu --json
        ▼
   Steward（vcu-daemon）
        ├── Browser Bridge（已有，0.2.8）
        ├── Scene（AX/UIA 树 + 窗口截帧）
        ├── Actuator（AXPress / SetValue / key，不 warp 物理鼠标）
        └── Stage（可见 HUD + Guide 虚拟指针）
                ▼
        用户真窗口：Edge/Chrome、Finder、Notes、TextEdit、终端、飞书客户端…
```

### 参考边界

**可以、应该做的参考**

- OpenAI / Codex **公开** Computer Use 文档与公开仓库
- 本机 Codex CU **运行时可见行为**（光标外形、HUD 结构、AX 流程）——用自有 SVG/AppKit/文案复现
- 公开技术（Accessibility、CGWindow、Chrome tabGroups）
- 仓库根目录 **`/reference/`**（已 gitignore）：本机对照用的 Computer Use 实现。开发时可反编译、拆包、读符号/结构，弄清 overlay、虚拟光标、AX、IPC 怎么串。结论写成自有笔记（`docs/research/`、本路线图），代码用 VCU 自己的 Steward/Stage/Guide/Scene/Actuator 重写。

`/reference/` 使用约定：

- 路径：`reference/computer-use/`（本地，不上传）
- 用途：理解官方实现的架构与行为，指导独立实现
- 反编译/拆包产物只留在本机或 `/reference/` 下，**禁止**把 `.app`、二进制、Lens 序列帧、反编译源码、AppInstructions 原文 commit 进 git
- 不修改用户正在使用的 `~/.codex/computer-use/` 安装；对照副本只放 `/reference/`

**仍然禁止**

| 禁止 | 原因 |
| --- | --- |
| 把 `~/.codex/computer-use/` 的文件拷进 git | 私有安装包，不是公开参考实现 |
| 改 / 卸用户机器上的 Codex CU | 破坏其产品；本仓库也不依赖那份文件 |
| `CGWarpMouseCursorPosition` / HID 当主路径 | 硬约束；Guide ≠ 系统光标 |
| 点击 Edge「允许调试」 | CDP Allow 已抛弃 |
| 自动化微信 | 硬 denylist |
| 把飞书「发送」当自动完成 | 必须用户点名收信人 |
| 把 AX chrome 树冒充 HTML DOM | `source` 必须诚实 |

### 能力分层（对照）

| 层 | 0.2.8 现在 | 本史诗结束后 |
| --- | --- | --- |
| 登录态网页 DOM | 主路径，约七成 Codex 浏览器能力 | 保持并回归，不回退 |
| 桌面真窗口观察 | 列窗 / 截帧 POC | 会话内 Scene 稳定可用 |
| 桌面真点击 / 输入 | AXPress 网页像素未过；POC 默认拒绝 | allowlist App 上 AX 动作诚实成功或诚实失败 |
| 可见性 | 浏览器虚拟光标；桌面 HUD 非默认 | desktop 会话必须有 Stage + Guide |
| 总体 vs 完整 Codex CU | 约 25%–35% | 目标 70%+ 结构对齐（仍无微信、无 HID warp） |

## 2. 编排原则

1. **浏览器不回退。** 每个桌面切片结束都要跑现有 `make check` / Node 扩展测试。
2. **先观察，后动作。** Scene 绿了才能开 Actuator。
3. **先策略，后真机。** 微信拒绝、os_cursor 拒绝、Abort 拆 HUD，必须单测，不依赖真窗口。
4. **网页仍走 extension。** 桌面像素点到 AXWebArea 只能点到整块 WebArea；细按钮继续 `source=extension_dom`。
5. **可见才允许桌面会话。** `surface=desktop` 升起 Stage 失败则拒绝动作。
6. **切片可独立验收。** 每个 CU-D-* 有命令、证据路径、通过/失败标准。
7. **对照 `/reference/`，实现进本仓库。** 桌面切片可先看本地 Computer Use 实现再写 VCU 代码；提交物只能是自有源码与 `assets/` 自绘资源。

## 3. 阶段与工作项

### 阶段 0 — 立项与门禁（本文档）

| ID | 工作 | 完成标准 |
| --- | --- | --- |
| CU-D-000 | 路线图 + 测试计划进仓库；AGENTS 指向下一史诗 | 本文与测试计划已提交；0.2.8 浏览器冻结不被改写为「已含桌面」 |

### 阶段 1 — 桌面会话骨架（约 1 周）

目标：能开始一段**看得见、可中止**的 desktop 会话，只观察、不乱点。

| ID | 工作 | 验收 |
| --- | --- | --- |
| CU-D-010 | `vcu session start --surface desktop` 必须升起 Stage 胶囊 HUD | **完成（单测）** `ErrorCode::StageRequired`；hidden Stage 拒绝；session JSON `stage_hud`；raise 写 `hud:true` |
| CU-D-011 | Abort（默认 Escape 热键）立刻拆 Stage/Guide，会话结束 | **完成（单测/HTTP）** `POST /v1/session/{id}/abort` 与 `vcu session abort`；Escape 写同一 abort 文件。未在用户屏幕上弹 HUD 做 TextEdit 目视 |
| CU-D-012 | `vcu app windows` 只返回 allowlist；微信硬拒绝 | **完成（单测）** `wechat_is_hard_denied` + `denied_app_covering_point` |
| CU-D-013 | 窗口截帧：CGWindowID，不截被挡应用，失败不包装成功 | **单测完成** `observe_does_not_wrap_a_failed_snapshot_in_success` |
| CU-D-014 | Scene：AX 摘要 + screenshot_scale∈{1,2,3} | **单测完成** `pixel_scale_snaps_retina_and_rejects_junk`；doctor 不提 Allow |

**本阶段不做：** 对 Finder/飞书真点击。

### 阶段 2 — Actuator（约 1–2 周）

目标：在 **TextEdit / Notes** 上证明「不搬鼠标也能按下、输入」。

| ID | 工作 | 验收 |
| --- | --- | --- |
| CU-D-020 | AXPress 按 Scene ref；非零错误码如实返回 | **完成（单测）** `ax_ref_press_succeeded`；去掉 System Events `click el` 回退；非 `axpress:0` 失败 |
| CU-D-021 | AXSetValue / 键盘；Return 仍要 confirm_send | **完成（单测）** `desktop_key_policy_gates_return_and_escape`；type 走 `ax_set_value` |
| CU-D-022 | Guide 画在目标 AX 点；`os_cursor_used=false` | **完成（单测）** click/hover/pixel `guide.overlay=true` 且 `os_cursor_used=false` |
| CU-D-023 | 受控真机：TextEdit 输入一行字，Notes 点按钮类控件 | **完成（真机）** TextEdit `e8` AXTextArea 写入标记，`os_cursor_used=false`，`input_path=ax_set_value`；Notes `AXPress` `axpress:0` + Guide overlay。脚本 `scripts/poc_desktop_textedit.py` / `poc_desktop_notes.py`。证据 `.local/desktop-cu/`（不入库） |
| CU-D-024 | 像素 click `space=window`：像素→AX，命中 WebArea 只报 WebArea | **完成（单测）** webview 像素 `hit_ref=e15` 且 `input_path != extension_dom` |

### 阶段 3 — 浏览器 + 桌面统一循环（约 1 周）

目标：宿主一次会话里能「看窗 → 决定走 DOM 还是 AX → 验证」。

| ID | 工作 | 验收 |
| --- | --- | --- |
| CU-D-030 | Observation 带 `surface`、`source`、`login_state` | **完成（单测）** snapshot `surface=desktop` `source=ax_scene` |
| CU-D-031 | 路由：HTTP(S) 页优先 extension_dom；原生控件走 AX | **完成（单测）** USER Edge AXWebArea click 失败并要求 extension_dom |
| CU-D-032 | 浏览器回归：ping / tabs / open 新标签 / screenshot click | **完成（真机）** ping 0.2.8；`source=extension_tabs`；现有窗口新标签（window_id 不变，focused=false）；screenshot `extension_viewport` + capture dry-run `pressed=false`；throwaway DOM click `clicks=1`；组 1/3 未改。脚本 `scripts/poc_cu_d_032.py` |
| CU-D-033 | playbook：`playbooks/desktop.md` 最短环 | **完成** `playbooks/desktop.md` |

### 阶段 4 — 允许名单 App 加宽（按优先级）

每个 App 单独切片，先观察后动作。

| ID | App | 先做 | 不做 |
| --- | --- | --- | --- |
| CU-D-040 | Finder | 列目录窗、选图标、回车打开 | **完成（真机，诚实路径）** Scene `role=CGWindow` 列出脚本自建窗；`act reveal`=`nsworkspace_reveal`；`act open_path`=`nsworkspace_open`；`os_cursor_used=false`。**不是** AXPress 图标，**不是** Return（仍 Send 门禁）。禁止批量删除。脚本 `scripts/poc_desktop_finder.py` |
| CU-D-041 | Terminal / Ghostty | 聚焦、输入、禁盲目 Return | **完成（真机 Terminal.app）** Scene `CGWindow`；`type` 走 `ax_menu_paste`（无换行）；换行与 Return 均为 `FocusPolicyViolation`。未碰 Ghostty。禁止执行命令。脚本 `scripts/poc_desktop_terminal.py` |
| CU-D-042 | 飞书 / Lark **客户端** | 打开已有会话、读可见消息 | **完成（观察+禁发送）** 已打开的客户端窗 Scene `CGWindow` 标题「飞书」；AX 无聊天气泡（Electron）；点「发送」拒绝；Return 门禁。未自动发送。脚本 `scripts/poc_desktop_feishu.py` |
| CU-D-043 | 系统设置 | 只读观察；辅助功能 repair 只给 hint | **完成** Scene 可观察；click/type 拒绝（observe-only）；doctor/AccessibilityDenied hint 指向系统设置且不含 tccutil。未改 TCC。脚本 `scripts/poc_desktop_settings.py` |

微信：**永不进入允许名单。**

### 阶段 5 — 体验对齐（与 Codex 观感）

| ID | 工作 | 说明 |
| --- | --- | --- |
| CU-D-050 | Stage 文案「VCU 正在使用这台 Mac」+ Abort | **完成** native/JXA HUD 文案锁定；真机 `session abort` `hud=false`。无 ChatGPT/Codex 用户文案 |
| CU-D-051 | Guide 与浏览器虚拟光标同一套短三角+柔光 | **完成** `guide_overlay_is_short_dart_with_fog`；click `guide.overlay=true` `os_cursor_used=false`。不宣称官方动画复刻 |
| CU-D-052 | 会话审计：做了什么窗、什么 source | **完成** `VCU_AUDIT=1` 写 `audit.jsonl`（tab/source/surface）；默认关闭 |

### 阶段 6 — Windows（后置）

UIA 列窗 / 截图 / Invoke；同一套 CLI/MCP 契约。macOS 未过门禁不开 Windows 产品切片。

| ID | 工作 | 验收 |
| --- | --- | --- |
| CU-D-060 | UIA 列窗 / 截图 | **完成（CI 真机，非产品会话）** run `35462329205` @ `aa87390`：`UIA_OK` Untitled Notepad children=2；`PRINTWINDOW_OK` PNG。Darwin SKIP。Invoke/SetValue **未** live。不得写成 Windows 产品 CU。 |
| CU-D-070 | Notepad 真写入（无 HID） | **完成（CI 真机，非产品会话）** run `35463098806` @ `1c86e6e`：`SETVALUE_OK path=wm_settext` class=Edit。Server 2022 Notepad Edit 是 `ControlType.Pane`，`GetSupportedPatterns` 为空，**不是** ValuePattern。禁止 SendInput。 |
| CU-D-080 | WindowsAppBackend set_value 回退 WM_SETTEXT | **完成（单测）** `ok:wm_settext` → `input_path=wm_settext` `os_cursor_used=false`。无 SendInput。未接 `vcu session` 真机。 |
| CU-D-090 | 经 `vcu` 的 Windows Stage + Notepad type | **完成（CI 真机）** run `35465535542`：`STAGE_OK presenter=winforms`；`SNAP_OK source=uia_scene ref=e2`；`TYPE_OK path=wm_settext os_cursor_used=False`。无 SendInput。不是完整 Windows CU。 |
| CU-D-100 | 经 `vcu click` 点抛弃型按钮 | **完成（CI 真机）** run `35466230837`：`INVOKE_OK path=bm_click`（非 InvokePattern；按钮是 ControlType.Pane）。无 SendInput。 |
| CU-D-110 | 经 `vcu screenshot` 的 PrintWindow | **完成（CI 真机）** run `35466641180`：`SHOT_OK bytes=10984 width=768 height=519`。无 CopyFromScreen / SendInput。 |
| CU-D-120 | 经 `vcu act open_path` 打开 Explorer 目录 | **完成（CI 真机）** run `35467498771`：`OPEN_OK path=explorer_open`。无 SendInput。 |
| CU-D-130 | 经 `vcu act reveal` 在 Explorer 中选中文件 | **完成（CI 真机）** run `35467970349`：`REVEAL_OK path=explorer_reveal`。无 SendInput。 |
| CU-D-140 | cmd 无换行输入；换行拒绝 | **完成（CI 真机）** run `35469208163` @ `af2fcde`：`SNAP_OK ref=e1`；`TYPE_OK path=clipboard_paste os_cursor_used=False`；`NEWLINE_DENIED`。HWND 经 conhost/AttachConsole，不是 MainWindowHandle。无 SendInput；未执行命令。 |
| CU-D-150 | Windows playbook 最短环 | **完成** `playbooks/desktop.md`：cmd `clipboard_paste`、禁换行、无 SendInput。单测读 playbook。不是产品 Windows CU。 |
| CU-D-160 | Calculator 点击数字（无 HID） | **完成（CI 真机）** run `35470610934` @ `a530de1`：`SNAP_OK ref=e14`；`INVOKE_OK path=bm_click os_cursor_used=False`；tab `win:win32calc:9932`。Server 数字键是 LegacyIAccessible 131，不是 UWP One。无 SendInput。 |
| CU-D-170 | Windows Settings 只读观察 | **完成（CI 真机）** run `35471043411` @ `2400fdc`：`SNAP_OK ref=e1 count=1`；`CLICK_DENIED`；`TYPE_DENIED`；tab `win:SystemSettings:1472`。无 SendInput；未改设置。 |
| CU-D-180 | Notepad 滚动（无 HID） | **完成（CI 真机）** run `35471497693` @ `514153c`：`SCROLL_OK path=wm_vscroll os_cursor_used=False`。不是 ScrollPattern。无 mouse_event / SendInput。 |
| CU-D-190 | Scene 读回 Notepad 值并 extract | **完成（CI 真机）** run `35471859281` @ `5852f9a`：`EXTRACT_OK count=1`。ValuePattern 或 GetWindowText。无 SendInput。 |
| CU-D-200 | wait 直到 Scene value 出现 | **完成（CI 真机）** run `35472199783` @ `0d77f27`：`WAIT_OK path=scene_wait found_ref=e2 os_cursor_used=False`；`CU-D-200 OK`。无 SendInput。 |
| CU-D-210 | wait miss 诚实超时 | **完成（CI 真机）** run `35472851832` @ `cd371d4`：`WAIT_MISS_OK`；`WAIT_REF_MISS_OK`；`ActionFailed` `wait timed out after 800ms` 含 `value=Some("VCU-D-210-MISS")` / `ref=Some("e999")`。无 SendInput。 |
| CU-D-220 | Notepad Return/key 拒绝 | **完成（CI 真机）** run `35473210457` @ `e47cf24`：`KEY_DENIED`；`FocusPolicyViolation` `Return/Enter is gated`；`CU-D-220 OK`。无 SendInput。 |
| CU-D-230 | PowerShell 无换行输入 | **完成（CI 真机）** run `35473547564` @ `5f9ead0`：`TYPE_OK path=clipboard_paste os_cursor_used=False`；`NEWLINE_DENIED`；tab `win:powershell:6700`。无 SendInput；未执行命令。 |
| CU-D-240 | Windows Abort 拆 HUD | **完成（CI 真机）** run `35473997888` @ `ae96695`：`HUD_UP count=1`；`ABORT_OK hud=false`；`LIST_EMPTY`；`ACT_DENIED`；`HUD_GONE`；`STAGE_OK2`；`CU-D-240 OK`。无 SendInput。 |
| CU-D-250 | Windows Guide hover | **完成（CI 真机）** run `35474340766` @ `cf4a803`：`HOVER_OK path=guide_hover overlay=True os_cursor_used=False`；`GUIDE_FILE_OK x=462 y=347.5`；`CU-D-250 OK`。无 SendInput。 |
| CU-D-260 | hover 后 click 仍 bm_click | **完成（CI 真机）** run `35474689948` @ `5348d75`：`HOVER_OK path=guide_hover os_cursor_used=False`；`INVOKE_OK path=bm_click os_cursor_used=False`；`CU-D-260 OK`。无 SendInput。 |
| CU-D-270 | doctor 诚实 Windows 范围 | **完成（CI 真机）** run `35475108584` @ `c40fd89`：`SCOPE_OK`；`BACKEND_OK`；`STAGE_OK`；`CU-D-270 OK`。warn 不是产品会话；无 AXPress 假绿。 |
| CU-D-280 | MCP desktop hover/wait/abort | **完成（CI 真机）** run `35476413803` @ `03f1cc5`：`TOOLS_OK hover=vcu_hover abort=vcu_session_abort wait.value=true`；`CU-D-280 OK`。tools/list；无 SendInput。不是 live tools/call。 |
| CU-D-290 | MCP tools/call live hover | **完成（CI 真机）** run `35477727787` @ `a6feb7f`：`HOVER_OK path=guide_hover os_cursor_used=False via=mcp_tools_call`；`ABORT_OK via=mcp_tools_call`；`CU-D-290 OK`。无 SendInput。 |
| CU-D-300 | MCP tools/call live click | **完成（CI 真机）** run `35478128021` @ `6e2fd82`：`INVOKE_OK path=bm_click os_cursor_used=False via=mcp_tools_call`；`CU-D-300 OK`。无 SendInput。 |
| CU-D-310 | MCP tools/call live wait | **完成（CI 真机）** run `35478526783` @ `8320d9f`：`WAIT_OK path=scene_wait via=mcp_tools_call`；`CU-D-310 OK`。无 SendInput。 |
| CU-D-320 | MCP tools/call live type | **完成（CI 真机）** run `35479050203` @ `720e6d2`：`TYPE_OK path=wm_settext os_cursor_used=False via=mcp_tools_call`；`CU-D-320 OK`。无 SendInput。 |
| CU-D-330 | MCP tools/call live type newline denied | **完成（CI 真机）** run `35479929439` @ `0c9ffcf`：`NEWLINE_DENIED via=mcp_tools_call`；`CU-D-330 OK`。无 SendInput。 |
| CU-D-340 | MCP tools/call live scroll | **完成（CI 真机）** run `35480404968` @ `402e12d`：`SCROLL_OK path=wm_vscroll os_cursor_used=False via=mcp_tools_call`；`CU-D-340 OK`。无 SendInput。 |
| CU-D-350 | MCP tools/call live extract | **完成（CI 真机）** run `35481212302` @ `14525d8`：`EXTRACT_OK count=1 via=mcp_tools_call`；`CU-D-350 OK`。无 SendInput。 |
| CU-D-360 | MCP tools/call live screenshot | **完成（CI 真机）** run `35481626341` @ `37a6a01`：`SHOT_OK mime=image/png via=mcp_tools_call`；`CU-D-360 OK`。无 CopyFromScreen / SendInput。 |
| CU-D-370 | MCP tools/call live key return denied | **完成（CI 真机）** run `35482034139` @ `63ab5b4`：`KEY_DENIED via=mcp_tools_call`；`CU-D-370 OK`。无 SendInput。 |
| CU-D-380 | MCP tools/call live wait miss | **完成（CI 真机）** run `35482614685` @ `27aa24d`：`WAIT_MISS_OK via=mcp_tools_call`；`WAIT_REF_MISS_OK via=mcp_tools_call`；`CU-D-380 OK`。无 SendInput。 |
| CU-D-390 | MCP tools/call live doctor | **完成（CI 真机）** run `35483037087` @ `8304b65`：`SCOPE_OK via=mcp_tools_call`；`BACKEND_OK via=mcp_tools_call`；`STAGE_OK via=mcp_tools_call`；`CU-D-390 OK`。无 SendInput。 |
| CU-D-400 | Dual-browser live lens hello | **完成（真机）** `CHROME_HELLO_OK` / `EDGE_HELLO_OK` / `MERGE_OK browser_count=2` / `CU-D-400 OK`；tabs chrome=5 edge=13；`lens_dual_browser` pass。同路径 unpacked 共用 runtime id，client key=`browser:id`。组 1/3 未动。不是产品 Windows CU。 |
| CU-D-410 | Drop placeholder lens client | **完成（真机）** health `chrome+edge` count=2，无占位 `browser`；doctor `lens polling chrome+edge`。poll 无 browser= 不登记。 |
| CU-D-420 | Allowlist process Chrome | **完成（真机）** observe `app_id=proc:Chrome:*` `target.allowed=true` 无 AppDenied；`poc_login_state.sh` PASS hud=false source=extension_viewport；`make check` 0。无 SendInput。不是产品 Windows CU。 |
| CU-D-430 | Observe merges extension tabs | **完成（真机）** Chrome AX 空树时 `tabs_source=extension_tabs` tabs=5 `page_url_source=extension_tabs`；不覆盖已有 AX tabs；不写 `source=extension_dom`。不是 TC-B-040。无 SendInput。 |
| CU-D-440 | Daemon login-state observe | **完成（真机）** `POST /v1/browser/observe`；CLI/MCP 不再各写一套；`tab_id_source=extension_tabs`；可选 `id` 指定窗。不是产品 Windows CU。无 SendInput。 |
| CU-D-450 | Bind DOM act to last observe | **完成（真机）** 无 tab_id 的 selector 动作绑 60s last observe；显式优先；`tab_id_source=last_observe`。不是跨源 iframe / TC-B-040。无 SendInput。 |
| CU-D-460 | Target lens by observe browser | **完成（真机）** tab 未登记时用 observe 的 chrome/edge client，避免 `wrong_extension_browser`。无 SendInput。 |
| CU-D-470 | Screenshot binds last observe | **完成（真机）** 无 tab_id 的 viewport screenshot 绑 last observe；`tab_id_source=last_observe`。无 SendInput。 |
| CU-D-480 | Open tab in observe browser | **完成（真机）** `open_tab` 定向 last observe 的现有 USER 窗口；抛页关闭。无 SendInput。 |
| CU-D-490 | Desktop scene browser tabs | **完成（HTTP）** Chrome/Edge `desktop.scene` 附 `browser_tabs`，`source` 仍 `ax_scene`；tab 管理命令定向 last observe。无 SendInput。 |
| CU-D-500 | MCP observe-bind instructions | **完成** initialize 指示 observe 后 60s last observe；Never CDP Allow。无 SendInput。 |
| CU-D-510 | login-state next_action observe | **完成（真机）** polling 即 user；next_action ping→observe→last observe。无 SendInput。 |
| CU-D-520 | Dual-browser tabs include Edge | **完成（真机）** 同一次 tabs chrome=6 edge=7；poc CU-D-520 OK。无 SendInput。 |
| CU-D-530 | Observe frontmost USER browser | **完成（真机）** 前台 Edge 时 observe 绑 Edge tab，非 Chrome。无 SendInput。 |
| CU-D-540 | login-state lists frontmost first | **完成（真机）** user_browsers[0] 与前台 Edge 一致。无 SendInput。 |
| CU-D-550 | extract stamps last_observe | **完成（真机）** extract 无 tab 时 `tab_id_source=last_observe`。无 SendInput。 |
| CU-D-560 | health/init observe-bind copy | **完成** health click 与 init next 为 observe-bind。无 SendInput。 |
| CU-D-570 | lens observe without AX walk | **完成（真机）** observe dt=0.49s `tabs_source=extension_tabs`。无 SendInput。 |
| CU-D-580 | observe capture_id viewport click | **完成（真机）** observe capture 可 dry-run viewport click。无 SendInput。 |
| CU-D-590 | observe --tab live viewport click | **完成（真机）** `observe --tab` 激活抛页且不抢 OS 前台；live capture click `#hit` 0→1。`poc_cu_d_590.py` OK。组 1/3 未动。不是 TC-B-040。无 SendInput。 |
| CU-D-600 | observe requires frontmost browser or --tab | **完成（真机）** 前台非 USER Chrome/Edge 时无 `--tab` 的 observe 失败；`--tab` 仍可用。`poc_cu_d_600.py` OK。无 SendInput。 |
| CU-D-610 | observe-bind live type/scroll | **完成（真机）** observe 后 type/scroll 无 `--tab` 绑 last observe；输入 `vcu-d-610`、scrollY=900。`poc_cu_d_610.py` OK。无 SendInput。 |
| CU-D-620 | observe-bind live hover + DOM wait | **完成（真机）** hover `#pad` → hovered=1；wait selector 文本 ready，missing 超时。`poc_cu_d_620.py` OK。无 SendInput。 |
| CU-D-630 | observe --browser Chrome vs Edge | **完成（真机）** `--browser chrome|edge` 打开并 extract 到对应浏览器；撞号须 `--browser`。`poc_cu_d_630.py` OK。无 SendInput。 |
| CU-D-640 | close/select --browser | **完成（真机）** close 错浏览器拒绝；`--browser chrome|edge` 只关对侧抛页。`poc_cu_d_640.py` OK。无 SendInput。 |
| CU-D-650 | extract/type/click --browser | **完成（真机）** `#who` chrome/edge 分向；type Chrome；click Edge dry-run。`poc_cu_d_650.py` OK。无 SendInput。 |
| CU-D-660 | hover/scroll/wait/screenshot --browser | **完成（真机）** hover/scroll Chrome；wait Edge；screenshot 可因遮挡失败。`poc_cu_d_660.py` OK。无 SendInput。 |
| CU-D-670 | open --browser + group same browser | **完成（真机）** open Chrome/Edge；跨浏览器 group 拒绝；同 Chrome 两抛页 group/ungroup。`poc_cu_d_670.py` OK。无 SendInput。 |
| CU-D-680 | group-update `--browser`（撞号定向） | **完成（真机）** 组列表带 `browser` 标记；`--browser chrome\|edge` 只改对应组，另一浏览器组不动；id 唯一时无 `--browser` 正确解析；错浏览器（真实 id）与未知 id 均诚实失败且不误改；组清理干净、组 1/3 未动。撞号分支由 `app_http` `upd_amb` 单测覆盖（真机构造不出同号）。`scripts/poc_cu_d_680.py` CU-D-680 OK。无 SendInput。 |
| CU-D-690 | extension DOM 原生 `<select>` 支持 | **完成（真机）** `type --selector` 现在接受原生 `<select>`：按 option 的 value 或可见文本匹配，设置后派发 `input`+`change`，回执 `input_path=dom_select`；匹配不到诚实报错且不改值；文本输入仍是 `dom_type`。`scripts/poc_cu_d_690.py` CU-D-690 OK；真机验收＝用 VCU 提交 GitHub Support 工单成功。 |
| CU-D-700 | `vcu self update` + Release 托管 | **完成（2026-09-20）** 发布 Release `v0.2.8`（Latest；macOS arm64/x64、Windows x64、`vcu-latest-*`+sha256、install.sh/ps1）：`curl -fsSL .../releases/latest/download/install.sh \| sh` 临时前缀安装成功；不带 `VCU_BASE_URL` 的 `vcu self update` → `updated: true`，装出的 vcu/vcu-daemon/vcu-mcp 与 Release 包 sha256 一致。失败路径输出 installer stderr + 本地 file:// 提示（`crates/vcu-cli/tests/self_update_failure.rs`）。顺带修两个发布缺陷：0 字节 `dist/.gitkeep` 被当资产（publish 失败）、Windows checkout 的 CRLF `install.sh` 覆盖 LF（`curl \| sh` 失败）；工作流修复待下一次 tag 端到端验证。 |

**本轮现场（2026-09-25 收口，功能完成仍是 2026-09-20）：** CU-D-680/690/700 均已完成：真机 POC（`scripts/poc_cu_d_680.py` / `poc_cu_d_690.py`）、发布 Release `v0.2.8` 并验证 `curl | sh` 与 `vcu self update`；仓库残留 `extension/background.js.bak` 已删。停放：MAC-NEXT 深 AX、FEISHU-001、TC-B-040/跨源 iframe。**CI：** run `36129175359`（`9c69b8c`）`test` 三平台与 `package-macos` / `package-windows` 全绿。此前失败是 observe 假 daemon 竞态、Windows 1MB 栈溢出，以及 `self update` 在 Windows 上误调 bash。不是桌面产品回归，也不等于完整 Windows 产品 CU。2026-09-25 只做文档收口，不新开史诗。下一步大纲见 [`PLAN.md`](PLAN.md)。

## 4. 建议执行顺序（编排）

```text
CU-D-000 文档
    → CU-D-010..014 会话+观察
        → 门禁：策略单测 + TextEdit 只观察
    → CU-D-020..024 动作
        → 门禁：TextEdit 真输入 + 浏览器 make check
    → CU-D-030..033 统一循环
        → 门禁：一条 MCP 会话里 DOM 与 AX 分流
    → CU-D-040 Finder（可选并行于 030 之后）
    → CU-D-042 飞书客户端（观察）
    → CU-D-050 HUD 体验
    → CU-D-060 Windows 列窗+截帧（CI）
    → CU-D-070 Notepad 真写入（CI WM_SETTEXT）
    → CU-D-080 backend set_value 回退
    → CU-D-090 vcu Windows set_value
    → CU-D-100 vcu Windows invoke
    → CU-D-110 vcu Windows screenshot
    → CU-D-120 vcu Windows explorer open_path
    → CU-D-130 vcu Windows explorer reveal
    → CU-D-140 vcu Windows cmd type
    → CU-D-150 Windows playbook
    → CU-D-160 vcu Windows calculator click
    → CU-D-170 vcu Windows settings observe-only
    → CU-D-180 vcu Windows notepad scroll
    → CU-D-190 vcu Windows notepad extract
    → CU-D-200 vcu Windows notepad wait value
    → CU-D-210 vcu Windows wait miss timeout
    → CU-D-220 vcu Windows notepad key return denied
    → CU-D-230 vcu Windows powershell type
    → CU-D-240 vcu Windows session abort HUD
    → CU-D-250 vcu Windows guide hover
    → CU-D-260 vcu Windows hover then bm_click
    → CU-D-270 vcu doctor windows scope
    → CU-D-280 MCP desktop hover/wait/abort
    → CU-D-290 MCP tools/call live hover
    → CU-D-300 MCP tools/call live click
    → CU-D-310 MCP tools/call live wait
    → CU-D-320 MCP tools/call live type
    → CU-D-330 MCP tools/call live type newline denied
    → CU-D-340 MCP tools/call live scroll
    → CU-D-350 MCP tools/call live extract
    → CU-D-360 MCP tools/call live screenshot
    → CU-D-370 MCP tools/call live key return denied
    → CU-D-380 MCP tools/call live wait miss
    → CU-D-390 MCP tools/call live doctor
    → CU-D-400 dual-browser live lens hello
    → CU-D-410 drop placeholder lens client
    → CU-D-420 allowlist process Chrome
    → CU-D-430 observe merges extension tabs
    → CU-D-440 daemon login-state observe
    → CU-D-450 bind DOM act to last observe
    → CU-D-460 target lens by observe browser
    → CU-D-470 screenshot binds last observe
    → CU-D-480 open tab in observe browser
    → CU-D-490 desktop scene browser tabs
    → CU-D-500 MCP observe-bind instructions
    → CU-D-510 login-state next_action observe
    → CU-D-520 dual-browser tabs include Edge
    → CU-D-530 observe frontmost USER browser
    → CU-D-540 login-state lists frontmost first
    → CU-D-550 extract stamps last_observe
    → CU-D-560 health/init observe-bind copy
    → CU-D-570 lens observe without AX walk
    → CU-D-580 observe capture_id viewport click
    → CU-D-590 observe --tab live viewport click
    → CU-D-600 observe requires frontmost browser or --tab
    → CU-D-610 observe-bind live type/scroll
    → CU-D-620 observe-bind live hover + DOM wait
    → CU-D-630 observe --browser Chrome vs Edge
    → CU-D-640 close/select --browser
    → CU-D-650 extract/type/click --browser
    → CU-D-660 hover/scroll/wait/screenshot --browser
    → CU-D-670 open --browser + group same browser
    → CU-D-680 group-update --browser（先提交工作树 + 真机 POC + 文档）
    → CU-D-690 extension DOM 设原生 `<select>`（含 GitHub 工单提交真机验收）
    → CU-D-700 Release 托管 + self update 可用性
```

同一时间只 claim 一个 CU-D 主切片。浏览器 bugfix 可并行，但不要和 Actuator 抢同一批真机窗口。

## 5. 怎么测试

权威用例见 [`testing/DESKTOP_CU_TEST_PLAN.md`](testing/DESKTOP_CU_TEST_PLAN.md)。分层：

| 层 | 命令 | 何时必须绿 |
| --- | --- | --- |
| 策略 / 单测 | `cargo test --workspace`；微信、os_cursor、Return 门禁 | 每个 PR |
| 扩展 | `node --test extension/tests/*.test.cjs` | 每个 PR |
| 浏览器冻结 | `make check` | 每个桌面切片结束 |
| 桌面 mock | `cargo test -p vcu-server app::` | 阶段 1 起 |
| 受控真机 | TextEdit / Notes 一次性窗口；脚本可清理 | 阶段 2 起 |
| 禁止 | 操作用户 Edge 组 `1`/`3`；真点微信；代点 Allow | 永远 |

真机默认 **dry-run 先过，再 live**。live 只动脚本自己创建的窗口。

## 6. 成功标准（史诗级）

可以说「对标 Codex CU 含桌面的 VCU 第一版」仅当同时成立：

1. desktop 会话有可见 Stage，Abort 能拆掉  
2. TextEdit 真机：输入可见文字，`os_cursor_used=false`  
3. Finder 至少能打开一个脚本自己建的文件夹窗口  
4. 登录态 Edge 网页仍走 extension_dom，0.2.8 门禁不回退  
5. 微信 / Allow / OS cursor warp / Codex 安装 四条红线测试全绿  
6. README 诚实写清桌面已覆盖与未覆盖，不把 AXWebArea 写成 DOM 点击  

预计：阶段 1–3 完成后，总体相对完整 Codex CU 从约三成升到约 **一半到六成**（仍缺 Windows、飞书真发送、可信手势、官方动画）。阶段 4 后再评估。

## 7. 风险

- AX 对网页内部控件几乎无细粒度 → 必须继续分流到 Browser Bridge  
- 无障碍权限用户没开 → doctor 只给系统设置，不绕过  
- 多屏 / 缩放 → 沿用 screenshot_scale∈{1,2,3}，拒绝乱猜  
- 与用户抢焦点 → 默认不抢；失败才请求前台并审计  
- 范围膨胀 → 微信、HID、抄 Codex 资源永远不进切片  

当前 **0.2.8 不停更浏览器**；桌面是新 surface，不是把浏览器计划作废。
