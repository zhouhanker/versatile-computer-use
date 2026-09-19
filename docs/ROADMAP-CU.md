# VCU 对标 Codex Computer Use（含桌面）路线图

更新：2026-09-20。作者 zhouhanker。

**本文是下一史诗的执行总览。** 已发布的浏览器 Bridge **0.2.8** 冻结仍有效，见 [`PLAN.md`](PLAN.md)。桌面工作不得回退浏览器门禁，不得把未测桌面能力写成已完成。

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
| CU-D-032 | 浏览器回归：ping / tabs / open 新标签 / screenshot click | 0.2.8 行为保持 |
| CU-D-033 | playbook：`playbooks/desktop.md` 最短环 | **完成** `playbooks/desktop.md` |

### 阶段 4 — 允许名单 App 加宽（按优先级）

每个 App 单独切片，先观察后动作。

| ID | App | 先做 | 不做 |
| --- | --- | --- | --- |
| CU-D-040 | Finder | 列目录窗、选图标、回车打开 | 批量删除 |
| CU-D-041 | Terminal / Ghostty | 聚焦、输入、禁盲目 Return | 执行未确认的破坏命令 |
| CU-D-042 | 飞书 / Lark **客户端** | 打开已有会话、读可见消息 | 自动发送；必须点名收信人 |
| CU-D-043 | 系统设置 | 只读观察；辅助功能 repair 只给 hint | 自动改权限 |

微信：**永不进入允许名单。**

### 阶段 5 — 体验对齐（与 Codex 观感）

| ID | 工作 | 说明 |
| --- | --- | --- |
| CU-D-050 | Stage 文案「VCU 正在使用这台 Mac」+ Abort | 不使用 ChatGPT/Codex 文案 |
| CU-D-051 | Guide 与浏览器虚拟光标同一套短三角+柔光 | 已有造型；桌面复用；不宣称官方动画复刻 |
| CU-D-052 | 会话审计：做了什么窗、什么 source | 可关 |

### 阶段 6 — Windows（后置）

UIA 列窗 / 截图 / Invoke；同一套 CLI/MCP 契约。macOS 未过门禁不开 Windows 产品切片。

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
    → CU-D-060 Windows（独立史诗）
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
