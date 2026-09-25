# Windows 真机修复计划

更新：2026-09-25。作者：本机真机测试记录。状态：WIN-FIX-001 与 WIN-FIX-002 已在本机复测通过，其余未开工。

测的是已安装 Release `v0.2.8`（`vcu --version` 仍打印 crate `0.1.0`）。对照当前源码后，下面的缺陷在 HEAD 里也还在。不要把本计划写成已完成，也不要把它说成完整 Windows 产品 CU。

权威边界仍以 [`PLAN.md`](PLAN.md) 为准。本文只记录这台 Windows 真机测出的缺口和修复顺序。

## 1. 边界

做修复时不能越过这些线：

- 不点 Edge「允许调试」。不走 CDP 修登录态。
- 不自动化微信。不发送飞书。不 claim `MAC-NEXT` / `FEISHU-001`，除非用户点名。
- 不移动系统光标。不改 `~/.codex/computer-use/`。不动用户标签组 1/3。
- 网页细操作必须 `source=extension_dom`。`trusted=false` 是诚实结果，不能改成假 trusted。
- 跨源 iframe 点内部必须继续拒绝，不能为了变绿而报成功。
- `WM_SETTEXT` 不是 UIA ValuePattern。`uia_set_value` 成功也不等于完整 Windows 产品 CU。
- CU-D-060…390 仍是 CI 切片，不是 `vcu session` 产品路径。
- Chrome 本轮按用户要求不测。不把单 Edge 结果写成双浏览器已过。

## 2. 已经做了什么

只做了测试和记录，**没有修代码，没有发版**。

| 范围 | 结果 |
| --- | --- |
| 安装与 daemon | Release 已装到 `~/.local/bin`，daemon 可起。`doctor` 诚实写出不是产品 Windows CU |
| Edge 扩展 | ping / tabs / 打开关闭 / extract / 选择器点击 / 输入 / 悬停 / 滚动 / 等待 / 原生 select / 截图 / 像素点击 / 标签组，在自建 `127.0.0.1` 页上有效 |
| 桌面传统窗口 | 找对真实 pid 后，记事本 `uia_set_value` 可见文字；cmd 截图成功；`open_path` 为 `explorer_open`；设置只观察；自建按钮可点；Abort 能拆 HUD |
| 策略门禁 | 微信 `AppDenied`；不抢焦点拒绝；无 `confirm_send` 的回车拒绝；跨源 iframe 像素点击拒绝 |

工作区没有相应代码提交。未提交的只有 `.gitignore` 里 6 行 AWR 本地目录，与这些缺陷无关。

## 3. 不要当成待修 bug

这些是当前设计或已经诚实的失败，修计划不把它们改成“成功”：

- DOM 事件 `trusted=false`。
- 跨源 iframe 像素点击返回 `unsupported_point_target` / `cross-origin iframe requires trusted input`。这是 `TC-B-040` 的诚实缺口，不是 Windows 回归。
- 微信、系统光标、无确认回车被拒绝。
- `doctor.windows_desktop_scope=warn`。
- 探测里对桌面发送 `type=reveal_path` 得到 `NotImplemented`。动作名是 `reveal`，不是 `reveal_path`。未按正确名字复测，**不列入必改**。

## 4. 需要 Fix 的问题

都未修。建议按这个顺序做，一次只开一条。

### WIN-FIX-001 登录态在 Windows 上看不见 Edge

状态：**已修，2026-09-26 本机复测通过。** 尚未发新的 GitHub Release。

