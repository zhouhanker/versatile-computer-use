# VCU 当前总计划（浏览器版）

> 写入：2026-09-19 CST  
> 地位：**比 HANDOFF 叠快照更优先**。  
> 作者：zhouhanker

**本版本范围：只做基于浏览器的 Computer Use。放弃所有桌面 App 操作（飞书客户端、Finder、Stage-for-app）。**

宿主 Grok 有视觉，不必 `vcu init model`。

硬约束：不点 Allow；不微信；不 OS 光标；不碰 `~/.codex/computer-use/`；desktop 会话若误开必须 `session stop all`。微信窗口盖住像素时真点必须 `AppDenied`。

---

## 0. 版本边界

| 做 | 不做（本版本） |
| --- | --- |
| 用户 Edge/Chrome 登录态 observe / click / type / scroll / wait / key / ping / extract | 飞书 / Lark **客户端** |
| 像素点击（window/webview）+ Guide | 任意 macOS App AX 产品路径 |
| CSS selector click/type（USER 扩展 DOM，`source=extension_dom`） | CDP TAKEOVER / 点 Allow |
| mock 浏览器 session（navigate/snapshot/click/type/extract/borrow） | 以 App HUD 为验收 |
| MCP：observe 带 PNG；`vcu_browser_ping` / `extract` / selector click/type | lark-cli / osascript 发消息 |
| ping 探测 SW；stale 必须 Reload | AX chrome 冒充 HTML DOM |

飞书 App（FEISHU-001）**停放**。

---

## 1. 用户目标如何落在本版本

1. 登录态浏览器 — **本版本主线**
2. 打磨 CU — 只打磨浏览器路径（Guide×retina、无 HUD observe、Return 门禁、图像传递、DOM extract/click/type）
3. 坐标精度 — 浏览器截图像素 → AX
4. Stage HUD — 本版本不作为验收；误开必须拆掉
5. Codex CU 结构 — 只借鉴浏览器 Scene/Actuator；不抄资源

---

## 2. 执行顺序

| 阶段 | ID | 状态 | 完成定义 |
| --- | --- | --- | --- |
| P0 诚实门禁 | TEST-001 | done | 假绿拆除 |
| P0.5 图像传递 | VISION-001 | 代码 done | `vision_handoff.must_view`；MCP `type=image` |
| P-B 浏览器套件 | BROWSER-TEST-001 | done | 计划 + 用例 TC-B-001…056；cargo / poc-login / poc_mock |
| P-B DOM 协议 | EXTRACT-001 | done | 无扩展失败；假扩展 `extension_dom`；超时不得 AX 假绿 |
| P-B 真机 DOM | EXTRACT-002 | **done** | 真机 ping pong 0.1.5；extract `extension_dom` |
| P-B 登录树 | ETH-001 | ready | `poc_etherscan_labels.sh` 无 CDP；成功须 extension_dom |
| L5 真像素点击 | LOGIN-LIVE / TC-B-040 | 延期，不进 check | 无微信遮挡；`pressed=true`；`os_cursor_used=false` |
| App / 飞书 | FEISHU-001 | **parked** | 本版本不执行 |

---

## 3. 登录态浏览器操作（本版本要做好的面）

| 操作 | CLI | 门禁 |
| --- | --- | --- |
| 观察 | `vcu browser observe --json` | 无 HUD；PNG + `must_view` |
| 像素点击 | `click --pixel-x --pixel-y --space webview [--dry-run] [--guide]` | 无 OS 光标；微信盖住 → AppDenied |
| DOM 点击 | `click --selector 'a' [--dry-run]` | `source=extension_dom` |
| 读/写地址栏 | `type --dry-run` / `type --text` | 默认 AX 地址栏 |
| DOM 输入 | `type --selector 'input' --text x` | `source=extension_dom` |
| 滚动 | `scroll [--dry-run]` | 扩展 polling 时 `source=extension_dom`；否则 AXWebArea |
| 等待 | `wait --role AXWebArea` | 无 HUD |
| Return | `key --key return --dry-run` | 无 confirm+Send ref 必须 blocked |
| 扩展健康 | `ping --json` | 必须 pong；unknown method = Reload |
| DOM 抽取 | `extract --selector a` | 必须 `extension_dom`，禁止 AX 假绿 |

MCP 同名：`vcu_browser_*`。

---

## 4. 测试权威文件

- 计划：`docs/testing/BROWSER_TEST_PLAN.md`
- 用例：`docs/testing/BROWSER_TEST_CASES.md`（TC-B-001 … 056）
- 方法：`docs/testing/METHODOLOGY.md`
- Playbook：`playbooks/user-browser.md`
- 门禁：`make test` + `make poc` + `make poc-login`（**不含** poc-app / poc-feishu / poc-cdp）
- 真机 DOM：`make poc-extract`（不进 check 直到 EXTRACT-002）
- Etherscan：`scripts/poc_etherscan_labels.sh`（禁止 CDP）

通过标准：`cargo test --workspace` 全绿；`poc_mock_flow.sh` PASS；`poc_login_state.sh` PASS 或 SKIP。

---

## 5. 当前节点

EXTRACT-002 真机 DOM 已绿。正在收口 **焦点标签 selector click**（必须回传 tab_id/page_url/focused，默认 last-focused http tab）。ETH-001 样本树次之。不做 App。
