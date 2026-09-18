# 会话交接文档（Session Handoff）

> **写入时间：** 2026-09-18 22:10 CST（Asia/Shanghai）  
> **权威计划：** [`docs/PLAN.md`](PLAN.md)（先读它，不要再堆第 N 个「当前快照」当计划）  
> **仓库：** https://github.com/zhouhanker/versatile-computer-use  
> **本地路径：** `/Users/zhouhan/ai/versatile-computer-use`  
> **作者：** zhouhanker

---

## 0. 当前节点（2026-09-19 CST）

**本版本放弃 App。总计划：`docs/PLAN.md`（浏览器版）。**

- 用例 TC-B-001…055；cargo 上次 82 passed
- 当前刀：EXTRACT-002 真机 Reload → ping pong → extension_dom
- 登录态已有：observe / 像素 click / selector click/type / ping / extract / Return 门禁 / 微信遮挡拒绝 / 无 CDP Etherscan POC
- FEISHU-001 **parked**

硬约束：不点 Allow；不微信；不 OS 光标。

Live：ping/extract 假绿已拆。真点会先 system-wide 判 pid，微信盖住 → AppDenied。第二屏 Edge 仍被微信挡住时禁止点击。EXTRACT-002 等用户移开微信并 Reload 0.1.5。

---

## 0. 当前权威快照（2026-09-18 21:48 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（CDP-001 + 用户更新）

1. **VCU Browser Bridge 已安装**（用户确认）。`vcu browser login-state` → `extension_profile=user`。
2. **CDP 路径已抛弃**：`login_next_action` 即使看到 Allow 也不再要求用户点击；health 的 `extension_profile` 跟 `likely_user_profile`；doctor 不再把 CDP 当必做下一步。
3. **飞书发 test**：选 黄埔实训营2204A 张北北 `ou_7d592505f6128766a9735a9a5167578d`。`lark-cli im +messages-send --as user` 缺 scope `im:message.send_as_user`，待用户授权后再发并核聊天记录。不要信 osascript ok。
4. 飞书原生任务为空。云空间「项目进度与风险看板」是 IKOTEK 硬件看板，不是 VCU 工程台账。VCU 当前节点以 AWR ledger 为准。

AWR ready：`FEISHU-001`（等 IM 授权）、`ETH-001`（扩展刮登录树）、`MAC-NEXT`。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`；不碰 `~/.codex/computer-use/`。

---


## 0. 当前权威快照（2026-09-18 21:08 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-014）

地址栏 BFS 把 AXWebArea 当叶子、不读 frame。`type --dry-run` **1.19s**（原 2.27s），仍读到 GitHub PRs URL，未写入。

daemon **pid 74558**，sessions **0**。`cargo test` **57**。未点 Allow。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人 + confirm_send。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 21:05 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-013）

`make poc-login` 已断言 type 后 sidecar `page_url` 为 https GitHub PRs。

### 本刀（LOGIN-013 原）

`vcu browser type --dry-run` 读到 `https://github.com/originoneai/agent-work-runtime/pulls`，`login_latest_url_merged=true`，sidecar `page_url` 已更新。未写入地址栏。

daemon **pid 73052**，sessions **0**。`cargo test` **57**。未点 Allow。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人 + confirm_send。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 21:03 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀

`make poc-login` **3.85s PASS**：

- observe hud=false scale=2
- click dry-run+guide ax=(1732,195)
- type 地址和搜索栏（未写入）
- wait AXWebArea fast
- center pixel ax=(2688.0, 635.5) 精确
- key return dry-run **blocked** FocusPolicyViolation
- infobar=false allow_dialog=false 未点 Allow
- sessions 0

快路径 observe 的 `page_url` 可能为空；URL 以 `type --dry-run` 为准。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人 + confirm_send。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 21:00 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-012）

默认 Computer Use 路径改为 **无 HUD 登录态闭环**，不再引导先 `session start`：

- README / `playbooks/user-browser.md` / MCP `instructions`
- `observe → click/type/wait/scroll/key`
- HUD 220×28 仅长任务；Return 门禁写进默认说明
- poc 增加 `key --key return --dry-run` 必须 blocked
- `cargo test` **58**

daemon 以 `vcu daemon status` 为准。未点 Allow。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人 + confirm_send。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:58 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-011）

Codex 式 keypress **不注入 HID**。Return 必须 `confirm_send` + Send ref（防飞书误发）。Esc 不注入（走 HUD abort）。

真机：

- `vcu browser key --key return --dry-run` → blocked FocusPolicyViolation，pressed=false，hid=false，0.01s
- `vcu browser key --key escape --dry-run` → blocked，未注入
- `vcu browser key --key return`（非 dry-run）→ 错误拒绝，未按键

