# Versatile Computer Use (VCU)

**厂商与模型无关、可插拔的 Computer Use 运行时**（Rust CLI/daemon + Chromium 扩展）。

## 状态

- [x] 浏览器 MVP：mock 全链路 + **真实 Chrome CDP 冒烟** + 扩展 RPC
- [x] 不抢 OS 光标 / 用户 tab borrow / 视觉模型可插拔
- [x] CLI · MCP · Skill
- [x] macOS App CU 最小 POC（list/snapshot；focus/invoke 默认拒绝）
- [x] GitHub Actions：macOS/Ubuntu/Windows CI 工作流
- [ ] Windows UIA App adapter（需 Windows runner）

## 安装（用户机器，无需 npm / 无需拉源码）

```bash
# 发布后:
# curl -fsSL https://<release-host>/install.sh | sh
# 本地打包验证:
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist bash scripts/install/install.sh
```

Windows: `irm <host>/install.ps1 | iex`

Agent 默认通过 **MCP**（`vcu-mcp`）调用；也可用 CLI。见 `docs/design/05-agent-integration.md`。

## 快速开始（开发）


```sh
cargo build
./target/debug/vcu init
./target/debug/vcu-daemon &          # 或: vcu daemon start --foreground

# mock（无浏览器）
./target/debug/vcu session start --backend mock --json
make poc

# 真实 Chrome/Edge CDP（脚本可自动拉起 headless Chrome）
make poc-cdp
```

### 视觉模型

```sh
vcu init model --name vision   --base-url https://api.openai.com/v1   --model gpt-4o-mini   --api-key-env VCU_VISION_API_KEY
export VCU_VISION_API_KEY=...
vcu model test vision

# 离线 mock 视觉（测试用，不访问网络）
vcu model set vision --provider mock --base-url http://mock.local --model mock-vl --api-key-env VCU_UNUSED
vcu model set-policy --mode vision_always
```

### CDP URL

```sh
vcu config set-cdp http://127.0.0.1:9333
```

### 扩展（Chrome/Edge）

见 [extension/README.md](extension/README.md)。配对 `~/.vcu/config.json` 的 `pairing_token`。

### 安装到 ~/.local/bin

```sh
bash scripts/install.sh
```

## 测试门禁

```sh
make check   # test + mock poc + cdp poc + release build
```

## 架构摘要

| 组件 | 职责 |
| --- | --- |
| `vcu` | CLI |
| `vcu-daemon` | 本地 HTTP IPC（pairing token） |
| `vcu-mcp` | MCP stdio |
| backends | `mock` / `cdp` / `extension` |
| `extension/` | MV3 Agent Window，不抢焦点 |

硬约束：`os_cursor=deny`；用户 tab 写入前必须 `tabs borrow`。

## 文档

- **会话交接（必读）：** [`docs/HANDOFF.md`](docs/HANDOFF.md)

- 安装：`docs/INSTALL.md`
- 验收矩阵：`evals/ACCEPTANCE.md`

- 设计：`docs/design/`
- 调研：`docs/research/`
- AWR：`.awr/intake/`
