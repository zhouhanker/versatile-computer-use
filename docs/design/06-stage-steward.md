# Stage + Steward — VCU 桌面主路径设计

版本：0.2-design  
日期：2026-09-18  
状态：**已确认。slice 1–2 已落地。Banner 形态以 `07-stage-hud.md` 为准（胶囊 HUD，禁止通栏）。**  
权威：本文覆盖 `01-architecture` 中「浏览器隔离 profile 为默认」的旧结论。用户已确认；台账 STEW-001。

## 0. 一句话

VCU 的主路径不再是「另开一个空 Edge」。本机长期跑 **Steward**；一次授权 **macOS 辅助功能**；用 **Scene**（AX 树 + 截帧）观察、用 **Actuator**（自有 press/set/key，不搬用户鼠标）操作；用 **Stage** HUD（胶囊，非通栏）+ **Guide** 虚拟指针让人看见 Agent 在干什么。看到的就是用户真实窗口：已登录的 Edge、飞书客户端等。

借鉴的是 Codex Computer Use 的**结构**（长驻助手、AX、叠加层、虚拟指针），**名称与协议全部自有**，不绑 OpenAI 宿主，不碰 `~/.codex/computer-use/`。

## 1. 名称（禁止与 Codex 撞名）

| VCU 名 | 职责 | 不要叫 |
| --- | --- | --- |
| **Steward** | 本机长驻运行时（可由 `vcu-daemon` 演进） | Sky* / ComputerUseService |
| **Stage** | 会话期间可见叠加层 | RecordAndReplay / Lens |
| **Banner** | Stage HUD 文案（顶部居中胶囊，非通栏） | “ChatGPT is using your computer” |
| **Guide** | 虚拟指针（绘制层，不是 OS cursor） | FogCursor / AgentCursor / ComputerUseCursor |
| **Scene** | 一帧观察：AX 摘要 + 可选截图 | Skysight |
| **Actuator** | press / set / key / scroll | 系统键鼠注入为主路径 |
| **Surface** | `desktop`（主）或 `browser_agent`（旁路） | — |

Banner 文案（产品，可 i18n）：**「VCU 正在使用这台 Mac」**，副文案「按 Abort 热键取消」。默认 Abort = Escape（常见取消键，不是在抄产品名）。

## 2. 为什么改主路径

旧默认：独立 Agent Edge profile。优点是不打断、不偷 Cookie。缺点是 **没有用户登录态**——Etherscan / 飞书网页都是「新浏览器」。

Codex 能操作已登录 Edge / 飞书，是因为它走 **系统 UI**，不是 CDP Allow，也不是空 profile。

VCU 要对齐这个能力，同时保持：

- 宿主无关（CLI / MCP / 任意 Agent）
- **不搬用户物理鼠标**（Guide 是 overlay；Hit 走 AX 或坐标命中，不 `CGWarpMouseCursorPosition`）
- 微信硬禁止
- 不修改 Codex CU 安装

## 3. 逻辑架构

```text
 Agent Hosts (Codex / Claude / Cursor / Pi / …)
        │  MCP 或 CLI --json
        ▼
   vcu-mcp / vcu CLI
        │  HTTP + pairing token
        ▼
   Steward  (vcu-daemon 演进，LaunchAgent 长驻)
        │
        ├── Permission / Audit / Session / Blackboard
        ├── Scene builder   (AX 树 + 截帧)
        ├── Actuator        (press/set/key，默认不 warp 鼠标)
        └── Stage presenter (Banner + Guide)
                │
                ▼
        真实用户会话：Edge / 飞书 / 允许名单内 App
```

Steward 是**一个**长驻进程，不是每个动作新连 CDP。辅助功能在系统设置里授一次；之后会话只复用 AX + Stage，不再弹浏览器 Allow debugging。

## 4. 权限与可见性

| 项 | 设计 |
| --- | --- |
| 授权 | macOS「辅助功能」一次；Windows 后期 UIA |
| 会话开始 | 必须升起 Stage HUD（单屏顶部居中胶囊，禁止全宽通栏），否则拒绝 `desktop` surface |
| 会话结束 / Abort | 立刻拆掉 Stage 与 Guide；归还任何临时焦点策略 |
| 焦点 | 默认 **不抢用户键鼠焦点**；若某 App 的 AXPress 无效，才允许「请求前台」并写审计 |
| 用户鼠标 | 禁止作为主路径移动；Guide 只画在 overlay 窗口 |
| 微信 | `denied` 应用，Steward 拒绝 Scene/Actuator |
| Codex CU | 只读调研对象，禁止卸载/改文件 |

