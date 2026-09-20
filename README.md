# Versatile Computer Use (VCU)

厂商与模型无关的 **浏览器 Computer Use** 运行时：Rust CLI、本地 daemon、MCP，外加 Edge/Chrome 扩展（Browser Bridge）。

附着**用户自己的** Edge / Chrome 登录态，做网页观察、DOM 操作、绑图坐标点击，以及浏览器原生彩色可折叠标签组。

**不搬系统光标。不用 CDP。不点「允许调试」。**

宿主模型（Grok 等）若已有视觉，不必 `vcu init model`。

仓库：https://github.com/zhouhanker/versatile-computer-use

## 当前版本

| 项 | 值 |
| --- | --- |
| Browser Bridge | **0.2.8** |
| Runtime package | **0.1.0** |
| 范围 | 浏览器 Bridge **0.2.8 冻结**。macOS 桌面：可见 Stage HUD、Abort、TextEdit 输入、Finder Launch Services 打开自建文件夹、Terminal 粘贴输入（无 Return）、飞书/系统设置只读观察。Windows **CI 真机切片**（不是产品会话）：WinForms Stage（abort 拆 HUD；Guide hover 不搬鼠标）、Notepad `wm_settext`、按钮 `bm_click`、PrintWindow 截图、Explorer open/reveal、cmd / PowerShell `clipboard_paste`（无换行）、Calculator `bm_click`（win32calc）、Settings 只读（click/type 拒绝）、Notepad `wm_vscroll` / extract 读回 / wait 到 Scene value；wait miss 诚实超时；Return/key 无 confirm_send 拒绝；MCP `tools/list` 含 `vcu_hover` / `vcu_session_abort` / wait `value`；CI live `tools/call` hover=`guide_hover`、click=`bm_click`、wait=`scene_wait`、type=`wm_settext`（无换行）、cmd 含换行 type 拒绝、scroll=`wm_vscroll`、extract 读回 Scene 值、screenshot=`image/png`、abort 拆 HUD（不是产品 Windows CU 会话）。**不是**完整 Codex CU（无微信、无 HID、飞书不自动发送、无官方动画）。 |

VCU 是独立的登录态浏览器操作层，**不是** Codex 官方桌面 Computer Use，也不是通用 OS 键鼠。桌面路径见 [`docs/ROADMAP-CU.md`](docs/ROADMAP-CU.md)。

## 能做什么

- 使用已登录的 Edge / Chrome，不另开空 Agent profile
- 列出 / 选择 / 打开 / 关闭标签；默认在**现有窗口开新标签**（`--new-window` 才开新窗口）
- 原生标签组：名称、颜色、折叠 / 展开；选中组内网页时自动展开
- DOM：唯一 selector 的 click / hover / type / scroll；失效 tab 不 fallback
- 先截 viewport PNG，再按像素点击（capture 60 秒过期，真实动作消费一次）
- 已加载扩展热更新：`vcu browser install-lens --reload`
- CLI、HTTP、MCP 同一套动作

网页动作必须 `source=extension_dom`，`trusted=false`，`os_cursor_used=false`。桌面网页仍走扩展；AXWebArea 不是 DOM 点击。

## 做不到 / 不要指望

- 微信自动化、飞书客户端自动发送、系统设置里勾 TCC
- Finder 图标 AXPress（当前 macOS AX 无图标；打开走 NSWorkspace）
- 通用桌面键鼠 / 完整 Windows 产品 CU（CI 有 Notepad/Explorer/cmd 切片，不是产品会话；无官方动画、无微信）
- 在 cmd 里执行命令或带换行 type（换行一律 FocusPolicyViolation）
- CDP、点击 Edge「允许调试」
- 移动系统光标、HID、把合成事件伪装成用户手势
- 跨源 iframe / `object`（明确拒绝）
- 依赖 `isTrusted === true` 的站点（支付、部分 canvas 游戏）
- 通用 AX 网页像素真点（网页真点走 extension viewport）
- 修改或复制 `~/.codex/computer-use/`

## 安装

本地从源码打包：

```bash
bash scripts/pack-release.sh
VCU_BASE_URL=file://$PWD/dist bash scripts/install/install.sh
```

从 GitHub Release 安装见 [`docs/INSTALL.md`](docs/INSTALL.md)。

```bash
# macOS / Linux
curl -fsSL https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.sh | sh

# Windows
irm https://github.com/zhouhanker/versatile-computer-use/releases/latest/download/install.ps1 | iex
```

然后：

