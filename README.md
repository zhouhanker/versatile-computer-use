# Versatile Computer Use

厂商与模型无关的本机 Computer Use 运行时。通过 CLI、本地 daemon 与 MCP，把观察与操作接到 Codex、Claude、Cursor 等宿主；宿主已具备视觉能力时无需再配置专用模型。

仓库：[github.com/zhouhanker/versatile-computer-use](https://github.com/zhouhanker/versatile-computer-use)

## 产品定位

VCU 附着用户自己的 Chrome / Edge 登录态，完成网页观察、DOM 操作、截图坐标点击与原生标签组管理。同一套运行时也提供 macOS 桌面会话（可见 Stage、Guide 虚拟指针、允许名单应用上的辅助功能操作），以及 Windows 侧的窗口观察与控件动作。

Browser Bridge **0.2.8**。运行时软件包 **0.1.0**。

## 能力

**浏览器**

- 使用已登录的 Chrome / Edge，不另开空的 Agent 配置
- 列出、选择、打开、关闭标签；默认在现有窗口打开新标签
- 原生标签组：名称、颜色、折叠与展开
- 按 CSS selector 进行 click、hover、type、scroll、extract
- `observe` 生成 viewport PNG；可用 `observe --tab` 指定标签
- 按截图像素点击（`capture_id` + `space=viewport`）
- 已加载扩展可通过 `vcu browser install-lens --reload` 热更新
- CLI、HTTP、MCP 同一套接口

**桌面**

- macOS：desktop 会话升起 Stage HUD，Abort 结束会话；TextEdit / Notes / Finder / Terminal 等允许名单应用上的观察与输入
- Windows：Notepad、Explorer、命令行、计算器等窗口的观察与控件动作（CI 真机覆盖）

接入方式：`vcu` CLI、本机 HTTP daemon、以及 `vcu-mcp`（stdio MCP）。

## 边界

已发布的是浏览器 Bridge 0.2.8，加上允许名单内的桌面切片。这不是完整 Codex Computer Use，也不是完整 Windows 产品 CU。

- 网页细操作走 extension DOM（`source=extension_dom`），不用 AX 树冒充 HTML
- 不搬系统光标，不代点 Edge「允许调试」，不自动化微信，不修改 Codex CU 安装
- 跨源 iframe、trusted 手势、通用 AX 网页像素真点（TC-B-040）未做
- Windows 证据是 CI 上的 Notepad / Explorer / cmd / 计算器切片，不是带 Stage HUD 的产品会话
- Windows 登录态会枚举本机 Edge/Chrome 主进程，不再依赖 Unix `ps`。扩展已轮询且 lens 已安装时，`login-state` 不应再要求打开浏览器或重新加载扩展
- 桌面会话如果点名的窗口没有可见窗体，会拒绝并保持不改绑到其他应用
- Windows 商店版记事本的启动桩进程会解析到唯一的可见记事本窗口；多开时仍拒绝，不猜窗口
- Windows Terminal 进程名 `WindowsTerminal` 在允许名单内，可以观察
- Windows 桌面截图失败会说明没有可见窗口或 PrintWindow 没产出 PNG，不再提示 macOS 的屏幕录制权限

- 飞书只观察，不自动发送；macOS 深 AX 全树停放

## 安装

macOS / Linux：

```bash
curl -fsSL https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh | sh
```

Windows：

```powershell
irm https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.ps1 | iex
```

从源码打包：

```bash
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist bash scripts/install/install.sh
```

安装说明见 [`docs/INSTALL.md`](docs/INSTALL.md)。

## 开始使用

```bash
export PATH="$HOME/.local/bin:$PATH"
vcu daemon start
vcu browser install-lens
```

首次使用时，在用户自己的 Edge 与 Chrome 中打开扩展页，Developer mode → Load unpacked → `~/.vcu/lens-extension`。之后更新扩展：

```bash
vcu browser install-lens --reload
vcu browser ping --json
```

登录态网页操作：

```bash
vcu browser login-state
vcu browser tabs --json
vcu browser observe --json          # 前台须为 Chrome 或 Edge
vcu browser observe --tab <id> --json
vcu browser observe --browser chrome --json
vcu browser select --tab <id>
vcu browser open --url https://example.com --background
vcu browser open --url https://example.com --browser chrome --background
vcu browser extract --tab <id> --selector a --json
vcu browser extract --tab <id> --browser chrome --selector a --json
vcu browser click --tab <id> --selector '#continue'
vcu browser hover --tab <id> --selector '#continue'
vcu browser type --tab <id> --selector 'input[name=q]' --text 'hello'
vcu browser scroll --tab <id> --dy 600
vcu browser hover --tab <id> --selector '#pad'
vcu browser wait --selector '#ready' --text ready --ms 2000
vcu browser screenshot --tab <id> --json
vcu browser click --space viewport --capture <capture_id> --pixel-x <x> --pixel-y <y>
vcu browser close --tab <id>
vcu browser close --tab <id> --browser chrome
```

标签组：

```bash
vcu browser group --tabs <id>,<id> --title '设计' --color purple
vcu browser group-update --group <id> --collapsed true
```

浏览器操作手册：[`playbooks/user-browser.md`](playbooks/user-browser.md)。桌面最短环：[`playbooks/desktop.md`](playbooks/desktop.md)。

## MCP

将 `vcu-mcp` 配入宿主即可调用浏览器与桌面工具，包括 `vcu_browser_observe`、`vcu_browser_click`、`vcu_browser_extract`、`vcu_hover`、`vcu_session_abort` 等。

```bash
vcu mcp print-config --json
```

推荐顺序：`ping` → `observe`（查看返回的 PNG）→ 在 60 秒内对 last observe 执行 click / type / screenshot / open。也可用 `observe` 的 `tab_id` 指定标签。

## 文档

| 文档 | 说明 |
| --- | --- |
| [`docs/PLAN.md`](docs/PLAN.md) | 当前版本计划 |
| [`docs/ROADMAP-CU.md`](docs/ROADMAP-CU.md) | Computer Use 路线图 |
| [`docs/HANDOFF.md`](docs/HANDOFF.md) | 会话交接 |
| [`docs/INSTALL.md`](docs/INSTALL.md) | 安装 |
| [`playbooks/user-browser.md`](playbooks/user-browser.md) | 浏览器操作 |
| [`playbooks/desktop.md`](playbooks/desktop.md) | 桌面操作 |