daemon **pid 70793**，sessions **0**。`cargo test` **58**。未点 Allow。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人 + confirm_send。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:55 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-010）

登录态动作不再为找地址栏扫 160 节点。

真机（未写入地址栏、未滚动、未点 Allow）：

| 命令 | 耗时 | 结果 |
| --- | --- | --- |
| login-state | 0.19s | |
| observe | 0.71s | hud=false |
| click --dry-run | 0.03s | ax webview 原点 |
| type --dry-run | **2.27s** | `地址和搜索栏` = GitHub PRs URL，typed=false |
| wait --role AXWebArea | **0.07s** | found=webview |
| scroll --dry-run | 0.04s | scrolled=false |

daemon **pid 69363**，sessions **0**。`cargo test` **57**。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:52 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-009）

`login-state` 曾为探测 Allow 对话框跑 **budget 4000 全树 AX（~8s）**，observe 每次先调它所以也是 8s。

现改为窗口 depth-1 名称（仍能看见 AXSheet/Allow），不点 Allow。

真机：

| 命令 | 耗时 |
| --- | --- |
| `vcu browser login-state` | **0.19s**（原 8.2s） |
| `vcu browser observe` | **0.70s**（原 8.5s），hud=false，scale=2 |
| `vcu browser click --dry-run` | **0.03s**，ax webview 原点 |

daemon **pid 68060**，sessions **0**。`cargo test` **57**。未点 Allow。

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:48 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-008）

- observe `budget<100`：只取窗口 frame/title（~17ms AX）+ 截图；`page_title` 真机 GitHub PR
- 无 AXWebArea 时用 Edge chrome 经验 inset，sidecar `webview_screenshot_frame=[1732,195,1912,881]`（与先前真 AX 一致），`webview_heuristic=true`
- `vcu browser click --dry-run` **0.04s**（不再二次扫 160 节点 AX）
- 完整 AX BFS 仍用于 `budget>=100`（wait/type）
- `cargo test` **57**；daemon **pid 66033**，sessions **0**
- 未点 Allow

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:40 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-007）

- `vcu browser wait --role AXWebArea --ms 3000` → `found_ref=e85`，`hud=false`
- webview 中心像素 dry-run → `ax=(2688.0, 635.5)` 与 `origin + size/2` **完全一致**
- 原点 `(1732,195)` 与中心都对齐；Guide×retina 不是只在 0,0 碰巧对上
- `make poc-login` **PASS**；`cargo test` **57**
- daemon **pid 61014**，sessions **0**；无残留 vcu-stage
- 未点 Allow；未真点击

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:35 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-006）

- `vcu browser click --space webview --dry-run --guide` → `ax=(1732,195)` **Guide 同点**，`hud=false`，闪完无 `vcu-stage`
- `vcu browser type --dry-run` → 地址栏 `ref=e43`，未写入
- `vcu browser scroll --dry-run` → 未滚动
- MCP：`vcu_browser_click`/`type`/`scroll`；control JSON `hud:false`
- HUD 胶囊仍仅 desktop 会话；登录态点击只闪 Guide
- `make poc-login` **PASS**；`cargo test` **57**
- daemon **pid 59366**，sessions **0**
- 未点 Allow；未真点击/输入/滚动用户页

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:28 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-005）

- `vcu browser click --pixel-x --pixel-y --space webview --dry-run`
- 真机：`ax=(1732,195)`（webview 原点），`hit=e84`，`hud=false`，`pressed=false`
- 不升起 Stage；不点 Allow；不搬光标
- MCP `vcu_browser_click`；`login-latest.json` 含 webview frame/scale
- HUD 真机 **220×28**（fitting min 220，HIG keep small）；Finder 会话已 `stop all`，无 vcu-stage
- `make poc-login` **PASS**（observe + dry-run click）；`cargo test` **56**
- daemon **pid 56952**，sessions **0**

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 20:22 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 本刀（LOGIN-004）

USER Edge `pid 60318` 无 HUD 观察：

- `page_url=https://www.bilibili.com/video/BV1iE896REp7/`
- `page_title` 窗口标题；`tabs` 9 个（当前 Bilibili 选中）
- `webview=true` `webview_ref=e85`；页面裁帧 `webview_screenshot_scale=2.0`
- `ax_enhanced=true`（`AXEnhancedUserInterface`，不是 Allow）
- `hud=false`；`login-latest.png` + `.json` 含 page_url
- `make poc-login` **PASS**；`cargo test` **56**
- daemon **pid 54578**，sessions **0**
- 未点 Allow；`allow_dialog=false`；`extension_profile=agent`