```bash
export PATH="$HOME/.local/bin:$PATH"
vcu daemon start

# 第一次：在 USER Edge 和 Chrome 中 Load unpacked → ~/.vcu/lens-extension
vcu browser install-lens

# 之后更新代码：拷文件并热重启已连接的扩展（不必再点扩展页）
vcu browser install-lens --reload

vcu browser ping --json    # 必须 pong；version 应为本 README 中的 Bridge 版本
```

`ping` 报 `unknown method` 或 version 落后：再执行 `--reload`。已打开的旧网页可能仍是旧 `content.js`，刷新该页即可。

## 快速使用

登录态来自用户自己的浏览器窗口，不要为点一下去 `session start`。

```bash
vcu browser login-state
vcu browser tabs --json
vcu browser select --tab <id>

# 默认：现有窗口的新标签。只要独立窗口时才加 --new-window
vcu browser open --url https://example.com --background

vcu browser group --tabs <id>,<id> --title '🎨 分享设计' --color purple
vcu browser group-update --group <id> --collapsed true

vcu browser extract --tab <id> --selector a --json
vcu browser click --tab <id> --selector '#continue' --dry-run
vcu browser hover --tab <id> --selector '#continue'
vcu browser type --tab <id> --selector 'input[name=q]' --text 'hello'

vcu browser screenshot --tab <id> --json
# 看返回的 PNG 后：
vcu browser click --space viewport --capture <capture_id> --pixel-x <x> --pixel-y <y>

vcu browser close --tab <id>
```

细节：[浏览器 playbook](playbooks/user-browser.md)。

约定：

- 网页动作 `extension_dom`；标签管理 `extension_tabs`；网页截图 `extension_viewport`
- 截图绑定 tab / 文档 / URL / 尺寸 / 滚动 / 缩放；布局或输入变化后旧 capture 会被拒绝
- 指定标签失效则报错；超时回执不会自动重放 mutation
- 同源 iframe 点内层节点；canvas 只派发非 trusted 合成事件

MCP 工具：`vcu_browser_tabs` / `select` / `open` / `close` / `group` / `group_update` / `ungroup` / `screenshot` / `observe` / `click` / `hover` / `type` / `wait` / `scroll` / `key` / `ping` / `extract` / `login_state` / `install_lens`；桌面另有 `vcu_hover` / `vcu_session_abort` / `vcu_wait`（`value`）。CI 已 live hover/click/wait/type（无换行）/cmd 含换行 type 拒绝/scroll=`wm_vscroll`/extract/screenshot/abort；live key return 拒绝尚未宣称。

## 限制（已知、不会假装完成）

- 光标：短斜三角 + 柔光；移动会朝向旋转，点击有压缩。不宣称官方逐像素动画
- Edge 与 Chrome 同时连接时，`tabs` 会尝试合并并带 `browser` 字段；`tab_id` 仍是各浏览器内部数字，可能撞号
- 布局扫描上限：1000 个 viewport 可见交互目标 / 5000 候选，超限拒绝
- DOM 事件一律非 trusted，无法替代网站要求的原生用户手势

## 验证与开发

```bash
make check                                     # Rust workspace + Node 扩展测试 + pack
node --test extension/tests/*.test.cjs
cargo test --workspace
python3 scripts/poc_browser_parity.py          # 默认不执行真实动作
python3 scripts/poc_browser_parity.py --live   # 仅 127.0.0.1 受控页
```

`--live` 只关闭自己创建且未被用户接管的标签。不要操作旧 tab ID。

macOS Stage helper：`vcu-stage`。Guide overlay 不移动物理鼠标。当前浏览器版默认无桌面 HUD。

## 硬约束

`os_cursor=deny`。写入用户网页前确认目标 tab。不要盲目 Return（`confirm_send`）。若曾开启 desktop 会话，结束必须 `vcu session stop all`。

## 文档

- 计划（已发布浏览器版）：[`docs/PLAN.md`](docs/PLAN.md)
- 下一史诗（含桌面）：[`docs/ROADMAP-CU.md`](docs/ROADMAP-CU.md)
- 交接：[`docs/HANDOFF.md`](docs/HANDOFF.md)
- 浏览器操作：[`playbooks/user-browser.md`](playbooks/user-browser.md)
- 桌面最短环：[`playbooks/desktop.md`](playbooks/desktop.md)
- 安装：[`docs/INSTALL.md`](docs/INSTALL.md)
- 验收记录：[`docs/testing/BROWSER_PARITY_RESULTS.md`](docs/testing/BROWSER_PARITY_RESULTS.md)
- 阶段设计：[`docs/design/09-browser-interaction-parity.md`](docs/design/09-browser-interaction-parity.md)
