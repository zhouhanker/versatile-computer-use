# USER browser：选择网页、标签组与可靠点击

登录态来自用户自己的 Edge/Chrome 和 VCU Browser Bridge。宿主已有视觉即可使用截图，无需先配置模型。CDP、桌面会话和空 Agent profile 不属于本流程。

## 选择与整理网页

```sh
vcu browser ping --json
vcu browser tabs --json
vcu browser select --tab <tab_id>
vcu browser open --url 'https://example.com' --session-name '🎨 分享设计'
vcu browser open --url 'https://example.org' --group <group_id> --background
vcu browser group --tabs <tab_id>,<tab_id> --title '🎨 分享设计' --color purple
vcu browser group-update --group <group_id> --collapsed true
vcu browser group-update --group <group_id> --collapsed false
vcu browser ungroup --tabs <tab_id>,<tab_id>
```

这些是浏览器原生标签组：名称、彩色顶部线条、点击组名折叠/展开，与参考截图对应。`select` 会展开目标所在组并聚焦其窗口。`tabs` 返回真实 `tab_id`、`window_id`、`focused`、`group_id` 及 groups 的 `collapsed`。

扩展工具栏弹窗也可点击网页名称切换、勾选网页后分组、折叠/展开和移出组。原生组折叠时，弹窗仍保留组内网页入口，便于直接选择。

新建具名任务只分组本次新标签；要把已有网页归组，必须显式列出 ID。同组标签须在同一窗口，不支持固定页或浏览器内部页。`--session-name` 与 `--group` 互斥。`--background` 不激活新网页。解除分组不会关闭网页。

## 观察 → 操作 → 验证

1. `vcu browser observe --json` 生成整窗 PNG 和坐标元数据。MCP observe 返回 `type=image`；同轮查看 `vision_handoff.must_view` 指定图片。
2. 用 `tabs` 确认目标 ID，再用 `extract --tab ID --selector ...` 读取 DOM。默认只定位最后聚焦的用户 HTTP(S) 标签。
3. 执行唯一 selector 的 click/type；尽量显式传 `--tab`。指定 ID 关闭或失效时直接失败，不换到其他网页。
4. 再次 observe/extract 检查页面变化；DOM activation 的 `pressed=true` 仅说明动作已派发，不证明保存/导航等业务结果成功。

```sh
vcu browser extract --tab <id> --selector 'a'
vcu browser click --tab <id> --selector '#continue' --dry-run
vcu browser click --tab <id> --selector '#continue'
vcu browser type --tab <id> --selector 'input[name=q]' --text 'hello'
vcu browser scroll --tab <id> --dy 600
```

DOM 操作必须 `source=extension_dom`；标签管理为 `source=extension_tabs`。指针已依据Codex原生CU实测截图改为短斜三角、细白描边和蓝灰柔光。点击有短暂反馈，不移动物理鼠标、不拦截网页事件，结束自动消失。`dry-run` 不滚动、不聚焦、不修改页面。

selector 多个匹配、隐藏、禁用或被遮挡时应修正目标；输入只接受可编辑节点。DOM 事件为非 trusted，无法替代网站要求的原生用户手势；不要靠重试伪造成功。超时可能发生在动作已经执行之后，应先重新观察，不盲目重复动作。

## 网页截图坐标点击（Bridge 0.2.5）

```sh
vcu browser select --tab <id>
vcu browser screenshot --tab <id> --json
# 查看响应中的 screenshot_path 图片，再使用它的 capture_id 和图片像素：
vcu browser click --space viewport --capture <capture_id> --pixel-x 1310 --pixel-y 422 --dry-run
vcu browser click --space viewport --capture <capture_id> --pixel-x 1310 --pixel-y 422
```

viewport PNG 由扩展直接截取目标页面，返回 `source=extension_viewport`，MCP `vcu_browser_screenshot` 附带图片。图片不含浏览器标签栏，因此坐标起点就是网页左上角。按 PNG 尺寸与实际 CSS viewport 尺寸映射，支持 Retina 和网页缩放；不要拿整窗图片套用此模式。

capture 绑定 tab、document、URL、尺寸、滚动、缩放和 DOM revision，60 秒过期。页面变化时必须重新截图；真实点击消费 capture，即使回执失败也不能盲重试。用户切换标签不会把动作重定向到新标签或抢回焦点，目标仍是截图绑定的原标签。

坐标动作 `source=extension_dom`、`input_path=dom_point_click`、`trusted=false`；iframe/canvas/object 等需原生输入的点目标明确拒绝。成功后再 extract/observe 核验业务结果。

## 浏览器整窗与 Guide（AX）

`browser observe` 使用 macOS 窗口 ID 截图，避免被其它应用覆盖；观察失败直接返回错误，不复用旧图。此路径要求当前版本的 `vcu-stage` helper。

```sh
vcu browser observe --json
vcu browser click --pixel-x 100 --pixel-y 80 --space window --dry-run --guide
```

整窗像素映射 AX 点 = frame 原点 + pixel / screenshot_scale。Guide 不移动物理鼠标。截图必须属于同一浏览器，窗口移动/缩放后拒绝旧坐标。AXPress 非零错误码返回失败；网页按钮优先使用上面的 viewport/selector 路径。微信遮挡时整窗像素真点仍必须 AppDenied。

## 地址栏、等待与按键

```sh
vcu browser type --dry-run
vcu browser type --text 'https://example.com'
vcu browser wait --role AXWebArea --ms 2000
vcu browser key --key return --dry-run
```

不带 selector 的 type 读写 AX 地址栏，不能与 `--tab` 混用。地址栏写入不会自动按 Return。Return 仍受 confirm_send + Send ref 策略约束；不要盲目按键。观察默认没有 HUD，也不需要 desktop session。

## 扩展安装与健康

`vcu browser install-lens` 复制到 `~/.vcu/lens-extension`，在 USER Edge/Chrome load unpacked。更新后 Reload 扩展，`vcu browser ping` 核实版本；本次新能力要求 Bridge 0.2.5。已打开的旧网页建议刷新后再测新内容脚本。

`extension_profile=user` 才是登录态主线；`agent` 表示空 Agent profile，`none` 表示未配对。`unknown method` 表示后台旧版本，应 Reload，不是点击 Allow。DOM 扩展失败不改用 AX 冒充成功。

MCP 对应 `vcu_browser_tabs/select/open/group/group_update/ungroup` 和 `observe/click/type/scroll/extract/ping`。

禁止自动化微信、点击调试 Allow、OS cursor warp/HID，以及修改 `~/.codex/computer-use/`。

当前0.2.5支持`browser open --new-window --background`与显式`browser close --tab`。真实多窗口POC：`python3 scripts/poc_browser_parity.py --live`（仅本地受控页面，默认不执行真实动作）。
