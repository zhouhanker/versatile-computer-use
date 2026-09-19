# 浏览器交互对齐验证记录

日期：2026-09-19。基线 `2c28009`，当前为未提交工作区。用户目标与阶段见 `docs/PLAN.md`、`docs/design/09-browser-interaction-parity.md`。

## 已完成的阶段证据

- P1：已查看用户参考图，确认采用原生彩色、具名、可折叠标签组；完整指针动画并未在参考图中展示，采用原创箭头/描边/halo对齐视觉方向。
- P2/P3：HTTP 新增 9 项与 MCP 新增 1 项集成测试通过；Node 后台 11 项通过。真实 USER Edge Bridge 0.2.0 命名组创建/折叠/选择自动展开成功。
- P2 真实 DOM：本地 fixture 13 个命令验证，dry-run 不点击、真实计数只增加 1、精确输入、遮挡/禁用/多匹配/缺失/readonly/无效 tab 拒绝。证据 `.local/browser-parity/live-dom.json`。
- P3 原生 UI：已目视检查展开与折叠组截图，原生紫色组名/色线符合参考交互。证据 `.local/browser-parity/group-expanded.png`、`group-collapsed.png`、`native-group-collapse.json`、`select-expands.json`。
- P4：content 7 项测试通过，覆盖重复注入、热点与事件坐标、拒绝目标、输入原生setter、cursor自动清理；Swift helper 编译成功。最终实际渲染检查待完成。

## 真机暴露的问题与后续修复

1. `AXPress` 返回 `-25201`，本地按钮计数保持 0。旧代码会当成功，现在报告 `ActionFailed`；证据 `.local/browser-parity/live-pixel.json`。不能把此项算坐标点击通过。
2. `observe` 采用 screen-region capture，浏览器被终端盖住时图片实际上是终端。P6 改为 CGWindowID 截图，并停止把网页的屏幕区域裁剪当作浏览器图片。
3. P6 新增 `browser screenshot --tab` 和 `click --space viewport --capture`，使用浏览器自身 captureVisibleTab；绑定文档、URL、尺寸、滚动、缩放和 DOM revision；60 秒过期，真实点击一次消费，防止不确定回执后重复操作。

## 验证边界

当前记录不等于全部阶段完成。Bridge 0.2.1 与 P6 的最终自动回归、真实像素点击、指针/弹窗视觉验证与完整门禁仍在执行。DOM 事件为 synthetic，`trusted=false`，不宣称支持需要原生用户手势的 iframe/canvas 交互。

截图保存在被 git 忽略的 `.local/browser-parity/`，避免把用户其它网页标题纳入提交。CLI 原始受控页面动作记录保留在同目录。未发布、未提交。


## 最终本轮回归（Bridge 0.2.2）

`rtk proxy make check` exit 0；完整日志 `.local/browser-parity/make-check.log`（原始 `/tmp/vcu-parity-evidence/make-check-complete.log`）。

- Rust：99 passed；Node：25 passed。
- mock flow、extra actions、login-state（observe、Guide坐标、地址栏dry-run、scroll dry-run、wait、Return门禁）均通过。
- release打包、checksum、curl本地安装与MCP stdio smoke通过；未发布GitHub Release，未提交或推送代码。
- Bridge0.2.2 ping已实测，源码、本机lens-extension与share bundle已同步；CLI/daemon/MCP/Guide已安装到`~/.local/bin`。
- P6真实viewport：3824×1770 PNG，CSS viewport2390×1106，浏览器80%缩放；像素(1310,422)映射CSS(818.75,263.69)，计数1→2，原输入888保留，重复capture拒绝。证据 `viewport-click-final.json`。
- 截图后手动页面变化导致revision变化，旧capture被拒绝；目标tab切换不改写指向，动作绑定原tab。
- 原生窗口截图改用CGWindowID；AX会忽略小于200×150的浏览器控制浮窗，正确读取内容窗。`observe_failure`回归验证内层截图错误不被包装成成功。
- 修复旧extra POC的桌面TextEdit调用，改为mock session的OS光标拒绝测试，符合browser-only范围。

## Codex光标实测对照与修订

