# 竞品与开源格局调研

日期：2026-09-17  
来源：公开文档与仓库（非内部实现审计）

## 1. 对照总表

| 项目 | 形态 | 浏览器策略 | 是否打断用户 | 模型/宿主绑定 | 跨平台 | 对 VCU 的启示 |
| --- | --- | --- | --- | --- | --- | --- |
| [Tencent/BrowserSkill](https://github.com/Tencent/BrowserSkill) | CLI `bsk` + daemon + Chromium 扩展 | 独立 **Agent Window**；用户标签需 **显式 borrow/return** | 设计目标即不打断 | **不绑模型**；任何能调 Shell 的 Agent | macOS/Linux/Windows；Chrome+Edge | **最接近“可插拔浏览器能力”**；VCU 浏览器期应直接对照其交互契约 |
| OpenAI Codex Desktop CU + In-app browser | 桌面 App 内闭环 + 扩展/CDP | 内置浏览器 + 可连真实 Chrome；有 allow/block 域 | Desktop CU 宣传可后台、多 agent 并行不抢用户当前工作 | **强绑定 Codex/OpenAI 生态** | macOS/Windows App | 分层：权限宿主 / 能力接口 / 观察-执行闭环；**不能作为无厂商方案直接依赖** |
| chrome-devtools-mcp / auto-connect | MCP → 本机 Chrome | `chrome://inspect` remote debugging / autoConnect | 共享用户窗口时冲突风险高 | 宿主无关，但能力偏调试 | 视 Chromium | attach 已有浏览器的官方向路径；**安全面大** |
| CDP attach 类（cdp-browser-mcp、local-browser-agent、pi-chrome-cdp） | MCP/CLI → `connectOverCDP` | 挂已有调试端口 | 默认可能操作当前前台标签 | 通常不绑模型 | 好 | 实现简单，但 **Chrome 136+ 对默认配置文件调试限制**、端口暴露、焦点冲突需产品化处理 |
| [browser-use/browser-use](https://github.com/browser-use/browser-use) | Python Agent 框架 | 多自管浏览器 | 视实现 | 常与具体 Agent 编排耦合 | 依赖 Python 环境 | 观察/动作与 DOM grounding 算法可参考，**不适合作为跨 Agent 的系统级 CLI 运行时** |
| [trycua/cua](https://github.com/trycua/cua) | Computer-use driver/fleet | 宣称 extension-free + CDP 页内动作 + 桌面 | 强调不必借物理指针 | 偏训练/评测/驱动 | 跨 OS fleets | **页内动作 vs OS 键鼠** 分层值得借鉴；产品重心不同 |
| Playwright/Puppeteer 直驱 | 库 | 新上下文或 CDP | 可控 | 库级 | 好 | 作为 **adapter 实现细节**，不宜暴露为唯一集成面 |

## 2. BrowserSkill 关键设计（浏览器期主参照）

公开能力要点：

- **双件运行时**：`bsk` CLI/daemon + 浏览器扩展
- **任意 Shell Agent**：不依赖特定模型或 harness；另提供 `install-skill` 适配 Cursor/Claude Code/Codex/Pi 等
- **不打断**：任务在独立可见 Agent Window；动用户标签必须显式借用并归还
- **人机回路**：验证码/登录等可 `request-help`
- **会话**：`bsk session start --json`，后续命令带 `--session`
- **沙箱 Agent**：daemon 放 host，沙箱内 `BSK_AUTO_START=0` + 共享 `BSK_HOME`
- **远程**：扩展主动外连，浏览器机可只装扩展

对 VCU 的含义：

1. “可插拔 + 不打断 + 接管已登录浏览器”在开源侧**已有直接竞品**。
2. VCU 若只做同等浏览器 CLI，差异化不足；必须在以下方向做清边界：
   - **统一 Computer Use 协议**（浏览器 → 桌面 App 同一观察/动作模型）
   - **视觉模型可插拔与主/子协同**（BrowserSkill 不解决主模型无视觉）
   - **MCP + Skill + CLI 一等公民**，以及 AWR 式任务/检查点（长任务）
   - **抓取/结构化抽取**作为一等能力（不只是点选）
   - **明确安全与权限模型**（域策略、会话、borrow、审计）

## 3. Codex 设计可借鉴点（非实现复制）

公开信息归纳：

- Computer Use = **观察 UI → 决策 → 输入 → 再观察** 闭环，而非单次“移鼠标”
- 需要**结构化观察**（窗口、焦点控件、对话框）以从误点击恢复
- 浏览器能力与 CU 同属控制环；**域允许/拒绝列表**、持久审批
- Chrome 扩展路径强调：**同一 profile、同一 cookies/tabs**
- 桌面 CU 强调**后台、不抢用户当前操作**（与“抢系统光标的早期 demo”不同）
- 插件/本地扩展：宿主管权限与生命周期，能力窄接口申请

VCU 映射：

- 协议分层：`Observation` / `Action` / `Session` / `Permission`
- 浏览器期默认 **CDP/扩展页内输入**，OS 键鼠仅 App 期且默认隔离工作面
- 配置面：`allow_origins` / `deny_origins` / 会话级授权

## 4. “接管已打开浏览器”的四条技术路线

| 路线 | 做法 | 优点 | 风险 | VCU 建议 |
| --- | --- | --- | --- | --- |
| A. 扩展注入 + 独立 Agent 窗 | BrowserSkill 模式 | 登录态真；不打断；可 borrow | 需装扩展；实现重 | **浏览器 MVP 主路径** |
| B. 官方 auto-connect / DevTools for agents | 用户打开 remote debugging | 接近官方 | 共享窗口冲突；全 profile 暴露 | 作为高级 attach 选项 |
| C. `--remote-debugging-port` + CDP | 启动或挂端口 | 实现快 | Chrome 默认 profile 限制；本地端口任意进程可连 | **开发/自动化 profile** 或受控端口 + 配对 |
| D. 复制 Cookie/存储到新配置 | 状态拷贝 | 隔离好 | 脆弱、敏感、易违规 | **默认不做**；仅用户显式导出场景 |

结论：MVP 采用 **A 为主，C 为无扩展降级/CI**，B 为可选；D 默认禁止。

## 5. 抓取能力格局

- DOM/a11y tree 快照 + 选择器/role 定位：自动化主流
- `evaluate` JS 只读抽取：灵活，需权限门
- 可读性/主内容抽取（Readability 类）：适合“总结页面”
- 网络层 HAR/响应拦截：强，但隐私与复杂度高——**二期**
- 全页截图 + 视觉：补 DOM 失败；依赖视觉模型

MVP 抓取：`snapshot(dom|a11y|text|markdown)` + `extract(selector|predefined)` + 可选 `screenshot`。

## 6. 差异化定位（调研结论）

**VCU = 面向任意 Agent 的本地 Computer Use Runtime**：

```text
任意 Agent (Shell/MCP/Skill)
        ↓
   vcu CLI / vcu-mcp / daemon
        ↓
  Session · Permission · Vision hook
        ↓
  Browser adapter (Chrome/Edge)  →  未来 App adapter (macOS/Windows)
```

一句话：BrowserSkill 解决“浏览器技能”；VCU 解决“**与模型/宿主解耦的 Computer Use 运行时 + 视觉补全 + 向桌面扩展的同一协议**”。
