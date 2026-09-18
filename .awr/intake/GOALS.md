# Versatile Computer Use {#vcu status=active}

构建 **厂商/模型/Agent 宿主无关** 的可插拔 Computer Use 运行时（VCU）：在 Codex、Claude Code、Pi、Cursor 等工具中，即使用户不使用其自带模型或内置 CU，也能通过 VCU 获得稳定的计算机操作能力。

首期聚焦 **Chrome/Edge 浏览器**：独立 Agent 工作面、不抢系统光标、可附着已打开浏览器、显式借用用户标签、页面观察与抓取；通过 CLI/MCP/Skill 快速接入；支持 `vcu init model` 配置视觉模型，使无多模态主 Agent 仍可完成需要“看见”的步骤。后续再扩展 macOS/Windows 桌面 App。

**当前版本（2026-09-18）：只做基于浏览器的操作；放弃飞书等桌面 App 产品路径。**

Success criteria:

- 需求与方案文档回答：语言选型、浏览器不打断/接管架构、视觉与主子协同、接入面、分期边界。
- 选定跨平台技术栈（至少覆盖 macOS 与 Windows 交付形态）。
- 明确与 BrowserSkill、Codex Browser/CU、CDP 方案的差异化与边界。
- 产品实现前不提前写 runtime 业务代码；设计接受后由台账切开实现项。
- 仓库由 AWR（≥0.4.0）托管；Agent shell 绑定 RTK。

Provenance: 2026-09-17 用户强制初始化需求（RTK/AWR、可插拔 CU、跨平台调研、浏览器优先、CLI 视觉配置、借鉴 AWR）。覆盖此前空目录名推导的泛化目标。