用户纠正：必须使用真实Codex Computer Use作为样式依据，不能将近似造型当成完成。已调用Codex原生CU打开受控网页，在App层获取真实光标：短斜三角、浅描边、蓝灰色弥散光晕。原始参考 `codex-native-reference.jpg`，放大检查 `codex-cursor-detail.png`；新版VCU `cursor-matched.png`。

Bridge0.2.2与Guide已取消旧的长箭尾和硬边圆环，采用参考轮廓与柔光。已目视验证实际网页渲染、保留热点/透传/清理测试。当前证据支持静态外观改进；不宣称原始动画资源或逐像素复刻，动态形态和表观尺寸仍是下一轮细化项。

真实扩展popup已通过Codex原生CU打开，网页列表、勾选、分组输入和折叠按钮均真实显示。折叠按钮操作期间用户接管并自行将测试页分为名称1/3的组，因此未将这次受打断的点击计为自动验证；先前CLI的折叠/展开与选择自动展开已有独立成功证据。保留用户的新组状态。


## 继续迭代：P7/P8（开发中）

用户已重置总额度，旧额度提示不作为本轮阈值。新增独立USER窗口、后台开启不抢焦点、显式关闭测试tab、按窗口组织的popup；状态绑定增加可见交互几何与input/change事件，修补CSSOM修改和输入property变化未触发DOM mutation的缺口。

当前自动测试：Node 30通过；HTTP parity 11通过（含大于2MiB的截图回执）。400节点fake几何采样约6ms，只作算法基准，不能当真实浏览器延迟。真实多窗口/400节点/旧截图拒绝POC正在准备。

布局最多1000个当前viewport内的可见交互目标、最多5000候选扫描；超限明确返回overflow而不假通过。完整几何绑定存于sidecar，工具输出仅摘要，减少上下文开销。截图回执路由扩为16MiB，避免Retina PNG触发默认小请求体限制。

P7第一次真实POC未通过，且测试页已全部清理：两个后台新窗口的tab最后都到了原窗口。根因是`chrome.tabs.group`的新组默认属于当前窗口。已改为显式`createProperties.windowId`（通用group和open命名组两条路径），并修正fake Chrome API默认行为，避免模拟测试再次假绿。Chrome官方契约：<https://developer.chrome.com/docs/extensions/reference/api/tabs#method-group>。失败证据 `.local/browser-parity/multi-window-poc-023.json`，修复后将重跑。


P7/P8修复后真实POC：`python3 scripts/poc_browser_parity.py --live --output .local/browser-parity/multi-window-poc-023-fixed.json` exit0，26项全部通过，cleanup成功。两个USER窗口ID不同；后台开窗/同组开页保持原焦点；跨窗分组拒绝且原组不变；选择自动展开折叠组；403个可见交互目标无overflow；CSSOM移动前后revision都为4，但布局绑定不同，旧截图被正确拒绝；input事件后旧截图也被拒绝，counter保持0。

截图调用耗时119.65ms /116.09ms（单次本机结果，不作为普遍性能保证）。截图3824×1770、CSS viewport2390×1106；内部几何绑定保留于sidecar，模型结果只包含摘要。POC只清理本次nonce对应的3个测试tab，恢复焦点仅在当前焦点仍属于测试窗口时执行，没有改动用户的1/3组。


0.2.4正向点选暴露并修复了布局比较误报：Rust持久化JSON会重排对象键，前端JSON.stringify比较把相同布局当成stale。0.2.3/0.2.4的负向截图检查不能单独证明正向可用性；已改为逐字段/节点内容比较，并加入Rust排序往返正向单测和真实POC稳定capture dry-run。

0.2.5真实正向复验已通过：查看受控页PNG后，以(120,318)点击“目标+1”，映射CSS(75,198.705)，counter从0到1；`positive-025-click.json`，测试页随后关闭。原截图与前一张已读截图SHA相同（ac812698…caffde），截图内容已用局部图核对。


## 当前节点收尾（2026-09-19T12:41:19+08:00）

