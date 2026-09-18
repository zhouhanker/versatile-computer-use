# CLI · MCP · Skill 接入设计

版本：0.1-design

## 1. 设计目标

- **最短路径**：Agent 会跑 shell 即可  
- **结构化**：所有子命令支持 `--json`  
- **可诊断**：`vcu doctor` 给出修复动作  
- **可配置视觉**：`vcu init model`

## 2. 命令草图

```text
vcu --version
vcu doctor [--json]
vcu init                      # 创建 ~/.vcu、配对说明、扩展安装引导
vcu init model                # 交互/非交互配置视觉或默认模型
vcu model list
vcu model set <name> [flags]
vcu model test <name>
vcu model set-policy --mode dom_first|vision_first|dom_only|vision_always

vcu daemon start|stop|status

vcu session start [--surface desktop|browser_agent] [--browser chrome|edge|auto] [--json]
vcu session list
vcu session show <id>
vcu session stop <id>
vcu session checkpoint <id>   # 保存 blackboard 摘要
vcu session request-help <id> --reason ...

vcu tabs list --session <id>
vcu tabs borrow --session <id> --tab <tab_id>
vcu tabs return --session <id> [--tab <tab_id>]

vcu snapshot --session <id> [--mode a11y|dom|text|full] [--budget N]
vcu screenshot --session <id> [--full-page] [--out path]
vcu extract --session <id> --selector ... | --plan file
vcu navigate --session <id> --url ...
vcu click --session <id> --ref e1 | --selector ...
vcu type --session <id> --text ... [--ref e1]
vcu act --session <id> --action-json file   # 批动作

vcu agent blackboard --session <id>        # 主/子共享状态
vcu agent spawn-vision --session <id>      # 可选：输出子 agent 提示词/入口

vcu install-skill --harness codex|claude|cursor|pi|generic
vcu mcp print-config                      # 打印 MCP 片段供复制
```

### 2.1 典型 Agent 工作流

```sh
vcu doctor --json
SID=$(vcu session start --surface desktop --json | jq -r .data.session_id)
vcu snapshot --session "$SID" --json          # Scene: AX + 可选截帧
vcu click --session "$SID" --ref a12          # Actuator press，Guide 跟随
vcu session stop "$SID"
```

`desktop`：真窗口，一次辅助功能。`browser_agent`：旧空 profile。会话期间必须有 Stage Banner。

## 3. MCP

`vcu-mcp` 暴露与 CLI 同名工具（snake_case）：

- `vcu_session_start` / `vcu_snapshot` / `vcu_act` / …
- 资源：`blackboard://session/{id}`（可选）

配置示例：

```json
{
  "mcpServers": {
    "vcu": {
      "command": "vcu-mcp",
      "args": ["--user-dir", "~/.vcu"]
    }
  }
}
```

## 4. Skill

每个 harness 一份短 `SKILL.md`：

- 何时用 VCU  
- 必须先 `doctor`  
- session 生命周期  
- 禁止事项（不读 cookie、不关用户窗、不 borrow 除非用户要求）  
- 无视觉时跑 `vcu init model` 的提示  

安装：`vcu install-skill --harness <id>`（对齐 BrowserSkill 的 agent 安装体验）。

## 5. 与 RTK

本仓库 Agent 执行 shell 时仍遵循 RTK（`rtk` 前缀）。  
**VCU 产品 CLI 本身不依赖 RTK**；RTK 是 Codex 侧 token 优化代理。

## 6. 错误码（节选）

| code | 含义 | repair_hint 示例 |
| --- | --- | --- |
| DaemonNotRunning | 守护进程未起 | `vcu daemon start` |
| ExtensionDisconnected | 扩展未连（仅 browser_agent） | 加载扩展或改用 `--surface desktop` |
| AccessibilityDenied | 未授辅助功能 | 系统设置 → 隐私与安全 → 辅助功能 |
| AppDenied | 微信等 denylist | 换目标 App |
| BorrowRequired | 写用户 tab 未借 | `vcu tabs borrow ...` |
| VisionProviderRequired | 需要视觉未配置 | `vcu init model` |
| FocusPolicyViolation | 试图 OS 光标或抢焦点 | 改用页内 action |
| BudgetExceeded | 观察过大 | 提高 budget 或改 mode=a11y |
