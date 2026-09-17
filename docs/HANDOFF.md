# 会话交接文档（Session Handoff）

> **写入时间：** 2026-09-18 00:42 CST（Asia/Shanghai）  
> **原因：** 用户即将关闭会话并重启 Ghostty，需完整保留上下文以便新会话无缝继续。  
> **仓库：** https://github.com/zhouhanker/versatile-computer-use  
> **本地路径：** `/Users/zhouhan/ai/versatile-computer-use`

---

## 0. 最新用户反馈（2026-09-18，Ghostty 重启前补充）

1. **飞书**：给「张北北」发送 `Test` **未成功**（对方未收到）。上一会话 `osascript` 虽返回 `ok`，但投递失败——需重做飞书发送链路（焦点/会话选择/发送键/辅助功能）。
2. **Edge 远程调试已开启**（用户确认）。下一会话应优先：
   - `vcu browser discover --json` 找到 CDP port
   - `vcu config set-cdp http://127.0.0.1:<port>`
   - 重跑 `scripts/poc_etherscan_labels.sh`，目标 `MODE=TAKEOVER_CDP` + 登录态 label 树

---

## 1. 项目是什么

**VCU（Versatile Computer Use）**：与 Agent 宿主 / 模型厂商解耦的本地 Computer Use 运行时。

- 语言：**Rust** 核心（`vcu` CLI / `vcu-daemon` / `vcu-mcp`）+ **TS** 浏览器扩展
- 浏览器优先（Chrome/Edge），可选 macOS App（AX）
- 不抢 OS 光标；用户 tab 需显式 borrow
- Agent 推荐用 **MCP**（`vcu-mcp` → HTTP daemon）；也可用 CLI

### 硬性约束（用户明确）

1. **禁止**卸载/修改 **Codex Computer Use**（`~/.codex/computer-use/`）
2. **禁止**自动化 **微信 / WeChat**
3. 安装方式应支持 **curl / irm**，不依赖 npm / 源码
4. 远程仓库身份：**zhouhanker**（不是 zhouhan）
5. 优先做完 **macOS** 全能力后再停

---

## 2. 本地安装状态（重启后可直接用）

已安装到用户机器：

| 路径 | 说明 |
|------|------|
| `~/.local/bin/vcu` | CLI 0.1.0 |
| `~/.local/bin/vcu-daemon` | 守护进程 |
| `~/.local/bin/vcu-mcp` | MCP server |
| `~/.local/share/vcu/extension` | Chrome/Edge 扩展包 |
| `~/.vcu/` | 用户配置（pairing token、port 等） |

```bash
export PATH="$HOME/.local/bin:$PATH"
vcu --version          # 期望 0.1.0
vcu self info --json
vcu init               # 若 ~/.vcu 已存在则复用
vcu-daemon &           # 或: vcu service install （LaunchAgent）
vcu doctor --json
```

### 生命周期命令

```bash
vcu self info
vcu self update                          # 重装二进制，默认保留 ~/.vcu
vcu self uninstall --yes                 # 不删 Codex CU
vcu self uninstall --yes --purge-config  # 连 ~/.vcu 删掉
```

### 从本仓库重装（无 GitHub Release 时）

```bash
cd /Users/zhouhan/ai/versatile-computer-use
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist VCU_PREFIX=$HOME/.local bash scripts/install/install.sh
```

### 发布安装（Release 就绪后）

```bash
curl -fsSL https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh | sh
```

---

## 3. Git / CI / 身份

| 项 | 值 |
|----|-----|
| remote | `https://github.com/zhouhanker/versatile-computer-use.git` |
| branch | `main` |
| 最近提交（交接时） | `8ec975a` 及之后可能有 `715a55d` 等；以 `git log -5` 为准 |
| tag | `v0.1.0`（曾 force 更新作者） |
| git user | **必须** `zhouhanker <zhouhanker@gmail.com>` |
| Contributors | API 仅 **zhouhanker**；网页 Insights 可能缓存旧 `zhouhan` |

**曾出的作者问题：**  
- 误用 `user.name=zhouhan`  
- 误用邮箱 `zhouhan@users.noreply.github.com`（会绑定 GitHub 用户 **zhouhan**）  
- 已 filter-branch 重写并 force-push；新会话提交勿再写错。

