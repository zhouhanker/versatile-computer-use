# 浏览器 Computer Use 测试计划

> 版本：本版本（放弃 App）  
> 日期：2026-09-18  
> 作者：zhouhanker

## 1. 目标

证明 **Chrome/Edge 登录态 + mock 浏览器 session** 的观察、映射、动作、门禁、图像传递、DOM extract 是可重复的。  
**不证明** 飞书客户端、任意桌面 App、CDP Allow 接管。

## 2. 范围

### In

- `vcu browser login-state / observe / click / type / scroll / wait / key / ping / extract`
- pixel click 与 CSS selector click/type（`source=extension_dom`）
- 微信窗口盖住像素 → AppDenied
- mock `session start --backend mock`：navigate、snapshot、click、type、extract、scroll、wait、tabs borrow
- 像素 → AX（scale 1/2/3）
- Return/Enter 门禁（无 HID、无盲发）
- `os_cursor=deny`
- 用户 tab 必须 borrow
- Agent 空 profile ≠ 登录态
- CDP 文案不得要求点 Allow
- `vision_handoff.must_view` + MCP image content
- extract 必须 `source=extension_dom`；超时不得 `ok`+AX chrome
- ping 必须 pong；unknown method ping = stale SW，要 Reload

### Out

- 飞书 / WeChat / Finder 等 App
- `poc_feishu_*`、`poc_app_macos` 不进本版本 `make check`
- CDP smoke 不进本版本必过门禁
- AX Scene 冒充 HTML DOM

## 3. 层级与门禁

| 层 | 入口 | 频率 |
| --- | --- | --- |
| L1 单元 | `cargo test -p vcu-server --lib` | 每次提交 |
| L2 集成 | `cargo test --workspace` | 每次提交 |
| L3 mock 浏览器 POC | `make poc`（`poc_mock_flow.sh`） | 每次 check |
| L4 真机登录态 dry-run | `make poc-login`（需用户 Edge） | 每次 macOS check |
| L4.5 真机 DOM | `make poc-extract`（ping + extension_dom） | EXTRACT-002；暂不阻塞 check |
| L5 真机非 dry-run | LOGIN-LIVE，本计划登记 | 不阻塞 L1–L4 |

`make check` 本版本 = L1+L2+L3+L4 + pack/install。不含 App/飞书/CDP。不含未绿的真机 DOM。

## 4. 策略

- **政策类**（Allow、微信、光标、盲 Return、AX 假绿）必须单测，不依赖真窗口。
- **坐标类** 用固定 frame/scale 表驱动。
- **会话类** 用 mock backend，禁止 OS 光标。
- **真机类** 只 dry-run；SKIP 仅当没有用户浏览器（exit 0 且打印 SKIP）。
- 禁止把 osascript / lark-cli / mock Send ref / `source=ax_scene_fallback` 写成产品通过。

## 5. 用例索引

见 `docs/testing/BROWSER_TEST_CASES.md`（TC-B-001 … 056）。

## 6. 通过标准

- `cargo test --workspace` 全绿
- `bash scripts/poc_mock_flow.sh` PASS
- `bash scripts/poc_login_state.sh` PASS 或明确 SKIP（无用户浏览器）
- 用例表每条都有「自动化」列：rust 测试名或脚本；不得只写「手工」除非标 L5