### 仍等用户

1. USER Edge Load unpacked `~/.vcu/lens-extension` 才能刮 DOM。
2. 飞书真发送：点名收信人。
3. CDP Allow：用户自己点；Agent 不点。

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

---

## 0. 当前权威快照（2026-09-18 19:55 CST）

宿主 Grok 有视觉，**不必** `vcu init model`。

### 已落地（可自测）

| 能力 | 命令 / 证据 |
| --- | --- |
| 用户 vs Agent Edge | `vcu browser login-state` |
| 无 HUD 观察登录窗 | `vcu browser observe`；`hud=false`；scale=2 |
| 固定截图 | `~/.vcu/captures/login-latest.png` + `.json` |
| 飞书 Scene 不发送 | `make poc-feishu-scene`；`feishu-latest.png` |
| 像素→AX | `vcu click --pixel-x --pixel-y`；Guide overlay |
| Apple HUD | `vcu-stage` 真机 **312×32** hudWindow |
| Codex 结构 | `docs/design/08-codex-cu-parity.md`；desktop Scene |
| 扩展文件 | `vcu browser install-lens` → `~/.vcu/lens-extension`（`lens_copied=true`） |
| 下一步文案 | `vcu browser next` / health `login_next_action` |
| 门禁 | `cargo test` 54；`make poc-login`；`make poc-feishu-scene` |

硬约束：不代点 Allow；不微信；不 OS 光标；desktop 会话必须 `session stop all`。

### 仍等用户

1. **USER Edge** `edge://extensions` Load unpacked `~/.vcu/lens-extension` → `extension_profile=user` 后才能刮 DOM。
2. 飞书真发送：点名收信人 + `confirm_send`。
3. CDP Allow：用户自己点一次；Agent 不点。

daemon：`vcu daemon start`（勿 `service install`）。Shell 前缀 `rtk`。

---


## 0. 续（2026-09-18 19:40 CST）

- Extension hello 上报 `likely_user_profile`（由已打开 tab hostname 判断）。用户 Load unpacked 后 login-state 的 `extension_profile` 可变为 user。
- `cargo test` 51 passed.

## 0. 续（2026-09-18 19:38 CST）

- MCP `vcu_browser_install_lens` + `POST /v1/browser/install-lens`。真机 `extension_profile` 仍为 agent（用户尚未 Load unpacked）。
- `cargo test` 51 passed.

## 0. 续（2026-09-18 19:36 CST）

- `vcu browser install-lens` 复制 7 个文件到 `~/.vcu/lens-extension`，clicked_ui=false。用户需在 USER Edge load unpacked。
- `cargo test` 50 passed.

## 0. 续（2026-09-18 19:34 CST）

- `vcu browser login-state` 现含 `automation_infobar` / `allow_dialog_visible` / `cdp_handshake`。真机 infobar=true，Allow 对话框未出现。未代点。
- `cargo test` 50 passed.

## 0. 续（2026-09-18 19:32 CST）

- MCP `vcu_browser_login_state` / `vcu_browser_observe`。CDP 9222 仍 404/WS timeout（未代点 Allow）。
- `cargo test` 49 passed.

## 0. 续（2026-09-18 19:30 CST）

- `vcu browser observe`：自动用户 Edge、无 HUD。真机 `proc:Microsoft_Edge:60318`，`screenshot_scale=2.0`，selector `github` 1 hit。
- `cargo test` 49 passed。

## 0. 续（2026-09-18 19:28 CST）

- `vcu app snapshot --selector` 无 HUD 抽取。真机用户 Edge `bilibili` **4 hits**，`hud=false`，`browser_profile=user`。
- daemon pid 以 `vcu daemon status` 为准。

## 0. 续（2026-09-18 19:26 CST）

- Desktop `extract` 已从「NotImplemented」改为 Scene AX 过滤（`*` / 名称/角色/ref）。`cargo test` **49 passed**。
- 不代点 Allow；不在用户正在看的 Edge 上开 HUD 抢焦点。
- HUD 真机 312×32；`extension_profile=agent`。

## 0. 续（2026-09-18 19:24 CST）

- HUD 真机 **312×32**（`hudWindow` + 系统符号点 + 11pt），Finder 会话后已 stop，无残留。
- `extension_profile=agent`：扩展仍在空 Agent Edge；doctor 对此 **warn**。
- 用户 Edge 截帧 `screenshot_scale=2.0`，`browser_profile=user`。
- 像素 click → `press_at_point`（AX hit-test，不搬光标）。CLI：`vcu click --pixel-x --pixel-y`。
- `cargo test --workspace`：**48 passed**。daemon 以 `vcu daemon status` 为准。
- ETH-001 / FEISHU-001 仍需用户（不代点 Allow；飞书要点名收信人）。

