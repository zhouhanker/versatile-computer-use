更新：2026-09-26。CU-WIN-SESSION-015 已在本机复测：科学模式 cos(0) 显示「显示为 1」，并切回标准模式。不是完整科学计算器。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-014 已在本机复测：科学模式 sin(30°) 显示「显示为 0.5」，三角学菜单用 toggle 展开，并切回标准模式。不是完整科学计算器。

更新：2026-09-26。CU-WIN-SESSION-013 已在本机复测：科学模式 ln(e) 显示「显示为 1」，并切回标准模式。不是完整科学计算器。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-012 已在本机复测：科学模式 log10(100) 显示「显示为 2」，并切回标准模式。不是完整科学计算器。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-011 已在本机复测：科学模式输入 π，显示为 3.1415926535897932384626433832795，并切回标准模式。列表项点击是 selection_item。不是完整科学计算器。

更新：2026-09-26。CU-WIN-SESSION-010 已在本机复测：清除全部记忆后调用，显示仍是「显示为 0」，不是之前存的 5。不是完整计算器产品。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-009 已在本机复测：记忆里 5 减 2，调用后显示「显示为 3」。不是完整计算器产品。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-008 已在本机复测：记忆里 2 加 3，调用后显示「显示为 5」。不是完整计算器产品。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-007 已在本机复测：记忆存储 5，清除后调用，显示回到「显示为 5」。不是完整计算器产品。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-006 已在本机复测：会话计算 12+7，显示变成「显示为 19」。GitHub Contributors API 当前只列出 zhouhanker（198），没有 zhouhan；Insights 若仍显示旧名，那是图缓存，没有改历史。不是完整 Windows 产品 CU。

更新：2026-09-26。CU-WIN-SESSION-005 已在本机复测：会话滚动把自建列表从 `top=0` 滚到 `top=8`，系统光标仍是 `1187,239`。不是完整 Windows 产品 CU。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。CU-WIN-SESSION-004 已在本机复测：会话悬停把 Guide 放到自建按钮上，`dart=78`，系统光标仍是 `1187,239`。不是完整 Windows 产品 CU。详见 docs/PLAN-EPIC-WIN-SESSION.md。

更新：2026-09-26。WIN-VIS-008 已在本机复测：正后方从蓝变绿后胶囊从 `14,49,125` 变成 `14,121,36`，下方仍是蓝，说明没有只采下方。不是 macOS 系统材质。详见 docs/PLAN-WINDOWS-VISUAL.md。

更新：2026-09-26。WIN-VIS-007 已在本机复测：胶囊模糊从 `119,16,99` 跟着背后变成 `14,121,38`，没有重启 Stage。不是 macOS 系统材质。详见 docs/PLAN-WINDOWS-VISUAL.md。

更新：2026-09-26。WIN-VIS-006 已在本机复测：HUD 采样桌面模糊后像素 `125,26,105`，不是不透明海军蓝，也不是 macOS `NSVisualEffectView`。详见 docs/PLAN-WINDOWS-VISUAL.md。

更新：2026-09-26。WIN-FIX-009 已在本机复测并准备提交：`scripts/poc_win_session_own_edit.ps1` 对自建 WinForms 编辑框写入成功，`input_path=wm_settext`，独立 WM_GETTEXT 读回标记，没有剪贴板假成功，没有移动系统光标。不是完整 Windows 产品 CU。详见 docs/PLAN-WINDOWS-FIX.md。

# VCU 会话交接

更新：2026-09-26。WIN-FIX-001 至 006 已提交并在本机复测。WIN-FIX-007 已复测：中文 Windows 上的 POC 不再因 GBK UnicodeDecodeError 退出。CU-D-590 与 CU-D-620 通过。CU-D-610 的 DOM 输入和滚动成功，但截图 dry-run 仍失败，不写成整项通过。计算器宿主仍未放行。没有新的 GitHub Release。详见 docs/PLAN-WINDOWS-FIX.md。 WIN-VIS-001：Windows Stage 改为 280x28 胶囊和圆形短箭，实机可见；不是完整产品 CU，也没有 DPI 逐像素对齐。详见 docs/PLAN-WINDOWS-VISUAL.md。


更新：2026-09-25。HEAD `e5d47df`，`main` 与 `origin/main` 同步，工作区在写入本交接前是干净的。上一轮功能收尾是 2026-09-20 的 CU-D-680/690/700（当时 HEAD `b627506`，交接提交 `5a3ae97`）。

