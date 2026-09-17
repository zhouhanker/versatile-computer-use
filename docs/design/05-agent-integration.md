# Agent 与 VCU 如何通讯

## 结论

**推荐、一等公民协议是 MCP（Model Context Protocol）**，由 `vcu-mcp` 提供。

| 通道 | 协议 | 适用 |
| --- | --- | --- |
| **MCP（推荐）** | JSON-RPC 2.0 + **Content-Length** stdio 帧 | Codex / Claude Code / Cursor / 其它 MCP 宿主 |
| **CLI** | 子进程 shell 调用 `vcu … --json` | 任意能跑 shell 的 Agent；无 MCP 时的兼容路径 |
| **HTTP** | 本机 `vcu-daemon` REST（pairing token） | 高级集成 / 测试；MCP 与 CLI 都转调它 |

```
Agent Host
   │  MCP tools/call
   ▼
vcu-mcp  ──HTTP+X-Vcu-Token──►  vcu-daemon  ──► mock / cdp / extension / app
```

- `vcu-mcp` **不**再 shell 出 CLI，直连 daemon HTTP。
- 安装为本地二进制，**不依赖 npm**。

## 配置

```bash
vcu mcp print-config --json
```

```json
{
  "mcpServers": {
    "vcu": {
      "command": "vcu-mcp",
      "args": ["--user-dir", "/Users/you/.vcu"]
    }
  }
}
```

## 前置

```bash
vcu init
vcu daemon start   # 或 macOS LaunchAgent
```
