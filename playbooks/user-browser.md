# User browser (login-state)

登录态 = **用户自己的 Edge/Chrome 窗口**。空 Agent profile（`~/.vcu/edge-agent-profile`）没有 cookies。

宿主若已有视觉（Grok 等），直接读 Scene 截图，不必 `vcu init model`。

## 串行视觉闭环（强制）

`vcu browser observe` / snapshot 返回 `vision_handoff.must_view`。MCP 工具结果带 PNG。同一轮看图再点。禁止只读 JSON 就结束本轮。

## 默认路径（Codex 结构，无 HUD）

宿主已有视觉时：

1. `vcu browser observe --json` — 截用户 Edge，写入 `login-latest.png` + scale/frame。
2. 读 PNG，用 `vcu browser click --pixel-x --pixel-y --space webview`（先 `--dry-run` 看 `ax_point`）。
3. `vcu browser type --dry-run` 读地址栏；真写入才去掉 dry-run。
4. `vcu browser wait --role AXWebArea`；`scroll --dry-run`；`key --key return --dry-run`（Return 默认拒绝）。
5. 需要挡菜单栏的长任务才 `session start --surface desktop`（Esc 取消）。
6. `vcu browser ping --json` 必须 `pong`。`unknown method ping` = 旧 SW，到 `edge://extensions` **Reload** VCU Browser Bridge（不要点 Allow）。
7. DOM：`vcu browser extract --selector a --json`，`source` 必须是 `extension_dom`。AX chrome 不是 HTML DOM。DOM 点击/输入：`vcu browser click --selector 'a' --dry-run`、`vcu browser type --selector 'input' --text x --dry-run`。
8. DOM / Etherscan 登录树：USER Edge Load unpacked `~/.vcu/lens-extension`（VCU Browser Bridge）。**CDP 已抛弃，禁止点 Allow。**

不要把 `session start` 当登录态主路径。

## 禁止

- 把 Agent Edge 当成已登录浏览器
- 微信 / 微信窗口盖住浏览器时仍点击（真点会 AppDenied）
- `os_click` / HID / 光标 warp
- 改 `~/.codex/computer-use/`
- 把 `source=ax_scene_fallback` 当成 DOM extract 通过

## 扩展在哪个 profile

`vcu browser login-state` 的 `extension_profile`:

- `user` — DOM lens 在已登录浏览器里（要的状态）
- `agent` — 扩展挂在空 Agent Edge，**没有 cookies**
- `none` — 未配对

把 `~/.local/share/vcu/extension` load unpacked 到 **用户** Edge（`edge://extensions`）。不要把 Agent Edge 当成登录态。

## 用户 Edge 扩展（DOM lens）

```text
vcu browser install-lens
```

只复制到 `~/.vcu/lens-extension`，**不点 UI**。在用户 Edge 的 `edge://extensions` 选 Load unpacked。完成后 `extension_profile` 应为 `user`。

## 一等入口

MCP：`vcu_browser_login_state` / `vcu_browser_observe` / `vcu_browser_ping` / `vcu_browser_extract`（无 HUD）。

```text
vcu browser login-state
vcu browser observe --json
vcu browser observe --selector github --json
```

自动选 **user** Edge，无 Stage HUD，带 `screenshot_scale` 与 `extract`。

登录态 Scene（**无 CDP / 不点 Allow**）还会带：

- `page_title` — 窗口标题
- `page_url` — 快路径 observe 可能为空；用 `vcu browser type --dry-run` 读地址栏
- `tabs` — 标签名 + `selected`
- `webview` — 命中 `AXWebArea` 时为 true（像素 `space=webview` 用这块 frame）
- `ax_enhanced` — 已对 Chromium/Electron 设置 `AXEnhancedUserInterface`

`login-latest.json` 同样写入 `page_url` / `page_title`。网页 DOM 树走用户 profile 扩展：`vcu browser extract --selector a`（无 HUD、不 navigate）。**不要 CDP / 不要 Allow**。

## 无 HUD 抽取（推荐登录态观察）

不升起 Stage，不抢焦点：

```text
vcu app snapshot proc:Microsoft_Edge:<pid> --selector "bilibili" --json
vcu app snapshot proc:Microsoft_Edge:<pid> --pixels --selector "*" --json
```

`extract.hud=false`。宿主视觉读 `screenshot_path`（scale 常为 2）。

## Scene extract（无 CDP）

登录态窗口用 desktop Scene 抽 AX，不连 9222：

```text
vcu session start --surface desktop --browser edge
vcu extract --session <id> --selector "*"
vcu extract --session <id> --selector "bilibili"
vcu session stop all
```

网页 DOM 树（Etherscan L3）走用户 Edge 里的扩展。AX 只能拿到标签/工具栏/WebArea 外壳。CDP 已抛弃。

宿主视觉固定读：`~/.vcu/captures/login-latest.png`（`vcu browser observe` 写入）。

## 无 HUD 像素点击（登录态）

看完截图后，不要为了点一下去 `session start`（会升起 HUD）。映射或点击：

```text
vcu browser click --pixel-x 100 --pixel-y 80 --space webview --dry-run
vcu browser click --pixel-x 100 --pixel-y 80 --space webview
```

`--dry-run` 只返回 `ax_point` / `hit_ref`，不 AXPress。真点击仍不搬 OS 光标、不点 Allow。`space=webview` 用 `webview_screenshot_frame`（页面裁帧）；默认 `window` 用整窗 `login-latest.png`。

`--guide` 在映射点闪一下 Guide（无 HUD 胶囊），然后拆掉 overlay。

```text
vcu browser click --pixel-x 100 --pixel-y 80 --space webview --dry-run --guide
vcu browser type --dry-run
vcu browser scroll --dry-run
```

```text
vcu browser wait --role AXWebArea --ms 2000
```

```text
vcu browser key --key return --dry-run
```

Return 无 `confirm_send` + Send ref 会被拒绝（防飞书误发）。Esc 不注入。
