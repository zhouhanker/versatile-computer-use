# Versatile Computer Use {#vcu status=active}

构建 **厂商/模型/Agent 宿主无关** 的可插拔 Computer Use 运行时（VCU）：在 Codex、Claude Code、Pi、Cursor 等工具中，即使用户不使用其自带模型或内置 CU，也能通过 VCU 获得稳定的计算机操作能力。

首期聚焦 **Chrome/Edge 浏览器**：独立 Agent 工作面、不抢系统光标、可附着已打开浏览器、显式借用用户标签、页面观察与抓取；通过 CLI/MCP/Skill 快速接入；支持 `vcu init model` 配置视觉模型，使无多模态主 Agent 仍可完成需要“看见”的步骤。后续再扩展 macOS/Windows 桌面 App。

**当前版本（2026-09-25）：已发布浏览器 Bridge 0.2.8（Release `v0.2.8`）。桌面切片 CU-D-010…700 已关闭，但仍不是完整 Codex CU，也不是完整 Windows 产品 CU。飞书真发送与深 AX 继续停放。**

Success criteria:

- 需求与方案文档回答：语言选型、浏览器不打断/接管架构、视觉与主子协同、接入面、分期边界。
- 选定跨平台技术栈（至少覆盖 macOS 与 Windows 交付形态）。
- 明确与 BrowserSkill、Codex Browser/CU、CDP 方案的差异化与边界。
- 产品实现前不提前写 runtime 业务代码；设计接受后由台账切开实现项。
- 仓库由 AWR（≥0.4.0）托管；Agent shell 绑定 RTK。

Provenance: 2026-09-17 用户强制初始化需求（RTK/AWR、可插拔 CU、跨平台调研、浏览器优先、CLI 视觉配置、借鉴 AWR）。覆盖此前空目录名推导的泛化目标。

当前迭代目标（2026-09-19 用户明确）：对比 Codex Computer Use，统一虚拟指针视觉与浏览器网页选择，实现参考图中的原生可折叠命名标签组；分阶段修复、验证并记录文档。执行 PARITY-001…005，保持浏览器版边界。

当前收尾（2026-09-25）：浏览器 0.2.8 与 CU-D-700 发布托管已完成。可领取项只剩停放的 MAC-NEXT 与 FEISHU-001，未点名不开。保留 1/3 标签组。用户已重置总额度，旧不足10%提示无效。