### CI 打包

- `.github/workflows/ci.yml`：test（mac/linux/win）+ package **macos-arm64/x64** + **windows-x64** artifacts  
- `.github/workflows/release.yml`：tag `v*` 发 Release 资产  
- 文档：`docs/RELEASE.md`、`docs/INSTALL.md`

---

## 4. 架构速查

```
Agent (Codex/Claude/Cursor/…)
  ├─ MCP ─► vcu-mcp ──HTTP+token──► vcu-daemon
  └─ CLI ─► vcu ──────────────────► vcu-daemon
                 ├─ backend mock
                 ├─ backend cdp   (Chrome/Edge remote debugging)
                 ├─ backend extension (Agent Window + borrow)
                 └─ app (macOS AX / Windows stub)
```

- 协议：Observation / Action JSON；`os_cursor=deny`
- MCP：JSON-RPC + Content-Length；**直连 daemon**，不再 shell 出 CLI

---

## 5. 本机第三方 Computer Use（只读参考，勿删）

### Codex Computer Use

- 路径：`~/.codex/computer-use/Codex Computer Use.app`
- 二进制：`SkyComputerUseService`
- 特征：虚拟光标 / Overlay / Accessibility / JSON-RPC socket  
- 分析文档：`docs/research/06-codex-computer-use-and-originone-helper.md`

### OriginOne gpt-bridge computer-helper

- 路径：`~/Library/Application Support/ai.originone.gpt-bridge/`
- `computer-capability/capability.json` → enabled: **false**
- `computer-policy/policy.json` → 按 bundle 允许/拒绝（含微信策略）
- **不要卸载**

---

## 6. 已完成的测试与结果

### 自动化门禁（应保持绿）

```bash
cd /Users/zhouhan/ai/versatile-computer-use
cargo test --workspace
bash scripts/poc_mock_flow.sh
bash scripts/poc_actions_extra.sh
bash scripts/poc_cdp_smoke.sh      # 真 Chrome
bash scripts/poc_cdp_edge.sh       # 真 Edge
bash scripts/poc_app_macos.sh
bash scripts/poc_install_curl.sh
bash scripts/poc_self_lifecycle.sh # update/uninstall，且确认 Codex CU 仍在
```

### 产品场景 POC（2026-09-18）

| 场景 | 脚本 | 结果 | 备注 |
|------|------|------|------|
| 飞书给张北北发 `Test` | `scripts/poc_feishu_message.sh` | `osa_out=ok` 但 **用户确认张北北未收到** | **失败待修**：不能相信 osascript 的 ok；需校验会话/输入框/发送 |
| Etherscan labelcloud | `scripts/poc_etherscan_labels.sh` | 产出在 `~/vcu-etherscan-labels/` | 见下节 |
| WeChat | — | `allowed=false`，无自动化脚本 | 硬禁止 |
| Codex CU | — | uninstall 测试后目录仍在 | 硬禁止删除 |

### Etherscan 关键结论（未完成登录态抓取）

目录：`~/vcu-etherscan-labels/`

| 文件 | 含义 |
|------|------|
| `mode.txt` | 上次为 **`NEW_HEADLESS_NO_LOGIN`** |
| `login_wall.json` | `new_browser_no_user_cookies: true`，`likely_login_required_or_wall: true`，`takeover: false` |
| `labels_flat.json` | 公开 DOM 扁平标签 ~1515 条（**不是**登录后 L1/L2/L3 树） |
| `labels_tree.json` | 说明层级标签依赖登录 UI |
| `snap.json` / `extract.json` | 原始观察 |

**解释：**

- **NEW**：临时 Edge profile + CDP → **没有**用户登录 Cookie  
- **TAKEOVER**：需对**已运行且已开 remote debugging** 的 Chrome/Edge 做 CDP attach → 才能带登录态  

接管步骤（**用户已确认 Edge 远程调试已开启**）：

1. ~~启用 remote debugging~~ **已完成**  
2. `vcu browser discover --json`  
3. `vcu config set-cdp http://127.0.0.1:<port>`  
4. `bash scripts/poc_etherscan_labels.sh`  
5. 期望：`mode=TAKEOVER_CDP` 且能抓登录后标签树  

文档：`docs/macos/BROWSER_TAKEOVER.md`

---

