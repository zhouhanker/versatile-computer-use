# Versatile Computer Use (VCU)

厂商与模型无关的 **浏览器 Computer Use** 运行时（Rust CLI / daemon / MCP）。

附着用户自己的 Edge/Chrome 登录态，做网页观察、DOM 操作、绑定截图的坐标点击，以及原生彩色可折叠标签组。**不搬系统光标，不用 CDP，不点调试 Allow。**

宿主模型（Grok 等）若已有视觉，不必 `vcu init model`。

## 当前定版

| 项 | 值 |
| --- | --- |
| Browser Bridge | **0.2.6** |
| Runtime package | **0.1.0** |
| 范围 | 仅浏览器（USER Edge/Chrome + Browser Bridge） |
| 仓库 | https://github.com/zhouhanker/versatile-computer-use |

0.2.6 在 0.2.5 门禁之上补完：光标同背景对照并加大柔光、Chrome 真机 DOM、原生 popup 跨窗禁选、双扩展错路由 retry。不宣称总体 Computer Use 产品已全部完成。

## 能力边界（当前 Computer Use 是什么）

VCU 这一版是 **登录态浏览器操作层**，不是 Codex 官方桌面 Computer Use，也不是通用 OS 键鼠。

**在边界内**

- USER Edge / Chrome + `~/.vcu/lens-extension`，保留登录态
- 标签：列出、选择、打开、关闭；后台新窗口不抢焦点
- 原生标签组：名称、颜色、折叠/展开；选择组内网页时自动展开
- DOM selector 点击 / 输入 / 滚动：唯一目标、可编辑、无遮挡；失效 ID 不 fallback
- viewport PNG 绑定页面状态后按像素点击（60 秒过期，一次消费）
- 跨窗口分组拒绝且无副作用；扩展弹窗按窗口分区、跨窗禁选
- macOS 整窗 CGWindowID 截图 + Guide overlay；AXPress 失败如实报错
- DOM 合成事件：`source=extension_dom`，`trusted=false`，`os_cursor_used=false`

**明确不在边界内**

- 飞书 / Lark 客户端、Finder、微信自动化
- CDP、点击 Edge「允许调试」
- OS cursor warp / HID / 伪造 trusted 手势
- 修改 `~/.codex/computer-use/` 或复制其私有资源
- iframe / canvas / object 等需要原生用户手势的点目标（明确拒绝，不假成功）
- 通用 AX 网页像素真点（TC-B-040 未通过；网页真点走 extension viewport）
- 把 `browser tabs` 当成同时枚举 Edge+Chrome：同一 daemon 上两个扩展会抢 poll，列表通常只反映当前抢到的那一边

## 已知剩余问题

这些是测过、有证据、但未当完成的项：

- 光标：短斜三角 + ~66px 圆雾已对齐原生静态外观；**不宣称**逐像素动画、移动朝向旋转、官方资源复刻
- viewport 截图会先摘掉虚拟光标（避免污染 layout signature），不能用 `browser screenshot` 当光标外观证据
- Edge 与 Chrome 同时装 lens 时，无 tab_id 的命令仍可能打到另一边；有显式 tab_id 时错误浏览器会 `retryable`
- CLI 无独立 `browser hover`
- 复杂动态站、旧页面 content script 升级、资源回收：无新证据不扩大重构
- 布局扫描上限：1000 个 viewport 可见交互目标 / 5000 候选，超限拒绝

## 能做什么（已验证）

- 使用用户已登录的 Edge/Chrome，不另开空 Agent profile
- Chrome 真机：extract / click / type，`source=extension_dom`，计数 0→1
- 更多场景 22 项：dry-run 无副作用、readonly/disabled/遮挡/缺失/歧义/无效 tab 拒绝、滚动后点页底、Return dry-run 阻断、viewport 像素点选、已消费 capture 拒绝
- 原生 popup：按窗口分区；勾选一窗后其它窗复选框禁用

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
