# Windows 真机修复计划

更新：2026-09-26。作者：本机真机测试记录。状态：WIN-FIX-001 至 006 已提交并复测通过。WIN-FIX-007 已复测：POC 不再因 GBK 的 UnicodeDecodeError 退出。CU-D-610 仍失败于截图 dry-run，不算整项通过。计算器宿主窗口仍未放行。没有新的 GitHub Release。

001–006 的产品修复已在源码里并推送。本轮 007 复测用的是本仓库 debug 构建 `target/debug/vcu.exe`，daemon 也是这份 debug 构建，不是已安装的 Release 包。`vcu --version` 仍打印 crate `0.1.0`。不要把本计划写成已完成，也不要把它说成完整 Windows 产品 CU。

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

001–006 已修代码并推送，没有发新的 GitHub Release。007 只改测试脚本的解码。

| 范围 | 结果 |
| --- | --- |
| 安装与 daemon | Release 已装到 `~/.local/bin`，daemon 可起。`doctor` 诚实写出不是产品 Windows CU |
| Edge 扩展 | ping / tabs / 打开关闭 / extract / 选择器点击 / 输入 / 悬停 / 滚动 / 等待 / 原生 select / 截图 / 像素点击 / 标签组，在自建 `127.0.0.1` 页上有效 |
| 桌面传统窗口 | 找对真实 pid 后，记事本 `uia_set_value` 可见文字；cmd 截图成功；`open_path` 为 `explorer_open`；设置只观察；自建按钮可点；Abort 能拆 HUD |
| 策略门禁 | 微信 `AppDenied`；不抢焦点拒绝；无 `confirm_send` 的回车拒绝；跨源 iframe 像素点击拒绝 |

007 的脚本解码与本文一起提交。`.gitignore` 里的 AWR 本地目录与这些缺陷无关，不放进这次提交。

## 3. 不要当成待修 bug

这些是当前设计或已经诚实的失败，修计划不把它们改成“成功”：

- DOM 事件 `trusted=false`。
- 跨源 iframe 像素点击返回 `unsupported_point_target` / `cross-origin iframe requires trusted input`。这是 `TC-B-040` 的诚实缺口，不是 Windows 回归。
- 微信、系统光标、无确认回车被拒绝。
- `doctor.windows_desktop_scope=warn`。
- 探测里对桌面发送 `type=reveal_path` 得到 `NotImplemented`。动作名是 `reveal`，不是 `reveal_path`。未按正确名字复测，**不列入必改**。

## 4. 需要 Fix 的问题

一次只开一条。下面已标状态的条目不要再写成未修。

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

状态：**记事本桩进程已修，2026-09-26 本机 `scripts/poc_cu_d_090.ps1` 通过。** 计算器仍拒绝，没有放行整个 `ApplicationFrameHost`。

- 现象：`Start-Process notepad.exe` 返回的进程没有窗口，而且经常马上退出。真正窗口在另一个 `Notepad` 进程。
- 修：请求 `win:notepad:<桩pid>` 时，如果当前只有一个可见记事本窗口，会话和后续 snapshot/type 绑定到那个窗口。有两个及以上记事本时仍然拒绝，不猜。配置读取会去掉 UTF-8 BOM，否则官方 POC 写的 `config.json` 会让 daemon 解析失败。
- 验收：`poc_cu_d_090.ps1` 输出 `STAGE_OK`、`SNAP_OK source=uia_scene ref=e6`、`TYPE_OK path=uia_set_value os_cursor_used=False`、`CU-D-090 OK`。单测覆盖桩 pid 绑到唯一记事本，以及两个记事本时拒绝。
- 计算器：窗口仍在 `ApplicationFrameHost`，标题「计算器」。`app snapshot win:ApplicationFrameHost:<pid>` 继续报该进程不在允许名单。没有为了计算器放行整个宿主进程。
- 不做：不把这次成功写成完整 Windows 产品 CU。

### WIN-FIX-004 允许名单进程名

状态：**已修，2026-09-26 本机复测通过。**

- 现象：Windows Terminal 正在运行，`app windows` 没有它。`app snapshot win:WindowsTerminal:<pid>` 报不在 allowlist。
- 根因：名单是 `windows terminal`，进程名是 `WindowsTerminal`，`contains` 对不上。
- 修：允许名单同时认 `WindowsTerminal`。
- 验收：单测 `parse_process_list_keeps_windowsterminal` 通过。本机 pid 19364 的 `app snapshot` 返回 `ok: true`，`process="WindowsTerminal"`，19 个 UIA 节点。没有移动系统光标。中文标题乱码仍属于 WIN-FIX-006。

### WIN-FIX-005 截图失败文案

状态：**已修，2026-09-26 本机复测通过。**