---


## 0. 会话关闭归档（2026-09-18 19:16 CST）

> 下一会话先读本节 + `.awr/intake/work-ledger.yaml`。Shell 前缀 `rtk`。

### 本回合结果

| ID | 状态 | 要点 |
| --- | --- | --- |
| LOGIN-001 | **completed** | 用户 Edge vs Agent profile；`vcu browser login-state`；doctor `login_browser`；desktop 优先 user Edge |
| STEW-027 | **completed** | 截图像素 → AX 点 → Guide；`pixel_x/pixel_y` + `space=window\|webview` |
| STEW-028 | **completed** | `docs/macos/COORDINATES.md`、`playbooks/user-browser.md`；宿主视觉即可 |
| STEW-029 | **completed** | HUD 300×32，`hudWindow` 材质，11pt，Guide 48pt |
| DESIGN-008 | **completed** | `docs/design/08-codex-cu-parity.md` |

- `cargo test --workspace`：**47 passed**
- 已 rebuild + 安装：`~/.local/bin/{vcu,vcu-daemon,vcu-mcp,vcu-stage}`
- daemon：**pid 23525**，`http://127.0.0.1:17890`，sessions 0
- 真机：user Edge **pid 60318** `browser_profile=user`（Bilibili/GitHub/Grok 登录态标签）。Agent Edge 61307 无 cookies。CDP 9222 在听，handshake 仍可能卡 Allow — **未代点**。
- Finder desktop 会话升起 vcu-stage 后 `session stop all`，helper 已拆。

### 硬约束（不可破）

- 禁止点 UI（Edge Allow、TCC）；禁止微信；禁止动 `~/.codex/computer-use/`
- 禁止 OS 光标 warp；Guide 仅 overlay
- desktop 同回合 `vcu session stop all`
- 禁止 `vcu service install`
- 登录态 ≠ 空 Agent profile

### 仍开放

| ID | 原因 |
| --- | --- |
| ETH-001 | 用户 Edge 当前不是 Etherscan。L1/L2/L3 DOM 仍要用户 Allow 一次或用户自己打开该页。禁止代点 Allow。 |
| FEISHU-001 | 仍要点名收信人；AX 无发送。 |

### 关键新路径

| 路径 | 用途 |
| --- | --- |
| `vcu browser login-state` | user vs agent pid |
| `vcu session start --surface desktop --browser edge` | 附着用户 Edge |
| `docs/macos/COORDINATES.md` | 像素↔点 |
| `docs/design/08-codex-cu-parity.md` | Codex 结构对齐 |
| `helpers/vcu-stage/main.swift` | Apple HUD |

---

# 会话交接文档（Session Handoff）

> **写入时间：** 2026-09-18 17:50 CST（Asia/Shanghai）  
> **原因：** 会话关闭归档（STEW-023–026 已落盘；下一会话直接续）  
> **仓库：** https://github.com/zhouhanker/versatile-computer-use  
> **本地路径：** `/Users/zhouhan/ai/versatile-computer-use`  
> **作者：** zhouhanker

---

## 0. 会话关闭归档（2026-09-18 17:50 CST）

> **下一会话第一件事：读本节 + `.awr/intake/work-ledger.yaml`。**  
> Shell 命令前缀 `rtk`。作者 **zhouhanker**。

### 本回合结果

| ID | 状态 | 要点 |
| --- | --- | --- |
| STEW-023 | **completed** | desktop/MCP screenshot 带 webview 裁帧；`tab_id`；doctor `scene_webview_crop`；PNG IHDR |
| STEW-024 | **completed** | `screenshot_scale` / `webview_scale`（1/2/3）；真机飞书 scale=2 |
| STEW-025 | **completed** | full Scene Observation 带 `screenshot_scale` / `webview_screenshot_scale` |
| STEW-026 | **completed** | MCP `type`/`scroll`/`wait`/`extract` 认 `tab_id` |

- `cargo test --workspace`：**43 passed**
- 已 rebuild + 安装：`~/.local/bin/{vcu,vcu-daemon,vcu-mcp}`
- daemon：**pid 97428**，`http://127.0.0.1:17890`，`sessions: 0`，extension polling
- 无 `vcu-stage` overlay；未 `vcu service install`
- 真机飞书 `proc:Feishu:11666`（`app snapshot --pixels`，无 HUD）：
  - 窗 `1920×1050pt → 3840×2100px` scale **2.0**
  - messenger `e15` `1424×1038pt → 2848×2076px` scale **2.0**