## 当前进度

- 已发布产品是浏览器 Bridge **0.2.8**。crate 仍打印 `0.1.0`，不代表没更新。
- 桌面切片 `CU-D-010`…`CU-D-700` 已关闭。不是完整 Codex CU，也不是完整 Windows 产品 CU。
- 台账无进行中切片。可领取只剩停放的 `MAC-NEXT`、`FEISHU-001`。未点名不开。
- CI 已恢复：Actions run `36129175359`（代码 `9c69b8c`）五件 job 全绿。文档记录提交是 `e5d47df`。

## 下一步大纲（未开工）

默认停在收口，不新开史诗。用户点名后只开一条：

1. 守门：CI 或真机回归坏了再修。
2. 浏览器诚实缺口：跨源 iframe / trusted 手势 / `TC-B-040`。
3. Windows 产品会话：现有 Windows 切片不是 `vcu session` 产品路径。
4. `MAC-NEXT`：深 AX，先过 Accessibility 门禁。
5. `FEISHU-001`：点名收信人之后才谈发送。

红线：不搬系统光标，不代点 Edge Allow，不自动化微信，不改 `~/.codex/computer-use/`，不动标签组 1/3。网页细操作必须 `source=extension_dom`。

## 本轮（收口 + CI 门禁）

- 不新开停放史诗。`MAC-NEXT` / `FEISHU-001` 仍不 claim。
- 对齐覆盖边界：`README.md` 增加「边界」；`GOALS.md` 从 0.2.5 改到已发布 0.2.8；测试计划第 8 节去掉已通过的 TC-D-690/700「未过不得宣称」。
- CI 失败根因不是产品回归。`observe_failure` 的假 daemon 只等 5 秒，且非阻塞读把 `WouldBlock` 当失败；Windows 上 `vcu` 还因 1MB 主线程栈溢出（`0xC00000FD`）在连上 mock 前退出。随后 `self_update_failure` 在 windows-latest 上调用了 WSL stub `bash`，没有跑 `install.ps1`。
- 修复：假 daemon 活到 CLI 退出；CLI 入口改到 8MB 栈，Windows 二进制 `/STACK:8388608`；Windows `self update` 走 `install.ps1`，安装器 UTF-16 输出可解码。本地 POC：`observe_failure` 连续 21 次，以及 `cargo test -p vcu-cli --tests --bin vcu`。
- 门禁已恢复：Actions run `36129175359`（`9c69b8c`）五件 job 全绿，含 `test (windows-latest)`、`test (macos-latest)`、`package-windows`、`package-macos`。

## 2026-09-20 收尾

HEAD `b627506`（main 已推送）。上一轮结尾为 `7a7a7c8`（CU-D-670）。

## 本轮完成（CU-D-680 → CU-D-690 → CU-D-700，全部提交并推送）

- **CU-D-680 `group-update --browser`（真机通过）**：`scripts/poc_cu_d_680.py` → `CU-D-680 OK`，报告 `.local/desktop-cu/cu-d-680.json`。覆盖：合并列表带 `browser` 标记；`--browser chrome|edge` 只改对应组（title/color/collapsed），另一浏览器组不动；id 唯一时无 `--browser` 正确解析；另一浏览器真实 id + 错 `--browser`、以及未知 id 都是诚实失败且不误改；组清理干净、组 1/3 未动。撞号分支由 `cargo test -p vcu-server --test app_http`（`upd_amb`）覆盖（真机构造不出同号 `group_id`）。提交 `03db63b`、`1ad02ca`、`84e48ec`。
- **CU-D-690 extension DOM 原生 `<select>`（真机通过）**：`type --selector` 接受 `<select>`，按 option 的 value 或可见文本匹配，设置后派发 `input`+`change`，回执 `input_path=dom_select`；匹配不到报 `select option not found` 且不改值；文本输入仍是 `dom_type`。单测 46 项（`extension/tests/content.test.cjs`）；真机 `scripts/poc_cu_d_690.py` → `CU-D-690 OK`。真机验收：用 VCU 在 GitHub Support 工单页选中「Type of Issue」并**成功提交**（页面回执「您的信息已成功提交」，证据 `.local/desktop-cu/cu-d-690-github-ticket.png`）。提交 `ee66ae7`、`1af8764`。
- **CU-D-700 Release 托管 + `vcu self update`（完成）**：发布 GitHub Release `v0.2.8`（Latest，16 资产）。`curl -fsSL .../releases/latest/download/install.sh | sh` 装到临时前缀成功；不带 `VCU_BASE_URL` 的 `vcu self update` → `updated: true`，装出的 vcu/vcu-daemon/vcu-mcp 与 Release 包 sha256 完全一致。失败路径输出 installer stderr + 本地 `file://` 提示（单测 `crates/vcu-cli/tests/self_update_failure.rs`，并用真机 404 复现）。提交 `98737ca`、`f9a94c7`。
- **顺手清理 / 修复**：删除被 git 跟踪的残留 `extension/background.js.bak`（`fdd76f9`）；修发布管线两个缺陷——0 字节 `dist/.gitkeep` 被当资产导致 publish job 失败（改显式资产列表）、Windows checkout 的 CRLF `install.sh` 覆盖 LF 导致 `curl | sh` 报 `set: pipefail: invalid option name`（`.gitattributes` + publish 步骤 `tr` 归一化）；提交 `c0c49dd`，并用临时 tag 端到端验证（run `35511683467` 绿：14 资产、无 0 字节、install.sh 为 LF），验证后删除该 tag，记录于 `b627506`。

