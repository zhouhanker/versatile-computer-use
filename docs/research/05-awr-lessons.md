# 可借鉴的 AWR 设计点

来源：[originoneai/agent-work-runtime](https://github.com/originoneai/agent-work-runtime)（本机已用 CLI **0.4.0** 托管本仓库）

## 1. AWR 解决什么

让 Coding Agent **换会话也能接着干**：索引 Markdown/YAML 权威源、检查点、按任务编译有 token 预算的上下文；CLI + MCP；本地、不调模型。

## 2. 映射到 VCU

| AWR 概念 | VCU 可借鉴用法 |
| --- | --- |
| source_first 权威源 | 用户配置、权限策略、技能说明以文件为权威；SQLite 仅投影 |
| `init` 预览再 `--accept` | `vcu init` 先 doctor/预览权限与浏览器，再写入配置 |
| `intake inspect` / organization actions | `vcu doctor` 给出**可执行修复步骤**而非只报错 |
| session + checkpoint + resume | 浏览器长任务：observation 句柄、borrow 状态、open loops 可恢复 |
| `context compile` + budget | 给主 Agent 的 snapshot **默认预算化**（DOM 摘要有上限，防上下文爆炸） |
| claim / revision 乐观并发 | 同一 tab borrow、同一 session 写操作防双写 |
| CLI 与 MCP 同构 | `vcu` 与 `vcu-mcp` 同一 JSON 契约 |
| harness 无关 + 可选 native hooks | Shell 可用即集成；再提供 Skill/MCP |
| 不编造用户意图 | 不静默开启视觉云调用、不静默借用用户标签 |
| 密钥边界 | 配置与证据路径扫描，拒绝把 secret 写进 ledger/日志 |
| 工作台账 | 本仓库继续用 AWR 管研发；VCU 产品内可提供**轻量 session ledger**而非再造完整 AWR |

## 3. 不直接照搬

- VCU 不是“项目工作运行时”，是 **Computer Use 执行运行时**；核心实体是 Session/Target/Observation/Action，不是 Goal/WorkItem（研发过程仍用 AWR）
- AWR 刻意不调模型；VCU **可选**调视觉模型——必须把模型调用做成显式 provider，与核心协议分离
- AWR 的 Markdown 台账对终端用户过重；VCU 用户面保持 `vcu session`/`vcu model` 级简单

## 4. 对本项目流程的约束

本仓库已用 AWR 托管：调研文档与设计文档成为权威输入；实现阶段再拆 work items，用 `awr context compile` 控上下文。