### 硬约束（不可破）

- 禁止点 UI（Edge Allow、TCC 弹窗）；禁止 `CGRequestScreenCaptureAccess`
- 禁止动 `~/.codex/computer-use/`；禁止微信自动化（`AppDenied`）
- 禁止 OS 光标 warp（`OsCursorDenied`）；Guide 仅 overlay；禁止 HID / `click at`
- desktop 会话：同回合 `vcu session stop all`；勿留 HUD/`vcu-stage`
- 禁止 `vcu service install`（本机用手启 daemon）
- 命名：Steward / Stage / Banner / Guide / Scene — 不要 Sky* / FogCursor

### 架构（已锁定）

- **主路径 = desktop**：真实窗口 + 一次 Accessibility 授权
- `browser_agent`（空 Edge + extension/CDP）= 旁路，**无用户 cookies**
- 飞书 compose 是 Electron：**AX 无「发送」/输入框**；路径 = Scene AX + webview crop + `ax_frame_hit`，不信 `osascript ok`
- Retina：AX frame 是点；截屏 PNG 常 2x；用 `pixel_scale` / `screenshot_scale` 映射

### 已完成总表（STEW-001–026）

Runtime：desktop surface、capsule HUD 420×44、Guide overlay、Esc abort、AX BFS（深 13 / Finder 4，cap 80，4.5s）、无 `entire contents`。

Feishu Scene 链：
- 016 live 42 nodes（搜索/消息/MultiWebView）
- 017 `webview=true` `webview_ref=e15`
- 018 click/hit → `ax_frame_hit`，Guide 在 frame 中心
- 019 `playbooks/feishu.md`
- 020 act 认 `tab_id`
- 021 app snapshot `--pixels` 写 `webview_screenshot_*`
- 022 desktop full Observation `webview_screenshot_ref`
- 023–026 screenshot/MCP crop + scale + 全工具 tab_id

另：extension bootstrap Origin allowlist、单实例 flock、WeChat denylist、`confirm_send` Return 门闩。

### 阻塞（需用户）

| ID | 原因 |
| --- | --- |
| **FEISHU-001** | 用户必须点名收信人（如张北北）。AX 无发送。勿信 `osascript ok`。勿对真机 messenger 盲 invoke（抢焦点）。 |
| **ETH-001** | 登录态 L1/L2/L3 需 CDP Allow **或** desktop 附着**用户** Edge。Agent 禁止代点 Allow。空 profile 只能公开标签。 |

台账 ready 可选：`MAC-NEXT`（深 AX / GitHub Releases）。

### 下一会话可自领（无用户阻塞）

1. Guide 坐标与 retina `pixel_scale` 对齐（若视觉点选按像素回推点坐标）
2. Observation/MCP 文档化 scale 字段；doctor 已有 `scene_webview_crop`
3. 测试矩阵 / playbook 扩 Safari 等（不碰微信）
4. **不要** 在无用户明确收信人时做 FEISHU-001；**不要** 代点 CDP Allow

### 关键路径

| 路径 | 用途 |
| --- | --- |
| `docs/HANDOFF.md` | 本交接（权威会话状态） |
| `.awr/intake/work-ledger.yaml` | 台账 STEW/FEISHU/ETH |
| `crates/vcu-server/src/app/macos.rs` | AX + screencapture |
| `crates/vcu-server/src/app/mod.rs` | `webview_hint` / `webview_crop_frame` / `pixel_scale` / `png_ihdr_size` |
| `crates/vcu-server/src/browser/desktop.rs` | desktop Scene/Actuator/screenshot |
| `crates/vcu-server/src/api.rs` | HTTP observation / screenshot / app snapshot |
| `crates/vcu-server/src/doctor.rs` | `scene_webview_crop` |
| `crates/vcu-mcp/src/main.rs` | MCP 工具 tab_id |
| `crates/vcu-core/src/protocol.rs` | Observation scale 字段 |
| `helpers/vcu-stage/main.swift` | Stage HUD |
| `playbooks/feishu.md` | 飞书 compose 路径 |

### 新会话启动提示（可粘贴）

```text
继续 VCU：/Users/zhouhan/ai/versatile-computer-use
先读 docs/HANDOFF.md §0 会话关闭归档 与 .awr/intake/work-ledger.yaml。
硬约束：禁止 Codex CU 目录；禁止微信；禁止代点 UI/CDP Allow；禁止 OS 光标；desktop 同回合 stop。
作者 zhouhanker。daemon 应在 127.0.0.1:17890（若死：vcu daemon start，勿 service install）。
STEW-001–026 已完成。自领无阻塞切片；FEISHU-001/ETH-001 等用户。
Shell 用 rtk 前缀。
```

