# 需求边界与分期

版本：0.1-design

## 1. 一期（MVP）范围 — 设计接受后实现

**做：**

- Rust CLI + daemon 骨架协议（session/snapshot/act 最小集）
- Chrome + Edge 扩展：Agent Window、list/borrow/return、navigate/click/type、snapshot、screenshot、基础 extract
- `vcu init` / `vcu init model` / `vcu doctor`
- 视觉 provider：至少 OpenAI-compatible HTTP
- 感知策略：`dom_first` 默认
- MCP 最小工具集 + generic Skill
- macOS + Windows 安装说明（扩展 + 二进制）
- 本地审计日志（可关）
- 不抢 OS 光标硬约束（browser adapter）

**不做（一期）：**

- 桌面 App 键鼠接管
- Firefox
- 保证绕过 bot 风控 / 验证码自动破解
- Cookie 导出/注入
- 云端托管浏览器（可列后期）
- 完整多 Agent 框架
- 与 BrowserSkill 的强制依赖（可研究互操作，不阻塞自有扩展）

## 2. 二期

- 抓取增强（分页、滚动加载、网络过滤）
- 更多 harness 一键 skill
- vision providers 扩展（Anthropic、Ollama 等）
- L2 子 Agent 模板与 blackboard UX
- Linux
- 与 AWR session 的可选桥接（长研发任务）

## 3. 三期

- macOS / Windows App adapter（辅助功能 / UI Automation）
- 应用 allowlist、虚拟桌面/后台策略
- 统一 CU 评测集（evals）

## 4. 需求追踪矩阵（用户强制项）

| # | 用户要求 | 落点文档 | 分期 |
| --- | --- | --- | --- |
| 0 | RTK 检查/安装；AWR 更新并托管 | 本初始化过程；AGENTS.md | 已做 |
| 1 | 可插拔 CU，不限工具与模型 | brief + architecture | MVP |
| 2 | macOS+Windows；跨平台语言调研 | research/02 | 语言已定；实现 MVP |
| 3 | 浏览器优先；不打断；接管；抓取；Chrome/Edge；参考 Codex/BrowserSkill/社区 | research/01,03；design/01 | MVP |
| 4 | 其他 Agent 易接入 | design/02 | MVP |
| 5 | 先调研方案；定语言与边界；非多模态要视觉配置 | research/*；design/03 | 本阶段 |
| 6 | CLI 如 `vcu init model`；主/子协同 | research/04；design/02 | MVP(L0/L1)；L2 二期 |
| 7 | 借鉴 AWR | research/05 | 持续 |

## 5. 明确冲突与取舍

| 张力 | 取舍 |
| --- | --- |
| 完全接管用户当前前台标签 vs 不打断 | **默认 Agent Window**；用户标签必须 borrow，且 UI 可见 |
| 最快实现 vs 长期跨平台 CU | **Rust 核心 + TS 扩展**，不选纯 Python |
| 自研扩展 vs 直接依赖 BrowserSkill | 协议自有；实现可先验证互操作，**产品不绑定**其发行节奏 |
| 视觉默认开 vs 隐私 | **默认不调用云视觉**；需配置；DOM 优先 |

## 6. 设计阶段退出条件

当用户确认 `docs/design/*` 与本边界后：

1. AWR 将设计工作标 completed（附 source SHA/证据）
2. 打开实现史诗：`VCU-IMPL-001` 协议 schema 等
3. **才允许**写产品代码

在此之前禁止实现 daemon/extension 业务代码（文档与台账除外）。


## 7. 实现状态（2026-09-17）

一期 MVP **已实现并通过本地 POC 门禁**。详见 `evals/ACCEPTANCE.md`。

二期/三期项（深 UIA、网络抓取、完整多 Agent 框架等）明确不在本期完成定义内。
