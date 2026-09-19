# Versatile Computer Use (VCU)

厂商与模型无关的 **浏览器 Computer Use** 运行时（Rust CLI / daemon / MCP）。

附着用户自己的 Edge/Chrome 登录态，做网页观察、DOM 操作、绑定截图的坐标点击，以及原生彩色可折叠标签组。**不搬系统光标，不用 CDP，不点调试 Allow。**

宿主模型（Grok 等）若已有视觉，不必 `vcu init model`。

## 当前定版

| 项 | 值 |
| --- | --- |
| Browser Bridge | **0.2.7** |
| Runtime package | **0.1.0** |
| 范围 | 仅浏览器（USER Edge/Chrome + Browser Bridge） |
| 仓库 | https://github.com/zhouhanker/versatile-computer-use |

0.2.7 在 0.2.6 之上补完：光标移动朝向 + 点击压缩、viewport 截图保留虚拟光标、Edge+Chrome 合并 tabs、CLI/MCP hover、同源 iframe 内层点选、canvas 合成点击。不宣称总体 Computer Use 产品已全部完成，也不宣称与 Codex 官方桌面 CU 对等。

本机已装 lens 若仍 ping `0.2.5`/`0.2.6`，需要 `vcu browser install-lens` 后在 Edge/Chrome **Reload** 扩展。

## 能力边界（当前 Computer Use 是什么）

VCU 这一版是 **登录态浏览器操作层**，不是 Codex 官方桌面 Computer Use，也不是通用 OS 键鼠。

**在边界内（已有自动化证据）**

- USER Edge / Chrome + `~/.vcu/lens-extension`，保留登录态
- 标签：列出、选择、打开、关闭；后台新窗口不抢焦点
- 原生标签组：名称、颜色、折叠/展开；选择组内网页时自动展开
- DOM selector 点击 / hover / 输入 / 滚动：唯一目标、可编辑、无遮挡；失效 ID 不 fallback
- viewport PNG 绑定页面状态后按像素点击（60 秒过期，一次消费）；截图默认保留虚拟光标
- 跨窗口分组拒绝且无副作用；扩展弹窗按窗口分区、跨窗禁选
- macOS 整窗 CGWindowID 截图 + Guide overlay；AXPress 失败如实报错
- DOM 合成事件：`source=extension_dom`，`trusted=false`，`os_cursor_used=false`
- 双扩展同时轮询时，`browser tabs` 合并两边列表，并带 `browser=edge|chrome`；已知且不冲突的 `tab_id` 会路由到所属客户端
- 同源 iframe：按内层 `elementFromPoint` 点选
- canvas：合成 pointer/mouse 序列，`input_path=dom_point_click_canvas`，`trusted=false`

**明确不在边界内**

- 飞书 / Lark 客户端、Finder、微信自动化
- CDP、点击 Edge「允许调试」
- OS cursor warp / HID / 把合成事件伪装成 trusted 手势
- 修改 `~/.codex/computer-use/` 或复制其私有资源
- 跨源 iframe / `object` 等需要原生用户手势的点目标（明确拒绝，`unsupported_point_target`）
- 通用 AX 网页像素真点（TC-B-040 未通过，也不打算用 warp/CDP 假绿；网页真点走 extension viewport）
- 依赖 `isTrusted === true` 的站点手势（支付、部分 canvas 游戏、跨源小组件）
- 把 Codex CLI 的 Computer Use 能力当成 VCU 的一部分：VCU 是独立 daemon/扩展，不是 Codex 官方 CU

## 已知剩余问题

这些是测过或明确设计限制、不能写成已完成的项：

- 光标：短斜三角 + ~66px 圆雾；移动超过 8px 会朝向旋转，点击有压缩/光晕。仍**不宣称**逐像素官方动画或资源复刻
- 双浏览器：`tab_id` 仍是各浏览器内部数字，两边可能撞号；撞号时不保证路由，需看 `browser` 字段。双扩展合并目前是桥接单测，不是本机双浏览器真机复检
- 已装扩展若未 Reload，ping 版本会落后于仓库，新动作会 `unknown method` 或走旧行为
- 复杂动态站、旧页面 content script 升级、资源回收：无新证据不扩大重构
- 布局扫描上限：1000 个 viewport 可见交互目标 / 5000 候选，超限拒绝
- 门禁 `make check` 覆盖 Rust + Node 扩展测试 + mock/pack；真机更多场景脚本需独立跑，且不得碰用户现有 1/3 标签组

## 能做什么（已验证）

- 使用用户已登录的 Edge/Chrome，不另开空 Agent profile
- Chrome 真机（0.2.6）：extract / click / type，`source=extension_dom`，计数 0→1
- 更多场景 22 项（0.2.6）：dry-run 无副作用、readonly/disabled/遮挡/缺失/歧义/无效 tab 拒绝、滚动后点页底、Return dry-run 阻断、viewport 像素点选、已消费 capture 拒绝
- 原生 popup：按窗口分区；勾选一窗后其它窗复选框禁用
- 0.2.7 自动化：40 Node 扩展测试；桥接单测含双客户端合并 tabs 与按 tab 所有者路由

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
vcu browser install-lens    # 然后在 USER Edge 和 Chrome 都 Load unpacked ~/.vcu/lens-extension
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
vcu browser hover --tab <id> --selector '#continue' --dry-run
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
- 跨源 iframe / object 明确拒绝；canvas 只派发非 trusted 合成事件

MCP：`vcu_browser_tabs` / `select` / `open` / `group` / `group_update` / `ungroup` / `screenshot` / `observe` / `click` / `hover` / `type` / `wait` / `scroll` / `key` / `ping` / `extract` / `login_state` / `install_lens`

## 验证

```bash
make check                                          # 工作区门禁：Rust workspace + Node 扩展测试 + POC/pack
python3 scripts/poc_browser_parity.py               # 默认不执行真实动作
python3 scripts/poc_browser_parity.py --live        # 仅 127.0.0.1 受控页
python3 scripts/poc_browser_more_scenarios.py       # 更多真机场景（独立窗口，不碰用户组）
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
- 验收记录：[`docs/testing/BROWSER_PARITY_RESULTS.md`](docs/testing/BROWSER_PARITY_RESULTS.md)
- 阶段设计：[`docs/design/09-browser-interaction-parity.md`](docs/design/09-browser-interaction-parity.md)
- 安装：[`docs/INSTALL.md`](docs/INSTALL.md)