### 安装与运行快照

```text
bins: ~/.local/bin/{vcu,vcu-daemon,vcu-mcp,vcu-stage}
user-dir: ~/.vcu
endpoint: http://127.0.0.1:17890
daemon pid: 97428 (2026-09-18 17:50 仍存活；以 vcu daemon status 为准)
sessions: 0
vcu-stage: 无
tests: cargo test --workspace → 43 passed
```

---

## 0. 最新进展（2026-09-18 17:46 CST）

### STEW-023–026（本回合）

| ID | 状态 | 说明 |
| --- | --- | --- |
| STEW-023 | 完成 | screenshot/MCP 带 webview 裁帧；`tab_id` |
| STEW-024 | 完成 | `screenshot_scale` / `webview_scale`（点→像素） |
| STEW-025 | 完成 | full Scene Observation 带 scale |
| STEW-026 | 完成 | MCP type/scroll/wait/extract 认 `tab_id` |

真机飞书 `proc:Feishu:11666`（无 HUD）：窗 1920×1050pt → 3840×2100px **scale=2**；messenger `e15` 1424×1038pt → 2848×2076px **scale=2**。
doctor `scene_webview_crop` pass。daemon pid **97428**，sessions 0。`cargo test` **43 passed**。

FEISHU-001 / ETH-001 仍阻塞（需你指定收信人；禁止代点 CDP Allow）。

## 0. 最新进展（2026-09-18 15:19 CST）

### STEW-022 desktop full Scene webview crop

`mode=full` 的 desktop Scene 带 `webview` / `webview_ref` / `webview_screenshot_ref`（cas://）。
mock 飞书 e15 覆盖。本刀未开真机 desktop 会话（避免 HUD）。daemon pid **57256**，sessions 0。`cargo test` **40 passed**。

FEISHU-001 / ETH-001 仍阻塞。

## 0. 最新进展（2026-09-18 15:13 CST）

### STEW-021 messenger 裁帧

`vcu app snapshot --pixels` 在整窗 PNG 之外，按 `webview_ref` frame 再 `screencapture -R`。
真机飞书：窗 1920×1050 / 1.39MB，messenger-chat `e15` 1424×1038 / 853KB。无 overlay。daemon pid **55070**。`cargo test` **40 passed**。

FEISHU-001 / ETH-001 仍阻塞。下一刀可做 desktop full Scene 也带 webview crop。

## 0. 最新进展（2026-09-18 14:53 CST）

### STEW-018 / 019 / 020

- **018** `click`/`hit` webview_ref → `ax_frame_hit`，Guide only，无 CGWarp/HID
- **019** `playbooks/feishu.md` compose 路径
- **020** `act` 认 `target.tab_id` / `args.tab_id`，并更新 active tab
- daemon pid **49514**，sessions 0，无 overlay
- `cargo test --workspace` **40 passed**

未对真机飞书 messenger invoke。FEISHU-001 / ETH-001 仍阻塞。

## 0. 最新进展（2026-09-18 14:52 CST）

### STEW-018 / STEW-019

- `click`/`hit` webview_ref → `ax_frame_hit`（AXPress/click element），Guide 到 frame 中心；禁止 CGWarp / HID / `click at`
- mock 飞书 `e15` + desktop session 测试：`os_cursor_used=false`，微信 `AppDenied`
- playbook：`playbooks/feishu.md` compose 路径（snapshot → webview_ref → hit → pixels；发送仍要用户指定人）
- daemon pid **48890**，`sessions: 0`，无 overlay
- `cargo test --workspace` **40 passed**（既有用例内新增断言）

未对真机飞书 messenger 做 invoke（会抢聊天焦点）。FEISHU-001 / ETH-001 仍阻塞。

下一刀（自领）：desktop `act` 认 `tab_id`，或 Scene pixels 裁 messenger frame。不要点 CDP Allow。

## 0. 最新进展（2026-09-18 14:46 CST）

### 任务编排（本回合）

| ID | 状态 | 说明 |
| --- | --- | --- |
| STEW-017 | **本回合完成** | Scene `webview` / `webview_ref` |
| STEW-018 | ready | webview frame hit，不搬 OS 光标 |
| STEW-019 | ready | 飞书 compose playbook（像素+hit） |
| FEISHU-001 | blocked | 需你指定收信人 |
| ETH-001 | blocked | 禁止代点 CDP Allow |

### STEW-017 Scene webview hint