## 7. 未完成 / 下一会话优先任务

按用户原 9 点清单的剩余：

1. ~~卸载/更新命令~~ **已完成**（`vcu self *`）  
2. ~~本地安装~~ **已完成**  
3. 飞书消息 — **失败（张北北未收到）**；需重做发送校验与 UI 流程  
4. Etherscan **登录态 L1/L2/L3** — Edge 远程调试**已开**；优先 discover + TAKEOVER 抓树  
5. ~~逆向 Codex CU / OriginOne~~ **文档已写**（只读）  
6. 测试方法论 — 已有 `docs/testing/METHODOLOGY.md`；可继续网上对标补强  
7. 更全浏览器/App 用例矩阵 — 部分完成，需扩 Safari、多 tab 折叠、虚拟光标 overlay（Codex 风格 UX）  
8. 针对问题逐个修 — 随测试继续  
9. 持续迭代 — 开放  

### 建议的下一会话顺序

1. 确认 PATH 与 `vcu doctor`  
2. Edge remote debugging **已开** → `browser discover` + Etherscan **TAKEOVER**  
3. 实现 labelcloud 层级 DOM 解析 → 稳定写入 `labels_tree.json`  
4. 飞书：**修复发送**（张北北未收到），加成功/失败可观测校验  
5. Codex 风格 UX：Agent 窗/标签管理、可选虚拟光标 overlay（不碰 Codex 安装）  
6. 打正式 GitHub Release 资产（若 CI release 未成功发布）  
7. Windows 包验证与 UIA 深化（macOS 优先项完成后）

---

## 8. 关键代码与文档索引

| 路径 | 内容 |
|------|------|
| `crates/vcu-cli` | CLI（含 self/browser/app/service） |
| `crates/vcu-daemon` / `vcu-server` | HTTP API、CDP/mock/extension/app |
| `crates/vcu-mcp` | MCP stdio |
| `extension/` | MV3 扩展 |
| `scripts/install/install.sh` | curl 安装器 |
| `scripts/install/install.ps1` | irm 安装器 |
| `scripts/pack-release.sh` | 打 tar.gz |
| `scripts/poc_*.sh` | 各类 POC |
| `docs/HANDOFF.md` | **本文档** |
| `docs/INSTALL.md` / `docs/RELEASE.md` | 安装与发布 |
| `docs/design/05-agent-integration.md` | MCP 说明 |
| `docs/macos/FEATURES.md` | macOS 功能矩阵 |
| `docs/macos/BROWSER_TAKEOVER.md` | 接管 vs 新浏览器 |
| `docs/research/06-codex-computer-use-and-originone-helper.md` | 逆向笔记 |
| `docs/testing/METHODOLOGY.md` | 测试方法 |
| `evals/ACCEPTANCE.md` | 验收表 |
| `docs/IDENTITY.md` | 规范作者 zhouhanker |

---

## 9. 新会话给 Agent 的启动提示（可直接粘贴）

```text
继续 VCU 项目：/Users/zhouhan/ai/versatile-computer-use
先读 docs/HANDOFF.md 与 docs/macos/BROWSER_TAKEOVER.md。
硬约束：禁止动 Codex Computer Use；禁止微信自动化；作者必须是 zhouhanker。
本地已安装 vcu 0.1.0 到 ~/.local/bin。
优先：1) Edge 已开 remote debugging → discover + etherscan TAKEOVER + L1/L2/L3；2) 修复飞书（张北北未收到 Test）；3) 持续 macOS 打磨。禁止动 Codex CU / 微信。
```

---

## 10. 已知坑

- `browser discover` 在 async 里不能用 reqwest blocking（已改为 std TcpStream）  
- `app windows` 热加载 allowlist 时不要覆盖 mock backend（测试用）  
- 读 Chrome/Edge `DevToolsActivePort` 可能被 macOS TCC 拒绝；用 `edge://inspect/#remote-debugging` 或端口扫描  
- `vcu self update` 安装器日志需静默，保证 JSON 可 jq  
- Feishu 进程名可能是 `Feishu` 或 `Lark`  
- GitHub Contributors 网页缓存可能短暂仍显示旧作者  

---

**本文档是关闭会话前的权威上下文快照。新会话请以本文件 + git HEAD + `evals/ACCEPTANCE.md` 为准。**
