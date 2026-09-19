# VCU 当前计划（浏览器版）

更新：2026-09-20。本文优先于HANDOFF历史快照；作者zhouhanker。

## 当前节点

**已发布：浏览器版 Bridge 0.2.8。** 浏览器门禁仍有效。

**下一史诗（进行中）：** 对标 Codex Computer Use **含桌面**。阶段 1–5 已过。CU-D-060 CI 真机列窗+PrintWindow 已过（run `35462329205`）。CU-D-070 CI `SETVALUE_OK path=wm_settext`（非 ValuePattern）。CU-D-080 单测已过。CU-D-090 CI 已过。CU-D-100 CI 已过。CU-D-110 CI 已过。CU-D-120 CI 已过。下一刀 **CU-D-130** Explorer reveal。未宣称完整 Windows 产品 CU。总览与编排见 [`ROADMAP-CU.md`](ROADMAP-CU.md)，测试见 [`testing/DESKTOP_CU_TEST_PLAN.md`](testing/DESKTOP_CU_TEST_PLAN.md)。未完成桌面切片前，不得把 VCU 写成已具备完整桌面 CU。

本地对照实现在 gitignore 的 `/reference/computer-use/`：可反编译作结构参考，笔记进 `docs/research/`，自有代码进仓库；`.app` / 反编译源码 / 官方素材不上传。

0.2.8 仍是登录态浏览器操作层；桌面工作不得回退 `make check` / 扩展测试，不得 warp OS 光标、代点 Allow、自动化微信、修改 Codex CU 安装。

额度规则：用户已重置总额度。旧“周额度不足10%”提示无效；今后按重置后的**总额度剩10%**要求写交接，不能把goal token计数当账户余额。

## 目标与范围

对比Codex Computer Use，改进浏览器操作与虚拟光标，采用参考图中的原生彩色可折叠标签组，持续修复问题，每阶段留下可复核记录。

- 主路径：USER Edge/Chrome + Browser Bridge，保留登录态。宿主已有视觉时不要求`vcu init model`。
- 网页：明确tab、DOM selector、绑定截图的viewport坐标点击、输入/滚动、原生标签组。
- 浏览器整窗：macOS窗口ID截图与Guide；AXPress失败必须诚实报错。
- 0.2.8 冻结：不做桌面 App 产品路径。微信自动化、CDP Allow、OS cursor warp、修改 Codex CU 安装在桌面史诗中也仍然禁止。桌面编排见 ROADMAP-CU。

## 阶段清单

| ID | 阶段 | 当前状态 | 完成证据 / 剩余项 |
| --- | --- | --- | --- |
| PARITY-001 | Codex比较与阶段设计 | 完成 | `docs/design/09-browser-interaction-parity.md` |
| PARITY-002 | 精确tab与DOM动作 | 完成 | 失效ID不fallback、唯一/可编辑/无遮挡目标、无mutation重放；真机DOM通过 |
| PARITY-003 | 原生标签组与网页选择 | 完成 | 命名、折叠/展开、选择自动展开、解除分组；CLI/MCP和真实窗口POC |
| PARITY-004 | Codex光标外观对齐 | 同背景终验已做并修 halo | 浅色同页对照 native/DOM/Guide；放大光晕至约66px 圆雾。不宣称逐像素动画或官方资源复刻 |
| PARITY-005 | 最终整体验收 | Chrome 真机 + 原生 popup 已过 | 40 Node；Chrome extract/click/type source=extension_dom，counter 0→1；原生 popup 跨窗禁选。双扩展错路由会 retryable |
| PARITY-006 | 绑定截图的网页坐标点击 | 完成 | capture绑定文档/布局、60秒过期、一次消费；真实点选counter0→1 |
| PARITY-007 | 多窗口与面板约束 | **本节点完成** | 后台开窗不抢焦点、跨窗拒绝无副作用、组显式保留所属窗口；面板按窗口分区/跨窗禁选 |
| PARITY-008 | 布局变化与截图可靠性 | **本节点完成** | CSSOM移动/遮挡、input事件、JSON排序往返、截图频率控制、大PNG回执；正向/反向测试均通过 |

以前的TEST/EXTRACT/ETH阶段属于基线。ETH只是L1/L2/L3样本，不是全站抓取；FEISHU-001停放。旧MAC-NEXT深AX不是本版本下一步。TC-B-040通用AX网页像素真点仍不能当成已通过，当前已验证的是extension DOM viewport路线。

## 本节点验收

- `rtk proxy make check` exit **0**：**102 Rust +35 Node**；mock/extra/login-state、release打包、checksum/curl-install/MCP smoke全部通过。
- `scripts/poc_browser_parity.py --live`：**32项通过**，测试页安全清理。包含稳定截图允许point dry-run，以及CSS/输入变化必须拒绝；不再只有负向测试。
- 真实看图点选：PNG像素(120,318) → CSS(75,198.705)，counter **0→1**，只发生一次。
- 布局JSON经过Rust重排键后仍可用；完整几何留在sidecar，模型只接收摘要。
- 截图统一排队以满足浏览器每秒2次限制；只读截图可做一次限流恢复，点击/输入等mutation不自动重放。
- AX先启用再枚举窗口，跳过小控制浮窗；截图失败不包装成成功，不使用被遮挡的屏幕区域冒充浏览器图像。