飞书真机：`webview=true`，`webview_ref=e15`（`MultiWebView - messenger:messenger-chat:default`，1424×1038）。无 overlay。daemon pid **46616**。`cargo test` **40 passed**。

下一刀：STEW-018 frame hit（Guide only，禁止 CGWarp / HID）。

## 0. 最新进展（2026-09-18 14:35 CST）

### STEW-016 AX BFS（snapshot / invoke / set_value 同一访问序）

- 深度 13（Finder 4）、cap 80、4.5s 超时；禁止 `entire contents`
- 叶子（Button/StaticText/Splitter）不展开；每节点最多 16 个子节点
- 真机飞书 `proc:Feishu:11666`：42 节点 / 2.34s，含 `搜索（⌘＋K）`、`消息`、`MultiWebView - messenger`；**无「发送」**（webview）
- 真机 Finder：61 节点 / 3.54s，含搜索栏；未卡死
- 无 overlay；WeChat `AppDenied`
- daemon pid **43182**，`sessions: 0`
- `cargo test --workspace` **39 passed**

下一优先：FEISHU-001 仍需你指定收信人；发消息不能只靠 AX。ETH-001 登录态 CDP 仍禁止代点 Allow。

## 0. 最新进展（2026-09-18 14:22 CST）

### STEW-015 AX description 补空 name

无名控件用 description/help 填 name。真机飞书：`关闭按钮` / `全屏幕按钮` / `最小化按钮` / `ContentsView`（6 节点）。仍无「发送」。无 overlay。daemon pid 37894。`cargo test` **38 passed**。

## 0. 最新进展（2026-09-18 13:46 CST）

### STEW-006 完成：desktop scroll

`act type=scroll` 走 AX（window / splitter scroll area / scrollbar），2.5s 超时，禁止 `entire contents` 以免卡死 Finder。不搬 OS 光标。MCP `vcu_scroll` 桌面可用。

真机 Finder：`result=ok-split-sa`，`os_cursor_used=false`，Guide overlay `(2276.5,205)`，随后 session stop。`cargo test` **31 passed**（含 abort watcher 单测）。daemon pid 以 health 为准。

下一优先仍是飞书张北北（需你指定人）和 Etherscan L3。

### STEW-004 完成：Esc 中止会话

`vcu-stage` 在 Esc（keyCode 53，global+local monitor）或点击胶囊时写入 `*.abort`。Steward 50ms 轮询到文件后 `sessions.remove`，Stage Drop 拆 overlay。

真机：session `01M2SGCE983G3VKN6ZDY2SN9SR` 写入 abort sidecar → `session list` 空、`vcu-stage` 消失、abort 文件被清。未向系统注入真实 Esc（避免抢你的键盘）。`cargo test --workspace` **30 passed**。daemon pid 以 `/v1/health` 为准（曾 22030）。未点 UI / 微信 / Codex CU。

下一优先：飞书张北北可验证发送、Etherscan 登录态 L3。开 desktop 必须同一回合 stop。

### STEW-003 完成：胶囊 HUD

通栏顶栏已换成 **一块 420×44 胶囊**，贴在 Guide 所在屏 `visibleFrame` 顶部居中（菜单栏下方）。Guide 为 56px 自绘圆环+内点，不搬 OS 光标。

真机核验（session `01M2SFW7F0ZDK8B0Q1BVVZ5QST`，随后已 stop）：

- `vcu-stage` HUD 窗口 `Width=420 Height=44 X=2478 Y=38`（CGWindow，不是通栏）
- Guide 窗口 `56×56` at `(2248,177)`，中心 `(2276,205)` = click `e7` AX 中心
- WeChat `AppDenied`；`os_click` `OsCursorDenied`
- `session stop` 后 `pgrep vcu-stage` 空
- `cargo test --workspace` **29 passed**

安装：`~/.local/bin/vcu-stage` + daemon（pid 以 `/v1/health` 为准）。未 `vcu service install`。未点 UI、未碰微信/Codex CU。

下一优先：`STEW-004` Esc abort。飞书/Etherscan 仍延后。开 desktop 会话必须同一回合 stop。

### 为何长任务会停（仍有效）

一轮 abort 后若没写 HANDOFF 就会丢上下文。通栏会逼用户中断。Watchdog 12:00 只停保活脚本。

### 真机截屏

daemon 进程 **已有** 屏幕录制权限（`CGPreflightScreenCaptureAccess=true`），你不用再去系统设置勾。
捕获改为 preflight 门闩，不再靠 `VCU_ALLOW_SCREENCAPTURE=1`。
实测：desktop session 截 Finder → `/tmp/vcu-scene-probe.png` 2296x1462 PNG，随后 session stop（横幅已拆）。
daemon pid 77556。未点 UI、未碰微信/Codex CU。


