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
- Windows 登录态会枚举本机 Edge/Chrome 主进程，不再依赖 Unix `ps`。扩展已轮询且 lens 已安装时，`login-state` 不应再要求打开浏览器或重新加载扩展。2026-09-26 本机只复测了 Edge：自建 `127.0.0.1` 页的 extract、选择器点击和输入都是 `source=extension_dom`，已有标签没有被改，没有测 Chrome
- 桌面会话如果点名的窗口没有可见窗体，会拒绝并保持不改绑到其他应用
- Windows 商店版记事本的启动桩进程会解析到唯一的可见记事本窗口；多开时仍拒绝，不猜窗口
- Windows Terminal 进程名 `WindowsTerminal` 在允许名单内，可以观察
- Windows 桌面截图失败会说明没有可见窗口或 PrintWindow 没产出 PNG，不再提示 macOS 的屏幕录制权限
- Windows UIA 名称按 UTF-8 返回，记事本的「文件」和 Windows Terminal 中文标题不再是乱码
- Windows 上的官方 POC 按 UTF-8 读取 vcu 输出，避免中文系统的 GBK 解码把验收脚本打崩。这不代表 CU-D-610 的截图 dry-run 已通过
- Windows Stage 条是 280x28 半透明圆角胶囊，桌面 Guide 是和 macOS 相同几何的短箭加软雾，箭尖对准坐标，不移动系统光标。Stage 按物理像素摆指针，和这台 144 DPI 机器的 UIA 窗口框一致。胶囊上的标题和 Esc 取消左右分开。桌面短箭雾色和网页指针使用同一组蓝灰。 Windows 胶囊在升起前采样背后桌面并模糊，再盖半透明石墨；刷新时短暂排除自身截图，采的是正后方，不是只采下方。Windows 11 上胶囊优先用系统 Acrylic，半粗白字和 1px 描边在点击穿透层，胶囊外有轻阴影；Acrylic 不可用时才退回采样模糊。这不是 macOS `NSVisualEffectView`。Windows 计算器窗口可以按标题绑定，不放行整个 ApplicationFrameHost。`vcu session start --surface desktop --app-id win:Calculator:<pid>` 会拉起 Stage，Abort 会拆掉 HUD。经会话按「七」、1+1、12+7、记忆、科学模式的 π、log10(100)、ln(e)、sin(30°)、cos(0)、tan(45°) 和 sin(π) 的角度/弧度 走 UIA，不移动系统光标。列表项用 SelectionItem，不用系统光标。会话悬停只移动 Guide，系统光标坐标不变。会话滚动会先找列表；顶行没变就不把窗体滚动当成成功。会话截图对自建窗口能读回窗口颜色；PrintWindow 空白时只复制该窗口矩形，不扫整张桌面。会话等待可以按控件名找到自建按钮，缺失时超时。这不是完整 Windows 产品 CU。powershell.exe 托管的图形编辑框会先写子控件；只有控制台窗口类才剪贴板粘贴，写不进就不报成功

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

浏览器扩展不用商店上架。Windows 可以直接从已有的 GitHub Release 压缩包解出扩展，不点调试同意弹窗：

```powershell
irm https://raw.githubusercontent.com/zhouhanker/versatile-computer-use/main/scripts/install/install-lens.ps1 | iex
```

不需要先克隆仓库。脚本优先下载 Release 上的 `vcu-lens-extension.zip`；当前 `v0.2.8` 还没有这个独立包，会改用已经在 GitHub 上的 `vcu-latest-windows-x64.tar.gz` 里的 `extension/`。装到 `%USERPROFILE%\.vcu\lens-extension` 后，仍要在 Edge 里手动“加载解压缩的扩展”。不要点允许调试。已有检出时也可以：

```powershell
powershell -File scripts/install/install-lens.ps1 -FromRelease -Open
```

下一次打 tag 会把 `vcu-lens-extension.zip` 和 `install-lens.ps1` 放进 Release 资产。这不是新的 GitHub Release。

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
