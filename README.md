# Versatile Computer Use (VCU)

厂商与模型无关的 **浏览器 Computer Use** 运行时（Rust CLI / daemon / MCP）。

附着用户自己的 Edge/Chrome 登录态，做网页观察、DOM 操作、绑定截图的坐标点击，以及原生彩色可折叠标签组。**不搬系统光标，不用 CDP，不点调试 Allow。**

宿主模型（Grok 等）若已有视觉，不必 `vcu init model`。

## 当前定版

| 项 | 值 |
| --- | --- |
| Browser Bridge | **0.2.5** |
| Runtime package | **0.1.0** |
| 范围 | 仅浏览器（USER Edge/Chrome + 扩展） |
| 仓库 | https://github.com/zhouhanker/versatile-computer-use |

本定版已完成：精确 tab / DOM 动作、原生标签组、绑定截图的网页点击、多窗口与面板约束、布局变化后的截图失效。光标已按 Codex 原生取样改成短斜三角 + 柔光；同尺度动态对照和 Chrome 真机终验仍待继续，不在本定版宣称完成。

## 能做什么

- 使用用户已登录的 Edge/Chrome，不另开空 Agent profile
- 明确 tab：列出、选择、打开、关闭；后台新窗口不抢焦点
- 原生标签组：名称、颜色、折叠/展开；选择组内网页时自动展开
- DOM selector 点击 / 输入 / 滚动：唯一目标、可编辑、无遮挡；失效 ID 不 fallback
- 网页 viewport 截图绑定页面状态后，按 PNG 像素点击（60 秒过期，一次消费）
- 跨窗口分组拒绝且无副作用；扩展弹窗按窗口分区
- macOS 整窗用 CGWindowID 截图 + Guide overlay；AXPress 失败如实报错

## 不做

- 飞书等桌面 App、微信自动化
- CDP / 点击 Edge「允许调试」
- OS cursor warp / HID
- 修改 `~/.codex/computer-use/`
- 把 DOM 合成事件当成原生 trusted 手势（`trusted=false`）

## 安装

本地打包安装：

```bash
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist bash scripts/install/install.sh
```

从 GitHub Release 安装见 [`docs/INSTALL.md`](docs/INSTALL.md)。Windows：`irm <host>/install.ps1 | iex`。

安装后：

```bash
export PATH="$HOME/.local/bin:$PATH"
vcu daemon start
vcu browser install-lens    # 然后在 USER Edge/Chrome Load unpacked ~/.vcu/lens-extension
vcu browser ping --json     # 必须 pong；unknown method = Reload 扩展
```

## 快速使用

登录态来自用户自己的浏览器窗口，不要为点一下去 `session start`。

```bash
vcu browser login-state
vcu browser tabs --json
vcu browser select --tab <id>
vcu browser open --new-window --background --session-name '🔎 Task' --url https://example.com

# 原生标签组
vcu browser group --tabs <id>,<id> --title '🎨 分享设计' --color purple
vcu browser group-update --group <id> --collapsed true

# DOM
vcu browser extract --tab <id> --selector a --json    # source 必须是 extension_dom
vcu browser click --tab <id> --selector '#continue' --dry-run
vcu browser type --tab <id> --selector 'input[name=q]' --text 'hello'

# 看图点击：先截图，再使用返回的 capture_id 和 PNG 像素
vcu browser screenshot --tab <id> --json
vcu browser click --space viewport --capture <capture_id> --pixel-x <x> --pixel-y <y>

vcu browser close --tab <id>
```

完整流程见 [浏览器 playbook](playbooks/user-browser.md)。

- 网页动作 `source=extension_dom`；标签管理 `extension_tabs`；网页截图 `extension_viewport`
- 截图绑定 tab / 文档 / URL / 尺寸 / 滚动 / 缩放；页面布局或输入变化后旧 capture 会被拒绝
- 指定标签失效时返回错误，超时回执不会自动重放 mutation
- iframe / canvas 等需要原生手势的点目标明确拒绝

MCP：`vcu_browser_tabs` / `select` / `open` / `group` / `group_update` / `ungroup` / `screenshot` / `observe` / `click` / `type` / `wait` / `scroll` / `key` / `ping` / `extract` / `login_state` / `install_lens`

## 验证

```bash
make check                                          # 本定版：102 Rust + 35 Node
python3 scripts/poc_browser_parity.py               # 默认不执行真实动作
python3 scripts/poc_browser_parity.py --live        # 仅 127.0.0.1 受控页；32 项真机检查
```

`--live` 只关闭自己创建且未被用户接管的标签。不要操作旧 tab ID。

## 开发

```bash
cargo test --workspace
node --test extension/tests/*.test.cjs
vcu daemon start
```

macOS Stage helper：`vcu-stage`。Guide overlay 不移动物理鼠标。当前浏览器版默认无桌面 HUD。

## 硬约束

`os_cursor=deny`。用户网页写入前确认目标 tab。不要盲目 Return（`confirm_send`）。desktop 会话若曾开启，结束必须 `vcu session stop all`。

## 文档

- 当前计划：[`docs/PLAN.md`](docs/PLAN.md)
- 交接：[`docs/HANDOFF.md`](docs/HANDOFF.md)
- 浏览器操作：[`playbooks/user-browser.md`](playbooks/user-browser.md)
- 本定版验收：[`docs/testing/BROWSER_PARITY_RESULTS.md`](docs/testing/BROWSER_PARITY_RESULTS.md)
- 阶段设计：[`docs/design/09-browser-interaction-parity.md`](docs/design/09-browser-interaction-parity.md)
- 安装：[`docs/INSTALL.md`](docs/INSTALL.md)

## 本定版之后

仍待继续，不在 0.2.5 完成声明内：

- 光标与 Codex 原生在同背景、同窗口尺度下的动态 / 缩放对照
- Chrome 真机验收（Edge 已测，不能从 Edge 推定 Chrome）
- 原生扩展弹窗被打断的那段交互补验