### Stage + Steward slice 2（完成）

- Guide：press 前把 overlay 指针移到 AX frame 中心；`detail.guide.overlay=true`，**不搬 OS 光标**
- Scene：`dom_refs.frame`；mock `full` snapshot 带 `screenshot_ref`
- 真机 `screencapture` 默认关闭（`VCU_ALLOW_SCREENCAPTURE=1` 才开），避免弹出录屏授权
- `cargo test --workspace`：26 passed
- 禁止点 UI / 微信 / Codex CU

### Stage + Steward slice 1

用户已确认 `docs/design/06-stage-steward.md`。台账 `STEW-001`。

`cargo test --workspace`：**26 passed**。已安装新二进制并重启 daemon（pid 76423）；扩展仍 `extension_polling=true`，未点 UI。

已落地代码（本会话）：

- 协议：`SurfaceKind::{Desktop,BrowserAgent}`、`AdapterKind::Desktop`、`BackendKind::Desktop`、`ErrorCode::{AccessibilityDenied,AppDenied}`
- macOS Actuator：AXPress/AXSetValue，去掉 `VCU_ALLOW_APP_INVOKE` 门闩；微信 denylist 先于 allowlist；Edge/Feishu/Lark 进入默认 allowlist
- `session start --surface desktop`（CLI 无参数默认 desktop；`--backend mock` 仍走 browser_agent）
- Stage Banner：「VCU 正在使用这台 Mac」；mock-app 不弹窗；会话 drop 时拆除
- 旧 extension/CDP 保留为 `browser_agent` 旁路
- 禁止点 UI / 禁止 Codex CU / 禁止微信

### 上一快照（2026-09-18 02:48 CST）

### 已落地并安装

- `POST /v1/extension/bootstrap`：loopback only；**无 Origin 或 chrome/moz/safari-extension Origin**；网页 Origin → 401。
- `GET /v1/health`：`extension_connected` / `extension_polling` / `pid` / `last_poll_age_ms`。
- 单实例 flock + `vcu daemon start` → `already_running`。
- Agent Edge 必须 `open -n -a "Microsoft Edge"`；直接 exec 二进制会秒退。脚本：`scripts/start_agent_edge.sh`。
- 扩展 0.1.4：tab complete 等待、虚拟光标、hover/keypress、extract 限 200。
- LaunchAgent 模板 KeepAlive=false；**今晚未 `vcu service install`**。
- 安装：`~/.local/bin/vcu` / `vcu-daemon`（当前 pid 以 `vcu daemon status` 为准）+ `~/.local/share/vcu/extension` 0.1.4。
- 测试：workspace **20 passed**；evil Origin 401；example.com snapshot OK。
- Etherscan：扩展 Agent Window，L1 **800** href；L3 公开首页约 11 地址/页。登录墙仍在。未点 9222 Allow。
- 飞书：2 个精确「张北北」——dept `123`（几乎无聊天）与 `黄埔实训营2204A`（2025-09 有「我是蒹葭」）。**未发送**。`~/vcu-feishu-poc/decision.json`。
- Watchdog：`scripts/overnight_watchdog.sh` 到 12:00。
- `vcu session stop all` 清理残留 session。
- MCP `vcu_health`；hover 虚拟光标 `#vcu-virtual-cursor`。
- 当前 daemon 以 `curl -sS http://127.0.0.1:17890/v1/health` 为准（重启后 pid 会变）。

### 硬约束

禁止点 UI；禁止 Codex CU；禁止微信；作者 zhouhanker。

### 设计转向（2026-09-18 10:40 CST → 10:55 已确认并开始实现）

主路径改为 **Stage + Steward**（真窗口 + 一次辅助功能 + Banner/Guide），见 `docs/design/06-stage-steward.md`。  
独立 Agent Edge 降为 `browser_agent` 旁路。不碰 Codex CU / 微信。

### 下一优先

1. 飞书张北北发送校验（不信 osascript ok；需你指定 open_id/部门）
2. Etherscan 登录态 L3
3. 打 GitHub Release 仍需用户确认 commit

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
先读 docs/PLAN.md，再读 docs/HANDOFF.md。
硬约束：禁止动 Codex Computer Use；禁止微信；禁止点 Allow；作者 zhouhanker。
当前节点：P0 TEST-001 诚实门禁 → P2 飞书 App+视觉发 test 并截图核验。禁止用 lark-cli 当飞书验收。CDP 已抛弃。
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
