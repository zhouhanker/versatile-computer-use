# VCU 架构方案

版本：0.2-design  
主路径权威：`docs/design/06-stage-steward.md`

## 1. 逻辑架构

```text
 Agent Hosts: Codex · Claude Code · Cursor · Pi · …
 集成面: Shell(CLI) · MCP · Skill.md
                    │
            vcu CLI / vcu-mcp
                    │ HTTP + pairing token
                    ▼
 ┌──────────────────────────────────────────────┐
 │ Steward  (vcu-daemon 演进，LaunchAgent 长驻) │
 │ Session · Audit · Blackboard · Vision        │
 │ Scene (AX + 截帧) · Actuator · Stage/Guide   │
 └───────────────┬──────────────────┬───────────┘
                 │                  │
         Surface=desktop      Surface=browser_agent
         真窗口 + 一次 AX     空 Agent profile（旁路）
         Edge / 飞书 / allowlist    扩展 / CDP / mock
```

## 2. 核心协议（设计级）

所有集成只依赖稳定 JSON，不依赖 Rust/TS 类型。

### 2.1 Session

```json
{
  "session_id": "01...",
  "adapter": "desktop|browser",
  "surface": "desktop|browser_agent",
  "policy": {
    "focus": "no_os_cursor_warp",
    "stage_required": true,
    "os_cursor": "deny"
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

## 5. 桌面主路径（Stage + Steward）

设计接受后，**desktop surface 为一等公民**，不再是「浏览器做完再谈 App」：

- Steward 长驻；辅助功能一次授权
- Scene = AX + 截帧；Actuator = press/set/key，不 warp 用户鼠标
- Stage Banner + Guide 为会话可见不变量
- 微信 denylist；不碰 Codex CU
- 浏览器扩展/CDP 降为 lens / `browser_agent` 旁路

细节只维护在 `docs/design/06-stage-steward.md`。

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
- 默认不移动用户物理光标（Guide 是 overlay）
- pairing token 保护 Steward
- 微信不可 Scene/Actuator
