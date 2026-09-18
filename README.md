# Versatile Computer Use (VCU)

**厂商与模型无关的 Computer Use 运行时**（Rust CLI / daemon / MCP）。主路径对齐 Codex Computer Use 的结构：长驻 Steward、一次 Accessibility、真窗口 Scene、Stage HUD、Guide overlay。**不搬系统光标。**

宿主模型（Grok 等）若已有视觉，不必 `vcu init model`。

## 登录态（优先，无 HUD）

空 Agent Edge（`~/.vcu/edge-agent-profile`）**没有 cookies**。登录态是用户自己的 Chrome/Edge 窗口。宿主已有视觉（Grok）时走这条 Codex 式闭环，**不要**为点一下去 `session start`。

```bash
vcu daemon start
vcu browser login-state
vcu browser observe --json          # 写 ~/.vcu/captures/login-latest.png
vcu browser click --pixel-x N --pixel-y N --space webview --dry-run
vcu browser type --dry-run          # 读地址栏，默认不写入
vcu browser wait --role AXWebArea
vcu browser key --key return --dry-run   # 禁 HID；Return 要 confirm_send
vcu browser ping --json            # 必须 pong；unknown method = Reload 扩展
vcu browser extract --selector a --json   # source 必须是 extension_dom
vcu browser install-lens            # 然后 USER Edge Load unpacked ~/.vcu/lens-extension
```

- 像素：`ax = frame_origin + pixel / screenshot_scale`（1/2/3）；Guide：`--guide` 只 overlay
- 长任务 HUD：`vcu session start --surface desktop`（胶囊约 220×28）
- **CDP 已抛弃**：不要 `set-cdp`、不要点 Allow。DOM 用 USER Edge 扩展。禁止微信；禁止 OS 光标 warp；禁止盲目 Return

MCP：`vcu_browser_observe` / `click` / `type` / `wait` / `scroll` / `key` / `ping` / `extract` / `login_state` / `install_lens`

## 安装

```bash
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist bash scripts/install/install.sh
```

Windows: `irm <host>/install.ps1 | iex`

## 开发

```sh
cargo test --workspace
vcu daemon start
```

Stage HUD（macOS）：`vcu-stage` 顶部胶囊约 **220×28**，`hudWindow` 材质，内容自适应。

## 硬约束

`os_cursor=deny`；用户 tab 写入前 `tabs borrow`；desktop 会话结束必须 `vcu session stop all`。

## 文档

- 交接：[`docs/HANDOFF.md`](docs/HANDOFF.md)
- 登录态：[`playbooks/user-browser.md`](playbooks/user-browser.md)
- 坐标：[`docs/macos/COORDINATES.md`](docs/macos/COORDINATES.md)
- Codex 对齐：[`docs/design/08-codex-cu-parity.md`](docs/design/08-codex-cu-parity.md)
- 安装：`docs/INSTALL.md`