`vcu doctor` 在缺辅助功能时给出打开「系统设置 → 隐私与安全 → 辅助功能」的 repair_hint，而不是让用户去点 Edge Allow。

## 5. Scene（观察）

一次 `snapshot` 产出 Scene：

```json
{
  "kind": "desktop.scene",
  "frontmost": {"app": "Microsoft Edge", "window": "…", "pid": 0},
  "ax_summary": "window title=… nodes=…",
  "ax_refs": [
    {"ref": "a12", "role": "AXButton", "name": "Sign in", "frame": [0,0,0,0]}
  ],
  "frame_png_ref": "cas://…",
  "truncated": false,
  "budget_tokens_est": 0
}
```

规则：

1. **AX 优先**（给无视觉主模型）。
2. 浏览器网页内容 AX 经常是一块 WebArea —— 这时 Scene 必须带截帧，可选走已有扩展/CDP 作 **DOM lens**（不是另开空 profile）。
3. 截帧默认本地；云视觉仍要 `vcu init model` 才调用。
4. 不导出 Cookie / 密码框 value。

## 6. Actuator（操作）

| 动作 | 主路径 | 禁止 |
| --- | --- | --- |
| click / press | 目标 `ref` → `AXPress`；无 AX 则 Guide 移到 frame 中心后发 **进程内/AX 命中**，不 warp 用户光标 | 把真正的 OS 指针挪到用户脸上当日常路径 |
| type / set | `AXSetValue` 或焦点元素插入；Return 是否提交由 app 策略表约束 | 盲目 OS 级键盘轰炸 |
| scroll | AX scroll 或滚轮事件打到目标窗 | — |
| screenshot | 窗口/Stage 区域 | 默默全桌面上传云 |

Guide 在 press 前移到目标，纯展示（Codex 式可观测，不叫他们的 cursor 名）。

## 7. Surface 策略

`session start` 增加：

```text
--surface desktop          # 新默认（设计接受后）
--surface browser_agent    # 旧：独立 Agent profile，无用户 Cookie
```

| Surface | 登录态 | 何时用 |
| --- | --- | --- |
| `desktop` | 有（真窗口） | 飞书客户端、已登录 Edge、系统 UI |
| `browser_agent` | 无 | 公开页、不想碰用户窗、CI |

旧 extension/CDP **降为 lens / 旁路**，不再是「日常默认」。若用户以后把扩展装进**自己的** Edge，那是 `desktop` 上的 DOM 增强，不是再开空 profile。

## 8. 应用策略

Allowlist 默认（可配）：Microsoft Edge、Safari、Feishu/Lark、TextEdit、Notes、Finder、Terminal/Ghostty。

Denylist 硬编码：**WeChat / 微信**。Doctor 与 Actuator 双拒绝。

每个 App 可有一份 **操作提示**（自有目录 `~/.vcu/playbooks/<bundle>.md`），例如「飞书输入框用 set 而不是逐键 type 以免误发送」。这是策略文件，不是抄 Codex 的 AppInstructions 包名。

## 9. 与现有实现的关系（确认前只改文档）

已落地且可保留：

- `vcu-daemon` 单实例、pairing、MCP/CLI
- mock / 扩展 / CDP 作为 **browser_agent** 与测试
- `os_cursor=deny` 对物理光标仍然成立；Guide ≠ OS cursor

确认后才实现：

1. Steward 启动时检查辅助功能；Stage overlay 进程/窗口
2. Scene 从 AX 构建 refs
3. Actuator AXPress/AXSetValue
4. `session start --surface desktop`
5. doctor：辅助功能一次授权，而不是 CDP Allow

## 10. 设计验收

- [ ] 主路径是真实桌面 + 一次 AX，而不是空 Edge profile
- [ ] 名称表无 Codex 产品名
- [ ] Banner + Guide 为会话可见不变量
- [ ] 微信拒绝、不碰 Codex CU
- [x] 用户确认本文后，台账才能切开实现项（STEW-001）
