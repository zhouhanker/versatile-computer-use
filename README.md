# Versatile Computer Use

厂商与模型无关的本机 Computer Use 运行时。通过 CLI、本地 daemon 与 MCP，把观察与操作接到 Codex、Claude、Cursor 等宿主；宿主已具备视觉能力时无需再配置专用模型。

仓库：[github.com/zhouhanker/versatile-computer-use](https://github.com/zhouhanker/versatile-computer-use)

GitHub Contributors 目前只有 zhouhanker。仓库页和贡献图没有单独的 zhouhan。没有改写历史。旧图缓存若还显示这个名字，不是当前提交作者。

Browser Bridge **0.2.8**。运行时软件包 **0.1.0**。`vcu --version` 打印的是 crate 版本，不代表浏览器桥没有更新。

## 能做什么

- 使用已登录的 Chrome / Edge，不另开空的 Agent 配置
- 列出、选择、打开、关闭标签。打开网页时新建后台标签，放进折叠的紫色原生标签组「VCU」，不替换当前页
- 按 CSS selector 做 click、hover、type、scroll、extract
- `observe` 生成 viewport PNG，可按截图像素点击
- 同一套运行时也提供 macOS 桌面会话，以及 Windows 窗口观察与控件动作

接入方式：`vcu` CLI、本机 HTTP daemon、`vcu-mcp`。

扩展不进 Chrome 网上应用店，也不进 Edge 加载项。Windows 上怎么改、怎么测、怎么更新，见 [docs/WINDOWS-DEV.md](docs/WINDOWS-DEV.md)。

## 安装

macOS / Linux：

```bash
curl -fsSL https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh | sh
```

Windows：

```powershell
irm https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.ps1 | iex
```

安装说明见 [docs/INSTALL.md](docs/INSTALL.md)。Release `v0.2.8` 不包含只存在于 `main` 的后续提交。

## 开始使用

```bash
vcu daemon start
vcu browser install-lens
vcu browser ping --json
vcu browser open --url https://example.com
vcu browser observe --tab <id> --json
```

首次在用户自己的 Edge 或 Chrome 里加载解压缩的扩展，目录是 `~/.vcu/lens-extension`。不要点「允许调试」。

浏览器手册：[playbooks/user-browser.md](playbooks/user-browser.md)。桌面手册：[playbooks/desktop.md](playbooks/desktop.md)。

## 文档

| 文档 | 说明 |
| --- | --- |
| [docs/PLAN.md](docs/PLAN.md) | 当前版本计划 |
| [docs/WINDOWS-DEV.md](docs/WINDOWS-DEV.md) | Windows 开发与扩展更新 |
| [docs/INSTALL.md](docs/INSTALL.md) | 安装 |
| [docs/ROADMAP-CU.md](docs/ROADMAP-CU.md) | Computer Use 路线图 |
| [playbooks/user-browser.md](playbooks/user-browser.md) | 浏览器手册 |
| [playbooks/desktop.md](playbooks/desktop.md) | 桌面手册 |