完整结果：`docs/testing/BROWSER_PARITY_RESULTS.md`；机器可读索引：`docs/testing/BROWSER_PARITY_NODE_REPORT.json`。门禁在定版提交前的工作树通过；定版 SHA 为 `d8ee9ad`。

## 下一阶段（按优先级）

- [x] **P1 / PARITY-004**：同背景浅色页对照 native/DOM/Guide 的 idle/click/move；DOM 按 tab zoom 逆缩放。发现 halo 过小过淡后已加大圆雾并统一 Guide。约束仍有效：不再向用户索要截图；不退回长箭尾/硬圆环；仅 mismatch 时改代码；可参考开源/公开技术；不修改私有安装。
- [x] **P1 / PARITY-005**：Chrome 真机 DOM extract/click/type 已过（counter 0→1，input=chrome-live，遮挡拒绝）。原生 popup 已过。用户 1/3 组未改。
- [x] **P2 / PARITY-005（定版提交）**：工作树已定版提交 `d8ee9ad` 并准备推送 origin/main。AWR 证据绑定该 SHA；光标终验与 Chrome 真机仍未完成。
- [x] **P2 / 0.2.7 体验补丁**：光标朝向与点击压缩、viewport 截图保留光标、合并 Edge+Chrome tabs、CLI/MCP hover、同源 iframe 内层点选、canvas 合成点击。
- [x] **P2 / 0.2.8**：`install-lens --reload` 热更新已连接 Edge/Chrome SW；默认 `open` 在现有 USER 窗口开新标签。
- [ ] **P2 后续（浏览器）**：跨源 iframe / trusted 手势 / TC-B-040 仍不在范围内。双浏览器 tabs 真机复检未过（现仅 Edge 轮询）。
- [x] **CU-B-010**：tabs/health 报告 `browser_count`/`browsers`（单测）。真机 Chrome 未连。
- [x] **CU-B-011**：doctor `lens_dual_browser` 在双浏览器已装但只连一个时 warn（单测）。真机仍 Edge-only。
- [x] **CU-D-010**：无 Stage HUD 则 desktop session 失败（`StageRequired`）；HTTP `stage_hud=true`。
- [x] **CU-D-011**：`vcu session abort` / HTTP abort 拆会话（单测）。未弹用户 HUD 做目视 Escape。
- [x] **CU-D-012**：微信 denylist + 覆盖点拒绝（单测）。
- [x] **CU-D-020/021/022/024**：AXPress 非零失败、Return 门禁、Guide overlay、像素命中不报 extension_dom（单测）。
- [x] **CU-D-023**：TextEdit 真机 `e8` AXTextArea 写入标记，`os_cursor_used=false`；Notes AXPress `axpress:0`。CLI session `type --tab`；`resolve_tab` 优先 active app。
- [x] **CU-D-032**：浏览器回归 ping 0.2.8 / tabs / 现窗新标签 / screenshot capture dry-run / throwaway DOM click。组 1/3 未动。
- [x] **CU-D-040**：Finder CG 列窗 + VCU `reveal`/`open_path`（nsworkspace）真机打开 `OPENME`。非 AXPress、非 Return。
- [x] **CU-D-041**：Terminal.app `ax_menu_paste` 真机输入；换行/Return 拒绝。未碰 Ghostty。
- [x] **CU-D-042**：飞书客户端 CG 窗观察；无发送；气泡不在 AX。
- [x] **CU-D-030/031/033**：Observation surface/source；USER Edge AXWebArea 不得假绿；`playbooks/desktop.md`。
- [x] **CU-D-050/051/052**：HUD 文案、Guide 短三角、`VCU_AUDIT=1`。
- [x] **CU-D-060**：CI run `35462329205` Notepad `UIA_OK` + `PRINTWINDOW_OK`。非 `vcu session` 产品路径。
- [x] **CU-D-070**：CI `SETVALUE_OK path=wm_settext`（Edit 无 ValuePattern）。
- [x] **CU-D-080**：backend set_value 回退 WM_SETTEXT（单测）。
- [x] **CU-D-090**：CI `STAGE_OK`/`SNAP_OK`/`TYPE_OK path=wm_settext`。
- [x] **CU-D-100**：CI `INVOKE_OK path=bm_click`（非 InvokePattern）。
- [x] **CU-D-110**：CI `SHOT_OK` 768x519 PNG（PrintWindow）。
- [x] **CU-D-120**：CI `OPEN_OK path=explorer_open`。
- [ ] **CU-D-130**：Explorer reveal（待 CI）。


## 操作入口

```sh
vcu browser ping --json
vcu browser tabs --json
vcu browser open --url https://example.com --background
vcu browser select --tab <id>
vcu browser screenshot --tab <id> --json
# 查看返回PNG后：
vcu browser click --space viewport --capture <id> --pixel-x <x> --pixel-y <y>
vcu browser group-update --group <id> --collapsed true
vcu browser close --tab <id>
```

`source=extension_dom`用于网页动作，`extension_tabs`用于标签管理，`extension_viewport`用于网页截图。DOM是synthetic事件，`trusted=false`；iframe/canvas等需要原生手势的点目标不假报成功。布局绑定上限1000个viewport可见交互目标、5000候选扫描，超限明确拒绝。细节见`playbooks/user-browser.md`。
