# VCU 产品简报

版本：0.1-design  
日期：2026-09-17  
状态：待用户确认后进入实现

## 一句话

**Versatile Computer Use (VCU)** 是本地可插拔的 Computer Use 运行时：与 Agent 宿主、模型厂商解耦，先提供 Chrome/Edge 上不打断用户的浏览器自动化与抓取，再扩展到 macOS/Windows 桌面应用。

## 目标用户

- 使用 Codex / Claude Code / Cursor / Pi 等、但希望 **自选模型** 仍能 Computer Use 的开发者
- 需要 **真实登录态浏览器** 且 **不中断自己操作** 的 Agent 工作流
- 需要把浏览器能力以 **CLI/MCP/Skill** 接到自建 Agent 的团队

## 核心价值主张

1. **解耦**：Computer Use ≠ 某家模型的附赠功能  
2. **不打断**：独立 Agent Window + 禁止 OS 光标抢占（浏览器期）  
3. **可接管**：扩展附着已打开 Chrome/Edge；用户标签显式 borrow/return  
4. **可补视觉**：`vcu init model` 配置视觉提供者；主模型可纯文本  
5. **易接入**：会调 Shell 就能用；可选 MCP 与各 harness Skill  
6. **可演进**：同一 Observation/Action 协议扩展到桌面 App

## 与 BrowserSkill 的关系

BrowserSkill 是强参照与潜在互操作对象（同为 CLI+扩展、不打断）。  
VCU 差异化：统一 CU 协议、视觉 provider、session blackboard/主子协同、桌面 App 路线、AWR 式可恢复会话与预算化观察。

一期实现策略：**协议与集成面自有**；浏览器执行器可 **先适配/学习 BrowserSkill 交互，再自研扩展**（实现阶段再定 make-or-integrate，设计阶段不锁死代码依赖）。

## 成功指标（设计验收，非实现 KPI）

- [ ] 语言与组件边界文档被接受（Rust 核心 + TS 扩展）
- [ ] 浏览器不打断/接管/抓取方案被接受
- [ ] CLI 命令草图含 `vcu init model` 与 session 流
- [ ] 无视觉主模型路径被接受
- [ ] 需求边界与分期清晰
- [ ] AWR 台账反映上述结论并可 `awr ready` 指向下一实现准备项
