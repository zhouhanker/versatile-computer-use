# 浏览器交互对齐：Codex Computer Use → VCU

日期：2026-09-19。权威执行顺序见 `docs/PLAN.md`。

## 依据与范围

- 用户参考：`/Users/zhouhan/.codex/attachments/d0f3a6be-d409-411c-a0f5-f49d3888659b/image-1.png`。可直接观察到紫色原生标签组、任务名称和组内网页；静态图本身无法证明动作、鼠标动画或内部实现。
- OpenAI 官方浏览器扩展说明：<https://developers.openai.com/es-419/docs/chrome-extension>；官方 Computer Use 说明：<https://developers.openai.com/es-419/docs/computer-use>。作为产品交互参照，不从官方文档推导未公布的指针像素或私有 API。
- Chrome 原生分组 API：<https://developer.chrome.com/docs/extensions/reference/api/tabGroups>。分组名称、颜色、collapsed 与 tab groupId 可通过扩展管理。
- 本仓库 `extension/background.js` / `content.js`、`helpers/vcu-stage/main.swift`、`docs/research/06-codex-computer-use-and-originone-helper.md`。历史研究是结构参考，旧桌面主线和 CDP 建议已被浏览器版计划覆盖。
- 不访问、复制或修改 Codex CU 的安装资源。保持宿主/模型无关、USER Edge/Chrome、无 CDP Allow、无 OS cursor warp。

## 差距与设计

| 方面 | 当前证据与差距 | 实施方式 |
| --- | --- | --- |
| 网页选择 | 有 tab_id，但无效 ID 会 fallback；extract 会遍历其它站点 | `browser tabs` 列真实 tabs/groups；`browser select --tab`；明确失效即错误；未指定仅焦点 HTTP 页 |
| 任务可见性 | open 仅建 tab，没有截图中的组名/色线/折叠 | 原生 `tabs.group` + `tabGroups.update`；`open --session-name` 创建具名组，`--group` 加入已有组 |
| 标签折叠 | 缺少 API/CLI/MCP 入口 | group create/update/ungroup；选择隐藏 tab 自动展开组；用原生组状态跨 SW 重启恢复 |
| DOM 点击 | `.click()` 不验证遮挡；失败/超时可能重试动作 | 唯一目标与可交互性检测；滚动后重新测量/命中检查；先探活再执行一次；错误保留 |
| 输入 | selector 未命中回退 activeElement，任意节点 textContent 可被覆盖 | 精确目标、编辑能力检查、native value setter；拒绝 readonly/disabled/noneditable |
| 指针视觉 | 网页蓝圆环、macOS Guide 圆环，缺少统一箭头与动作反馈 | 原创 SVG/AppKit 箭头+柔和光晕；精确热点；短暂 click pulse；自动清理；pointer-events:none |
| 截图像素点击 | 当前基于 AX 命中，网页细粒度可能受 AXWebArea 限制 | 单列验收；记录实际路径与结果。后续如需 DOM 坐标路由，必须绑定截图目标/比例/页面时效，不可悄悄替换坐标语义 |

## 接口契约

HTTP `/v1/browser/tabs`（GET）、`select`、`group`、`group/update`、`ungroup`（POST）。管理响应 `source=extension_tabs`，网页动作 `source=extension_dom`。所有返回 `os_cursor_used=false`；未执行不填成功。

原生 group：`group_id` / `window_id`（字符串）、`title` / `color` / `collapsed`。分组必须显式给同窗口 tab IDs；拒绝固定页、受限页与跨窗口选择。已有用户组不自动接管；新建任务只分组新标签或用户明确列出的标签。

CLI：`browser tabs`、`select --tab ID`、`group --tabs ID,ID --title NAME [--color purple] [--collapsed]`、`group-update --group ID [--collapsed true|false] [--title NAME]`、`ungroup --tabs ID,ID`、`open --url URL [--session-name NAME | --group ID]`。MCP 对应 `vcu_browser_*`。

DOM click 是非 trusted 的 DOM activation；返回真实 input_path/trusted 字段，不以 pressed=true 推断网站业务成功。业务成功须再 observe/extract 验证。

## 验证与阶段记录

P1：已阅读参考图与当前实现，确认原生标签组方向与上述缺陷。P2–P4 已分配独立文件责任，主代理整合 Rust 接口、计划与验收。后续结果写入 `docs/testing/BROWSER_PARITY_RESULTS.md`，每项区分自动测试、受控真机与未验证状态。

P2 补充发现与修复：Rust bridge 的 1.5 秒 lease 原先会重投点击等非幂等动作，现限制仅只读命令重投；AXPress 错误码原先被脚本成功掩盖，现要求明确 axpress:0；像素路径要求同一应用的截图元数据，拒绝窗口移动/缩放与缺失比例。

## P6：从真实失败推动的截图坐标路线

真实AXPress返回-25201，不能支持该网页按钮。增加 `POST /v1/browser/screenshot` (`tab_id`)，扩展 captureVisibleTab 得到网页PNG；HTTP持久化 capture_id、文档状态、尺寸。`click` 的 `space=viewport` 和 capture_id 将像素映射到网页CSS坐标，内容脚本严格比较状态并命中目标。无效、过期(60秒)、已消费截图拒绝；非dry-run只消费一次。网页变更不重用旧图。

截图要求目标tab active以确保captureVisibleTab拍到正确页；动作始终绑定该tab，用户切换标签不重定向或强抢焦点。DOM point click保留原始热点，拒绝iframe/canvas等需trusted事件的点目标。整窗使用native helper查CGWindowID再screencapture -l，不以屏幕区域替代。CLI/MCP观察失败不再包成ok:true。

## 用户视觉反馈（待实施）

用户明确指出当前指针外观应与 Codex Computer Use 相同，现有近似设计不符合最终要求。保留已验证的热点/透传/清理逻辑，待实际光标参考图路径后重做视觉。不能以当前cursor-live.png截图证明视觉对齐已完成。

P4 已按用户建议完成原生CU取样：通过公开Computer Use工具打开受控页，在原生Edge App层捕获实际虚拟光标（浏览器DOM动作截图不保留该原生光标）。基准为短斜三角、半透明深灰内填、细白描边、蓝灰弥散光晕，无长尾、无硬边圆环。网页与Guide已统一该造型；参考与实现截图保存在`.local/browser-parity/`。使用原创SVG/AppKit路径，未读取或修改Codex安装资源。
