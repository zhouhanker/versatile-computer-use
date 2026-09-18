# 08 — Codex Computer Use 结构对齐（VCU 实现）

版本：0.1-design  
日期：2026-09-18  
状态：**已确认，本目标开始落地**  
政策：只借鉴 **结构**。禁止抄对方类名、Lens/Cursor PNG、AppInstructions 原文；禁止修改 `~/.codex/computer-use/`。

## 0. 用户前提

- 宿主模型（Grok 4.6 等）**已有视觉**。`vcu init model` 只在宿主没有视觉时才需要。
- 最高优先级：**登录态浏览器** = 用户自己的 Edge/Chrome 窗口，不是 `~/.vcu/edge-agent-profile`。
- 禁止代点 CDP Allow；禁止微信；禁止 OS 光标 warp。
- 飞书真发送仍要用户点名收信人。

## 1. 结构映射

| Codex CU 结构 | VCU | 本目标 |
| --- | --- | --- |
| 长驻 helper + 一次 Accessibility | Steward (`vcu-daemon`) + Stage (`vcu-stage`) | 已有 |
| 浮动 overlay「正在使用电脑」 | Stage HUD 胶囊（hudWindow 材质，~300×32） | 本目标改样式 |
| Software / fog cursor | Guide overlay，不 warp OS cursor | 已有；像素→点对齐本目标 |
| AX + 截帧 Scene | Scene + `screenshot_scale` | 已有 scale；像素 click 本目标 |
| 真窗口（已登录浏览器） | `surface=desktop` 附着 **user** profile 窗口；Scene 开 `AXEnhancedUserInterface` 抽 `page_url`/`tabs`/`AXWebArea` | **本目标主路径** |
| AppInstructions | `playbooks/*.md` | `playbooks/user-browser.md` |
| 独立空浏览器 | `browser_agent` 旁路，无 cookies | 降为 lens/CI |

## 2. 登录态浏览器

| 路径 | Cookies | Allow | 何时用 |
| --- | --- | --- | --- |
| **desktop + 用户 Edge/Chrome** | 有 | 无（AX 一次授权） | **默认 / 最高优先** |
| CDP attach 用户实例 | 有 | 会话最多一次 Allow，**禁止代点** | 用户主动 Allow 后做 DOM extract |
| 用户 profile 里的 VCU 扩展 | 有 | 无 | DOM lens；一次手动 load unpacked |
| `edge-agent-profile` | **无** | 无 | 公开页 / CI，**不是登录态** |

命令：

```text
vcu browser login-state
vcu session start --surface desktop --browser edge
vcu session start --surface desktop --app-id proc:Microsoft_Edge:<pid>
```

Doctor 检查名：`login_browser`。Observation 带 `login_state` / `browser_profile`.

## 3. 视觉

宿主多模态读 `screenshot_ref` / `webview_screenshot_ref` 即可。  
坐标：截图像素 → `ax_point_from_pixel` → Guide（AX 点）→ 最小包含 ref 的 AXPress。  
WebArea 内精细 DOM 仍要 CDP/扩展；像素 click 对整块 WebArea 只能命中该 AX 节点（诚实写在 `hit_ref`）。

## 4. 坐标

- AX frame：点，左上原点，y 向下  
- PNG：常 2×；`screenshot_scale` ∈ {1,2,3}  
- `click`：`target.ref` **或** `args.pixel_x/pixel_y` + `space=window|webview`  
- 禁止 HID / `CGWarpMouseCursorPosition`

## 5. 分期

| 切片 | 状态 |
| --- | --- |
| LOGIN-001 用户窗识别 + doctor/CLI + session 优先 user Edge | 本目标 |
| STEW-027 像素→AX + Guide | 本目标 |
| STEW-028 Observation/MCP 文档 | 本目标 |
| Desktop Scene extract（AX，无 CDP） | 本目标 |
| HUD Apple 材质缩小 | 本目标 |
| 用户点一次 Allow 后的 CDP L1/L2/L3 | 仍需用户；不代点 |
| FEISHU-001 真发送 | 仍需点名收信人 |
| login-state key（禁 HID / 禁盲目 Return） | 本目标 |
| Windows UIA / 公证 / JSON-RPC socket | 以后 |

## 6. 硬边界

微信 denylist；不动 Codex CU 安装；desktop 会话结束必须 `session stop`；不留 HUD。
