## 串行视觉闭环（强制）

`snapshot --pixels` 之后 **同一轮** 必须看 `vision_handoff.must_view` 里的 PNG（MCP 会直接带上 image content）。没看图不准 click/type，不准停下来问用户屏幕上有什么。

## 无 HUD 观察（不发送）

```text
bash scripts/poc_feishu_scene.sh
vcu app snapshot proc:Feishu:<pid> --selector messenger --json
```

真发送走 **飞书 App + 宿主视觉 + VCU**，禁止 lark-cli / osascript 当验收。`confirm_send` + 点到发送按钮（像素/Guide）。AX 里没有「发送」。

`--pixels` 会写 `~/.vcu/captures/feishu-latest.png` + `.json`（scale/frame），不发送。

# Feishu / Lark — VCU playbook

输入框用 `type`/`AXSetValue`（session type），不要对会话列表盲目 Return。
发送消息必须是用户明确要求之后；默认只 Scene / 截帧。
微信不在本 playbook 范围内（硬拒绝）。

列表过长用 `act type=scroll`（AX），不要 OS 滚轮。发送仍须用户明确要求。

发送：`act type=key key=return confirm_send=true` 且必须带发送按钮 `target.ref`。禁止盲 Return / HID keystroke。Esc 走 Stage HUD，不要注入 Escape。

AX（STEW-016）：窗口 chrome + 侧栏可走 BFS（搜索 ⌘K、消息/日历等 RadioButton、`MultiWebView - messenger`）。
聊天输入框和「发送」**不在 AX 树里**（Electron webview）。不要为发消息盲信 osascript `ok`。

## Compose 路径（STEW-017–019）

1. `vcu app snapshot proc:Feishu:<pid> --json`（不要为此升起 Stage）。
2. 读 `webview=true` 与 `webview_ref`（最大 messenger-chat 窗格，例如 `e15`）。
3. 需要焦点时：`click`/`hit` 该 ref → `input_path=ax_frame_hit`，Guide 移到 frame 中心，**不搬 OS 光标、不 HID**。
4. `--pixels` 会额外写出 `webview_screenshot_*`（messenger-chat 裁帧）。Retina 上 `webview_screenshot_scale=2`（点→像素）。输入/发送仍不在 AX，**不要**假装点到了「发送」。
5. desktop 会话截图：`vcu screenshot --session <id> --tab proc:Feishu:<pid>` 或 MCP `vcu_screenshot`/`vcu_snapshot` 传 `tab_id`；`mode=full` 带 `webview_screenshot_ref`，screenshot 带 `webview_path`。会升起 Stage HUD，用完必须 `vcu session stop`。
6. 真发送（FEISHU-001）：视觉打开张北北会话 → 输入 test → 点发送 → **截图里看见 test**。禁止 lark-cli，禁止相信 osascript `ok`。

禁止：`os_click`、`CGWarpMouseCursorPosition`、微信、Codex CU 目录。