- 现象：Edge 窗口开着、扩展已在轮询，`browser login-state` 的 `user_browsers` 仍为空，`next_action` 仍要求打开浏览器或加载扩展。`doctor` 的 `login_browser` 同样误报。
- 根因：`inspect_login_browsers()` 只跑 Unix `ps -ax`。另外 `lens_status` 只看 `HOME`，Windows 上 lens 实际在 `USERPROFILE\.vcu\lens-extension`，所以即使扩展已连通也会提示重新安装。
- 修：Windows 用 `Win32_Process` 枚举 `msedge` / `chrome` / `chromium`，跳过 `--type=` helper，并把 `\.vcu\` 识别为 agent profile。lens 目录优先用 `USERPROFILE`。health / doctor 的 next_action 与 login-state 一样，扩展已是 user profile 时不再要求“打开浏览器”。
- 验收：本机 Edge pid 16520 出现在 `user_browsers`，`profile=user`。扩展 ping pong，`extension_profile=user`，`lens_copied=true`，`doctor.login_browser=pass`。`next_action` 不再包含打开浏览器或 `install-lens`。单测 `login_state` 18 通过。没有点 Allow，没有 CDP。
- 不做：不因此改 macOS 的 `ps` 路径。不把这一条写成完整 Windows 产品 CU。

### WIN-FIX-002 请求的窗口不能静默改绑

状态：**已修，2026-09-26 本机复测通过。**

- 现象：对记事本桩 pid 开会话时，`active_app_id` 变成另一扇已有 cmd，随后对记事本 tab 报 `TabNotFound`。
- 根因：请求的 `app_id` 不在窗口列表时，会话启动回退到别的 allowlist 窗口。
- 修：桌面会话找不到请求的窗口就返回 `TabNotFound`，文案写明 refusing to bind another app。未点名窗口时仍可按原逻辑选择。
- 验收：`win:notepad:999999` 开会话失败，`session list` 为空。真实 `win:cmd:<pid>` 的 `active_app_id` 与请求一致，Abort 后无残留会话。单测 `require_desktop_window_refuses_missing_id` 通过。

### WIN-FIX-003 商店应用窗口 pid

- 现象：`Start-Process notepad.exe` 的 pid 没有 HWND。真正窗口在另一个 `Notepad` 进程。用真实 pid 时，UIA 有 35 个节点，输入是 `uia_set_value`，截图成功。
- 计算器窗口在 `ApplicationFrameHost`，标题「计算器」，进程名不在允许名单，快照被拒绝。
- 修：从启动的桩进程解析到真正拥有可见窗口的 pid，或明确拒绝并给出真实 pid。`ApplicationFrameHost` 只在能证明是允许的商店应用时放行，否则保持拒绝。不要为了计算器去放行整个 ApplicationFrameHost。
- 验收：官方 `poc_cu_d_090.ps1` 在这台 25H2 / `Microsoft.WindowsNotepad` 上能找到 Edit/Document 并写入。计算器要么诚实成功，要么错误里写明宿主进程，而不是笼统的 allowlist。
- 不做：不把这次成功写成完整 Windows 产品 CU。

### WIN-FIX-004 允许名单进程名

- 现象：Windows Terminal 正在运行，`app windows` 没有它。`app snapshot win:WindowsTerminal:<pid>` 报不在 allowlist。
- 根因：名单是 `windows terminal`，进程名是 `WindowsTerminal`，`contains` 对不上。枚举脚本里又写了 `WindowsTerminal`，所以先被列出再被滤掉。
- 修：允许名单同时认 `WindowsTerminal` 和 `windows terminal`。
- 验收：本机 Windows Terminal 出现在 `app windows`，且不再因名字被拒绝。仍不移动系统光标。

### WIN-FIX-005 截图失败文案

- 现象：桌面桩窗口截图失败时，错误写成 macOS 的 Screen Recording / `VCU_ALLOW_SCREENCAPTURE`。Windows 的 `capture_window` 并不看这个变量，失败原因是没有 HWND 或 PrintWindow 没产出 PNG。
- 浏览器侧：标签未激活时 `screenshot` 拒绝 `select the target tab`；刚打开的后台标签偶发 `image readback failed`，激活后再截可以成功。
- 修：Windows 桌面截图失败要写真正原因，不要提 Screen Recording。浏览器截图保持“先激活再截”，把 readback 失败写成可重试，不改成新的截图栈。
- 验收：桩 pid 截图的错误不再出现 `VCU_ALLOW_SCREENCAPTURE`。真实 cmd / 真实 Notepad pid 仍能出 PNG。

### WIN-FIX-006 UIA 文本编码

- 现象：记事本截图里的中文菜单正常，但 UIA JSON 里的标题和控件名是乱码。
- 修：PowerShell 枚举结果按 UTF-8 回到 Rust，不要经系统 ANSI/GBK 代码页。
- 验收：`app snapshot` 里 Notepad 标题和「文件」一类控件名是可读中文。

### WIN-FIX-007 Windows 上的测试脚本可跑

- 现象：`scripts/poc_cu_d_*.py` 在中文 Windows 上用默认 GBK 读 `vcu` 输出，遇到非 GBK 字节就崩。这让官方 POC 不能当这台机器的验收脚本。
- 另：用 PowerShell `Set-Content -Encoding utf8` 写 `config.json` 会带 BOM，daemon 解析 panic。安装器路径没有这个问题。
- 修：POC 子进程按 UTF-8 解码。文档或安装器若写 config，必须无 BOM。
- 验收：`CU-D-590` / `610` / `620` 在这台机器上不再因 `UnicodeDecodeError` 退出。无 BOM 的临时 `user-dir` 能启动 daemon。

## 5. 明确不在本计划里

| 项 | 原因 |
| --- | --- |
| `TC-B-040` / trusted 手势 / 跨源 iframe 内部点击 | 已诚实拒绝。要做必须用户点名，且不能假成功 |
| `MAC-NEXT` 深 AX | 停放。先过 Accessibility 门禁 |
| `FEISHU-001` | 停放。点名收信人才谈，禁止自动发送 |
| 完整 `vcu session` 产品 Windows CU | 比上面 7 条大。本计划只修真机已证实的洞 |
| Chrome 双浏览器 POC | 用户要求本轮忽略 |
| 微信、系统光标、Edge Allow | 永远不做 |

## 6. 实施顺序

1. WIN-FIX-001 登录态进程扫描。已完成并复测。
2. WIN-FIX-002 禁止静默改绑。已完成并复测。
3. WIN-FIX-003 商店应用真实窗口 pid。
4. WIN-FIX-004 Windows Terminal 名单。
5. WIN-FIX-005 截图错误文案。
6. WIN-FIX-006 UIA 编码。
7. WIN-FIX-007 POC / config 编码。

每条单独提交，带这台 Windows 的复测记录。修完一条再开下一条。未复测前不把对应 POC 改成通过。