## 门禁与证据

- `cargo test --workspace` 全绿；`node --test extension/tests/*.test.cjs` **46** 通过；`make check` **0**（mock 流程 / 额外动作 / 打包 / curl 安装 POC）。
- 真机 POC：`scripts/poc_cu_d_680.py`、`scripts/poc_cu_d_690.py`（报告 `.local/desktop-cu/`）。
- 发布冒烟：`curl | sh` 临时前缀安装 + `vcu self update` 二进制 sha256 对照（对照对象是线上 Release 资产）。

## 本机环境状态

- `~/.local/bin` 的 `vcu` / `vcu-daemon` / `vcu-mcp` 是 **Release v0.2.8 包内二进制**（sha256 与 release tarball 逐字节一致），`vcu-stage` 也在。
- `vcu --version` 仍打印 crate 版本 `0.1.0`（浏览器桥版本为 0.2.8）——版本号不变不代表没更新。
- daemon 在跑（`127.0.0.1:17890`），lens 已重连，`extension_browsers=["chrome","edge"]`。本轮为真机需要启动了 Chrome（此前只有 Edge 在线）。
- 本轮排查发布问题时下载的临时文件在 `/private/tmp`（可忽略）。AWR 无未关闭会话/claim。

## 本轮解决的两个真实问题（保留现场记录，均已修复）

1. **原生 `<select>` 无法设置** → CU-D-690。现场证据：`type` → `target is not an editable text element`；点 `option` → `target has no visible bounds`；`key` 只出策略 plan（`pressed=false`）。根因：`validateTarget(el,{editable:true})` 只认 input/textarea/contenteditable，且原生弹层点不到。
2. **`vcu self update` 不可用且不解释** → CU-D-700。现场证据：GitHub Releases 为空 → installer 非零退出，CLI 只报 `update installer exited non-zero`（stderr 被 `Stdio::null()` 吞掉）。

## 测试边界（不变）

- 允许：`127.0.0.1` 抛页、dry-run、自己的 tab
- 禁止：组 1/3、用户站点真点、微信动作、CDP Allow、OS cursor、SendInput
- 原生标签组只属于一个浏览器窗口

## 下次会话

1. 先读 `docs/PLAN.md` 的「下一步大纲」。默认不 claim。用户点名一条之后再开工。
2. 台账 `.awr/intake/work-ledger.yaml` 已无进行中切片；可领取只剩 `MAC-NEXT`（深 AX，停放）与 `FEISHU-001`（停放）。**不要 claim 这两项**，除非用户明确要开。
3. CI `test` 三平台与打包已在 run `36129175359` 全绿。不要把这次修复写成完整 Windows 产品 CU。
4. AWR 完成登记：`work complete` / `evidence add` 的报告 schema 未摸清（模板未公开，报错只有通用提示），现用台账 `status: completed` + docs 证据指针，与仓库既有做法一致。
5. 停放项：TC-B-040 / 跨源 iframe / trusted 手势 / 产品 Windows CU / MAC-NEXT 深 AX / FEISHU-001。不要 claim 完整 Codex CU 或完整 Windows 产品 CU。