- 现象：桌面桩窗口截图失败时，错误写成 macOS 的 Screen Recording / `VCU_ALLOW_SCREENCAPTURE`。Windows 的 `capture_window` 并不看这个变量。
- 修：Windows 失败返回 `Windows PrintWindow did not produce a PNG (no visible HWND)`，并写明这不是 macOS 屏幕录制权限。浏览器 `image readback` 失败会提示重试 `observe --tab`，不新做截图栈。
- 验收：`app snapshot win:notepad:1 --pixels` 的错误不含 `VCU_ALLOW_SCREENCAPTURE`。通过 conhost 打开的 cmd（pid 13956）截图为 992×517、9899 字节。单测 `windows_backend_platform_and_denials` 通过。
- 未宣称：直接 `Start-Process cmd.exe` 且主窗口句柄为 0 时，仍可能得到 1×1 PNG。那是控制台窗口归属问题，不是这条文案修复的范围。

### WIN-FIX-006 UIA 文本编码

状态：**已修，2026-09-26 本机复测通过。**

- 现象：PowerShell 5.1 把管道输出按系统 GBK 写出，Rust 用 UTF-8 解码，UIA 中文变成乱码。
- 修：所有 Windows PowerShell 调用前设置无 BOM 的 UTF-8 `OutputEncoding`，读回时去掉 BOM。
- 验收：单测 `decode_powershell_output_keeps_utf8_chinese` 通过。记事本 `app snapshot` 含「文件」「无标题」，没有 U+FFFD。Windows Terminal pid 19364 的标题「检查rtk和awr激活状态」可读，`replacement=false`。

### WIN-FIX-007 Windows 上的测试脚本可跑

状态：**解码崩溃已修，2026-09-26 本机复测通过。** 不代表 `CU-D-610` 整项通过，也没有发新的 GitHub Release。

- 现象：`scripts/poc_*.py` 在中文 Windows 上用默认 GBK 读 `vcu` 输出，遇到非 GBK 字节就崩。这让官方 POC 不能当这台机器的验收脚本。`poc_cu_d_590.py` 还会在空路径上调用 `Path.with_suffix`。
- 另：用 PowerShell `Set-Content -Encoding utf8` 写 `config.json` 会带 BOM，daemon 解析 panic。安装器路径没有这个问题。配置读取去 BOM 已在 WIN-FIX-003 落地。
- 修：POC 子进程按 UTF-8 解码，非法字节替换而不是抛出。`poc_cu_d_590.py` 对空路径直接返回，sidecar JSON 按 UTF-8 读取。
- 验收：`VCU=target/debug/vcu.exe`。`poc_cu_d_590.py` 退出 0，`CU-D-590 OK 1013794778 0->1`。`poc_cu_d_620.py` 退出 0，`CU-D-620 OK 1013794782 hovered=1`。两者都没有 `UnicodeDecodeError`。
- 未宣称：`poc_cu_d_610.py` 退出 1，也没有 `UnicodeDecodeError`。DOM 输入和滚动成功（`typed_value=vcu-d-610`，`scroll_y=y=900`，`groups_ok=true`，`os_cursor_used=false`）。失败点是 `capture_dry_run_ok: false`。2026-09-26 复现为扩展 `stale_viewport`：滚动后 `scroll_y=900`，旧截图不能再点。这是共享扩展的安全拒绝，不是 Windows 回归，不能放宽校验。这不是 007 的验收范围，也不写成 610 通过。


### WIN-FIX-008 计算器窗口在 ApplicationFrameHost 上

状态：**已修，2026-09-26 本机复测通过。** 没有放行整个宿主进程，也没有新的 GitHub Release。

- 现象：Windows 11 计算器窗口属于 `ApplicationFrameHost`，标题「计算器」。`CalculatorApp` 没有窗口，snapshot 是 0 个节点。直接 snapshot 宿主进程报不在允许名单。
- 修：只在宿主窗口标题是计算器时，把该 pid 记成 `win:Calculator:<pid>`。宿主进程本身仍拒绝。无窗口的 `CalculatorApp` 桩在已有可见计算器窗口时不再列出。
- 验收：单测 `frame_host_calculator_aliases_without_allowing_the_host` 通过。本机 pid 18668 列出为 `win:Calculator:18668`，snapshot `elements=53`，含「计算器」「关闭 计算器」。`win:ApplicationFrameHost:18668` 仍是 `FocusPolicyViolation`。没有移动系统光标，没有点计算器按钮。

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
3. WIN-FIX-003 商店应用真实窗口 pid。记事本桩进程已完成并复测。计算器宿主未放行。
4. WIN-FIX-004 Windows Terminal 名单。已完成并复测。
5. WIN-FIX-005 截图错误文案。已完成并复测。
6. WIN-FIX-006 UIA 编码。已完成并复测。
7. WIN-FIX-007 POC 解码。已复测：590 与 620 通过；610 不再因 `UnicodeDecodeError` 退出，但仍因截图 dry-run 失败。

每条单独提交，带这台 Windows 的复测记录。修完一条再开下一条。未复测前不把对应 POC 改成通过。