**当前有效验收：Bridge0.2.5，make check exit0，102 Rust +35 Node，32项真实多窗口/截图POC通过，cleanup成功，正向真实pixel counter0→1。** 以`make-check-final.log`、`multi-window-poc-025-final.json`、`positive-025-click.json`为准。先前0.2.3/0.2.4负向截图测试不能替代0.2.5正向验收。

原生popup打开有真实证据；新版多窗口禁选在使用生产HTML/JS的独立浏览器预览中验证：选中当前窗口后另一窗口checkbox变disabled，取消选择后恢复；文件`popup-window-selection.txt/.jpg`。这不是把mock数据当真实窗口状态，真实窗口状态另由32项POC验证。

已修复并复验截图调用频率限制（MAX_CAPTURE_VISIBLE_TAB_CALLS_PER_SECOND）、Rust JSON排序误报、CSS遮挡命中变化、AX关闭后恢复顺序。光标保持原生参考的短三角与柔光，并有页面zoom逆缩放；双端动态/精细尺寸终验仍在下阶段，不宣称逐像素原始动画复刻。

验收索引`BROWSER_PARITY_NODE_REPORT.json`包含工作树摘要、证据摘要与sha256。没有提交SHA，没有提交/推送/远端发布。当前节点关闭，下一步按PLAN的PARITY-004/005推进。


## PARITY-004 同背景终验（2026-09-19）

同浅色参考页对照已有 Codex 原生截图、DOM 虚拟光标与 Guide 离屏渲染。viewport 截图会先 `hideVirtualCursor`，故外观证据使用窗口 observe 与 Guide PNG。

发现 mismatch：原 46px 淡蓝晕在浅色背景上明显小于原生约 66px 圆雾，Guide 叠圆透明度过低。已按原生裁图和公开 Cursor Motion fog 尺寸加大圆雾、提高填色不透明度；保持短斜三角、无硬圆环、无长箭尾。未复制官方 PNG 或 `~/.codex/computer-use/`。

- Node：35 passed，含热点、透传、清理、80/100/200 zoom 补偿。
- 真机 DOM：idle 在目标钮上、move 移到 Hold move、click pulse 仍为短促缩放。证据 `.local/browser-parity/parity-004/`。
- Guide：`guide-idle-100.png` 与 DOM 同浅底。Guide 是屏幕像素，不随页面 zoom 变大。
- 不宣称：逐像素动画、移动朝向旋转、官方 252 资源复刻。



## PARITY-005 原生 popup（2026-09-19）

Edge 工具栏打开真实 `chrome-extension://…/popup.html`：按窗口分区（当前窗口 / 窗口 2 / 3 / 4），组名「1」「3」仍在。勾选当前窗口一条后，其它窗口复选框全部 disabled，文案「已选 1 个网页 · 同一窗口」。随后取消勾选，未点分组/折叠。用户 1/3 组仍在。证据 `.local/browser-parity/parity-005/native-popup.json`。

Chrome：受控页已打开；扩展菜单无 VCU。不能从 Edge 推定 Chrome 通过。


## PARITY-005 Chrome 真机（2026-09-19）

USER Chrome 已加载 VCU lens。受控页 `interaction.html?chrome=parity-005`：

- `extract` `source=extension_dom`，clicks=0
- `click #counter` pressed，trusted=false，os_cursor_used=false，随后 clicks=1
- `type #entry` chrome-live
- 遮挡按钮诚实拒绝 `target is occluded by div`

证据 `.local/browser-parity/parity-005/chrome-live.json`。双浏览器同时轮询时，另一边的扩展对未知 tab_id 返回 `wrong_extension_browser`/`retryable`，daemon 交给下一个 poller。


## 更多真机场景（2026-09-19）

`scripts/poc_browser_more_scenarios.py` 在独立窗口跑 22 项，全部通过。覆盖：dry-run 无副作用、真实点击计数、输入、readonly/disabled/遮挡/缺失/歧义/无效 tab 拒绝、滚动后点页底、wait、Return dry-run 阻断、viewport 截图像素点选、已消费 capture 拒绝、只关闭自己的测试页。用户 1/3 组保留。证据 `.local/browser-parity/parity-005/more-scenarios.json`。
