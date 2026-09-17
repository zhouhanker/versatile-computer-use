# 跨平台语言与技术栈选型

日期：2026-09-17  
约束：至少 macOS + Windows；浏览器优先；CLI 交付；后续 App 级 CU

## 1. 候选语言对比

| 语言 | 单文件/单二进制 CLI | 浏览器扩展 | CDP/Playwright 生态 | macOS/Windows 原生 UI 自动化 | Agent/MCP 生态 | 团队迭代速度 | 风险 |
| --- | :---: | :---: | :---: | :---: | :---: | :---: | --- |
| **Rust** | 优 | 需 JS 扩展 | 有客户端库，成熟度不如 Node | 优（可可直接链系统 API） | 可用；AWR/Codex CLI 同路线 | 中 | 扩展仍要 TS；早期开发慢 |
| **TypeScript (Node)** | 中（需 Node 或打包） | **最优** | **最优** | 弱（常 shell 出原生桥） | **最优**（MCP npm） | 高 | 运行时依赖；App 期吃力 |
| **Go** | 优 | 需 JS 扩展 | 中 | 中 | 中 | 高 | 浏览器生态薄于 Node |
| **Python** | 差（环境/打包） | 需 JS 扩展 | 强（Playwright/browser-use） | 中 | 强（研究向） | 高 | 不适合“安装即用”系统 CLI |

## 2. 业界实际选择（观察）

- BrowserSkill：公开为 **TypeScript**（CLI/扩展一体思路）
- AWR： **Rust** CLI + MCP
- Codex CLI： **Rust** 为主
- browser-use： **Python**
- 大量 CDP MCP： **Node/TS**

## 3. 决策原则

1. **浏览器 MVP 的关键路径在扩展 + CDP**，扩展侧语言锁定 **TypeScript**
2. **系统级 daemon/CLI 的长期形态**更接近 AWR：单二进制、可签名、少运行时依赖 → **Rust 有利**
3. 用户要求 **方便接入任意 Agent**：CLI 必须“下载即跑”，Windows 上 Python/Node 版本矩阵是实伤
4. 二期 App CU 需要 Accessibility / UI Automation，**Rust/C++/C# 更合适**；纯 TS 会被迫做多桥

## 4. 推荐方案（确定）

### 选定：**Rust 核心 + TypeScript 扩展（混合）**

| 组件 | 语言 | 职责 |
| --- | --- | --- |
| `vcu` CLI | Rust | 初始化、模型配置、session、doctor、执行子命令、JSON 输出 |
| `vcu-daemon` | Rust | 常驻：会话、权限、审计、任务队列、视觉调用编排 |
| `vcu-mcp` | Rust | MCP 工具面（与 CLI 同协议） |
| Browser extension | TypeScript | Chrome/Edge；Agent Window；borrow；DOM/a11y；页内输入 |
| Schema / 协议 | JSON Schema（源） | Observation/Action/Session；生成多语言类型可选 |
| 文档/Skill | Markdown | `SKILL.md`、各 harness 安装说明 |

### 为什么不是纯 TS

- 能更快做出浏览器 MVP，但与“跨平台 Computer Use 产品 + 类 AWR 托管体验”的长期目标不一致
- Windows 上 Node 全局依赖与企业环境限制常见
- App 期仍要 Rust/Go 重写核心，造成二次成本

### 为什么不是纯 Rust

- 扩展必须 JS/TS；强行全 Rust 无增益
- 早期 CDP 探索速度 Node 更快——允许 **adapter 原型用 TS 脚本验证**，稳定后收敛进 daemon 协议，而不是把原型当产品架构

### 降级策略

若一期人力极紧：允许 **TS 实现临时 `vcu` 启动器**，但 **协议与目录布局按 Rust 目标形态设计**，避免 Agent 集成面返工。

## 5. 支撑库方向（设计级，非锁定 crate 名）

- CLI：`clap` 风格解析；全局 `--json`、稳定 exit code
- IPC：daemon 与 CLI 之间 **localhost + 配对 token**（或 named pipe / unix socket）
- 浏览器：扩展 ↔ daemon：**WebSocket/native messaging**
- 可选 CDP 直连：Rust CDP 客户端或由扩展代理 CDP，**避免在产品默认路径暴露裸 9222 给全体本机进程**
- 配置：用户目录 `~/.vcu/` + 项目级 `.vcu/`；密钥不进仓库
- 视觉：HTTP OpenAI-compatible + 可插 provider；本地只存 endpoint/model/key 引用

## 6. 平台基线

| 平台 | 一期 | 说明 |
| --- | --- | --- |
| macOS 13+ (arm64/x64) | 必达 | 开发主平台 |
| Windows 10/11 x64 | 必达 | 安装器/路径/服务模型单独验证 |
| Linux | 二期 | 浏览器扩展路径类似；daemon 打包后做 |

## 7. 结论一句话

**用 TypeScript 做浏览器扩展与页内能力，用 Rust 做跨平台 CLI/daemon/MCP 核心**；协议 JSON 化，Agent 只依赖 CLI/MCP，不依赖实现语言。
