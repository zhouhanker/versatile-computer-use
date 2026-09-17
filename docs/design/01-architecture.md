# VCU 架构方案

版本：0.1-design

## 1. 逻辑架构

```text
┌─────────────────────────────────────────────────────────┐
│  Agent Hosts: Codex · Claude Code · Cursor · Pi · …     │
│  集成面: Shell(CLI) · MCP · Skill.md                     │
└───────────────────────────┬─────────────────────────────┘
                            │
┌───────────────────────────▼─────────────────────────────┐
│  vcu CLI          vcu-mcp                                │
│  同一 JSON 契约 · 稳定 exit code · --json                │
└───────────────────────────┬─────────────────────────────┘
                            │ IPC (socket + pairing token)
┌───────────────────────────▼─────────────────────────────┐
│  vcu-daemon                                              │
│  · Session manager · Permission · Audit                  │
│  · Perception (DOM/a11y + optional Vision provider)      │
│  · Blackboard / checkpoint                               │
│  · Adapter router                                        │
└───────────────┬───────────────────────────┬─────────────┘
                │                           │
        ┌───────▼────────┐          ┌───────▼────────┐
        │ Browser Adapter│          │ App Adapter    │
        │ Chrome / Edge  │          │ macOS/Windows  │
        │ (一期)         │          │ (二期+)        │
        └───────┬────────┘          └────────────────┘
                │
        ┌───────▼────────┐
        │ TS Extension   │
        │ Agent Window   │
        │ borrow/return  │
        └────────────────┘
```

## 2. 核心协议（设计级）

所有集成只依赖稳定 JSON，不依赖 Rust/TS 类型。

### 2.1 Session

```json
{
  "session_id": "01...",
  "adapter": "browser",
  "browser": "chrome|edge|auto",
  "policy": {
    "focus": "agent_window_only",
    "os_cursor": "deny",
    "borrow_required_for_user_tabs": true
  },
  "vision_policy": "dom_first",
  "created_at": 0
}
```

### 2.2 Observation（预算化）

```json
{
  "observation_id": "01...",
  "session_id": "01...",
  "kind": "browser.snapshot",
  "targets": [{"tab_id": "...", "url": "...", "title": "..."}],
  "a11y_summary": "...",
  "dom_refs": [{"ref": "e1", "role": "button", "name": "OK"}],
  "text_excerpt": "...",
  "screenshot_ref": "cas://sha256/...",
  "vision": {
    "used": false,
    "provider": null,
    "summary": null
  },
  "truncated": false,
  "budget_tokens_est": 0
}
```

### 2.3 Action

```json
{
  "action_id": "01...",
  "session_id": "01...",
  "type": "click|type|navigate|scroll|extract|borrow|return|request_help|...",
  "target": {"ref": "e1"} ,
  "args": {},
  "idempotency_key": "..."
}
```

### 2.4 Result

统一：`ok`、`error.code`、`error.repair_hint`、`session_revision`、`evidence_refs[]`。

## 3. 权限模型

- 安装扩展时最小化 host 权限；敏感 API 运行时再授权
- 域策略：`allow`/`deny` 列表（借鉴 Codex browser requirements 思路）
- borrow 是**写权限**边界；list/snapshot 可更宽但默认可收紧
- daemon 审计日志：本地、可开关、默认不含 DOM 全文

## 4. 视觉管道

见 `docs/research/04-vision-and-multi-agent.md`。  
daemon 内 `VisionProvider` trait/接口：配置来自 `vcu model set`。

## 5. 桌面 App 期（预留）

App adapter 实现同一 Action 子集（click/type 等），但：

- 默认在**虚拟桌面/隔离工作区**或明确允许的应用列表
- 仍尽量避免干扰用户当前键鼠；做不到则 **显式警告 + 确认**
- 不阻塞浏览器 MVP 设计

## 6. 技术栈锁定

| 层 | 选择 |
| --- | --- |
| CLI/daemon/MCP | Rust |
| 扩展 | TypeScript |
| 协议 | JSON Schema 源文件 |
| 配置目录 | `~/.vcu/` 用户级；项目可选 `.vcu/` |
| 研发台账 | 本仓库 AWR |

## 7. 安全底线

- 默认不导出 Cookie/密码
- 默认不静默调用云端视觉
- 默认 OS 光标注入 = deny（browser adapter）
- pairing token 保护 daemon
