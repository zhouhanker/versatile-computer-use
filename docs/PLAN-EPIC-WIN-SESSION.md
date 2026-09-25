# Windows 产品会话史诗

更新：2026-09-26。作者：本机真机记录。状态：第一扇门已复测。WIN-FIX-009 已复测 powershell 图形编辑框。不是完整 Windows 产品 CU，也没有新的 GitHub Release。

视觉对齐停在已验证的边界：胶囊、短箭、软雾、物理像素、网页指针是共享实现。macOS 系统材质模糊没有照搬。`MAC-NEXT` 和 `FEISHU-001` 仍停放。

## CU-WIN-SESSION-001 会话能绑计算器并拉起 Stage

状态：**2026-09-26 本机复测通过。**

- 命令：`powershell -File scripts/poc_win_session_calc.ps1`
- 结果：`CU-WIN-SESSION-001 OK win:Calculator:18668`
- 会话 `stage_hud=true`，`stage_presenter=winforms`，`os_cursor=deny`
- Abort 后 `hud=false`
- 没有点计算器按钮，没有移动系统光标，测完关闭了 CalculatorApp


## CU-WIN-SESSION-002 会话点击数字七

状态：**2026-09-26 本机复测通过。**

- 命令：`powershell -File scripts/poc_win_session_calc_click.ps1`
- 结果：`CU-WIN-SESSION-002 OK win:Calculator:18668 ... e50 0->7`
- 点击前显示「显示为 0」，点击「七」后是「显示为 7」
- `input_path=uia_invoke`，`os_cursor_used=false`，`hid_injected=false`
- 同一 pid 的 `win:ApplicationFrameHost:<pid>` 仍然被拒绝
- 没有移动系统光标，测完关闭了 CalculatorApp


## CU-WIN-SESSION-003 会话计算 1+1

状态：**2026-09-26 本机复测通过。**

- 命令：`powershell -File scripts/poc_win_session_calc_add.ps1`
- 结果：`CU-WIN-SESSION-003 OK win:Calculator:18668 ... 1+1=2`
- 顺序按「一」「加」「一」「等于」。每次都是 `uia_invoke`，`os_cursor_used=false`，`hid_injected=false`
- 显示从「显示为 0」变成「显示为 2」
- 没有移动系统光标，测完关闭了 CalculatorApp

## CU-WIN-FIX-009 会话写入 powershell 图形编辑框

状态：**2026-09-26 本机复测通过。** 详见 [PLAN-WINDOWS-FIX.md](PLAN-WINDOWS-FIX.md)。

- 命令：`powershell -File scripts/poc_win_session_own_edit.ps1`
- 结果：`CU-WIN-FIX-009 OK win:powershell:7576 ... e2 wm_settext`
- 自建窗口标题 `VCU-OWN-EDIT-009`。独立 `WM_GETTEXT` 读回标记，不是只看命令回执。
- `input_path=wm_settext`，`os_cursor_used=false`，`hid_injected=false`
- 这个 WinForms 文本框没有 ValuePattern。`WM_SETTEXT` 成功不等于 ValuePattern，也不等于完整产品 CU。
- 没有移动系统光标。测完只关闭脚本自己启动的窗口。

## 还没做

- 不把这次会话写成完整 `vcu session` 产品 CU。
- 只验证了 1+1。没有覆盖科学模式、内存键或连续长表达式，也不写成完整计算器产品。
- 没有在 100% DPI 和其他机器上复测。
- 不放行整个 `ApplicationFrameHost`。
