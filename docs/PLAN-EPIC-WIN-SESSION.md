# Windows 产品会话史诗

更新：2026-09-26。作者：本机真机记录。状态：会话点击、1+1、12+7、记忆、科学模式、常用对数和自然对数已复测。不是完整 Windows 产品 CU，也没有新的 GitHub Release。

视觉对齐停在已验证的边界：胶囊、短箭、软雾、物理像素、采样模糊刷新时采的是正后方。网页指针是共享实现。macOS 系统材质模糊没有照搬。`MAC-NEXT` 和 `FEISHU-001` 仍停放。

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

## CU-WIN-SESSION-004 会话悬停只移动 Guide

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 命令：`powershell -File scripts/poc_win_session_hover.ps1`
- 结果：`CU-WIN-SESSION-004 OK win:powershell:11484 ... e2 guide=339,473 dart=78 yellow=154 cursor=1187,239`
- 自建按钮名 `VCU-HOVER-HIT`。`vcu act` 的 `type=hover` 返回 `input_path=guide_hover`。
- 箭尖附近能看到短箭像素，同时还能看到按钮的黄底，说明 Guide 叠在按钮上。
- `GetCursorPos` 前后都是 `1187,239`，系统光标没有动。`os_cursor_used=false`，`hid_injected=false`。
- 没有 SendInput。测完 Abort，只关闭这个自建窗。

## CU-WIN-SESSION-005 会话滚动会移动列表

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 现象：`vcu scroll --session` 对窗体发 `WM_VSCROLL` 就返回 `ok:wm_vscroll`。自建列表的 `LB_GETTOPINDEX` 仍是 0。
- 修：先找带 ScrollPattern 的节点，再找 `LISTBOX`。列表滚动后读回顶行，没变就不算这次成功。找不到子控件时仍对主窗口发 `WM_VSCROLL`，那是记事本 CI 的旧路径，不代表内容一定动了。没有 SendInput。
- 命令：`powershell -File scripts/poc_win_session_scroll.ps1`
- 结果：`CU-WIN-SESSION-005 OK win:powershell:10844 ... wm_vscroll top=0 -> top=8 cursor=1187,239`
- 顶行是独立 `LB_GETTOPINDEX` 读回，不是只看命令回执。系统光标没有动。
- 测完 Abort，只关闭这个自建窗。

## CU-WIN-SESSION-006 会话计算 12+7

状态：**2026-09-26 本机复测通过。** 不是完整计算器产品，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动，结束时只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_expr.ps1`
- 结果：`CU-WIN-SESSION-006 OK win:Calculator:18668 ... 12+7=19`
- 顺序是「一」「二」「加」「七」「等于」。每次都是 `uia_invoke`，`os_cursor_used=false`，`hid_injected=false`。
- 显示从「显示为 0」变成「显示为 19」。
- 同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。没有移动系统光标。

## CU-WIN-SESSION-007 会话存取记忆

状态：**2026-09-26 本机复测通过。** 不是完整计算器产品，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动，结束时只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_memory.ps1`
- 结果：`CU-WIN-SESSION-007 OK win:Calculator:18668 ... MS 5 -> clear -> MR 5`
- 顺序是「五」「记忆存储」「清除」「记忆调用」。每次都是 `uia_invoke`，`os_cursor_used=false`，`hid_injected=false`。
- 显示从「显示为 0」到「显示为 5」，清除后回到「显示为 0」，调用记忆后又是「显示为 5」。
- 同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。没有移动系统光标。
- 只验证了存储和调用。没有覆盖记忆加法、记忆减法、清除所有记忆，也不写成完整记忆产品。

## CU-WIN-SESSION-008 会话记忆加法

状态：**2026-09-26 本机复测通过。** 不是完整计算器产品，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动，结束时只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_madd.ps1`
- 结果：`CU-WIN-SESSION-008 OK win:Calculator:18668 ... M+ 2+3 -> MR 5`
- 顺序是「二」「记忆存储」「三」「记忆加法」「清除」「记忆调用」。每次都是 `uia_invoke`，`os_cursor_used=false`，`hid_injected=false`。
- 清除后显示是「显示为 0」，调用记忆后是「显示为 5」。这是 2 加 3，不是只把当前数字存进去。
- 同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。没有移动系统光标。
- 没有覆盖记忆减法和清除所有记忆，也不写成完整记忆产品。

## CU-WIN-SESSION-009 会话记忆减法

状态：**2026-09-26 本机复测通过。** 不是完整计算器产品，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动，结束时只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_msub.ps1`
- 结果：`CU-WIN-SESSION-009 OK win:Calculator:18668 ... M- 5-2 -> MR 3`
- 顺序是「五」「记忆存储」「二」「记忆减法」「清除」「记忆调用」。每次都是 `uia_invoke`，`os_cursor_used=false`，`hid_injected=false`。
- 清除后显示是「显示为 0」，调用记忆后是「显示为 3」。这是 5 减 2。
- 同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。没有移动系统光标。
- 没有覆盖清除所有记忆，也不写成完整记忆产品。

## CU-WIN-SESSION-010 会话清除全部记忆

状态：**2026-09-26 本机复测通过。** 不是完整计算器产品，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动，结束时只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_mclear.ps1`
- 结果：`CU-WIN-SESSION-010 OK win:Calculator:18668 ... MC then MR stays 0`
- 先「五」「记忆存储」「清除」「记忆调用」，确认显示回到「显示为 5」。再「清除所有记忆」「清除」「记忆调用」，显示仍是「显示为 0」，不是 5。
- 每次成功点击都是 `uia_invoke`，`os_cursor_used=false`，`hid_injected=false`。
- 同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。没有移动系统光标。
- 这只证明清空后调用不会把 5 找回来。不写成完整记忆产品。

## CU-WIN-SESSION-011 会话切换科学模式并输入圆周率

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_sci.ps1`
- 结果：`CU-WIN-SESSION-011 OK win:Calculator:18668 ... scientific 显示为 3.1415926535897932384626433832795 restored`
- 「科学 计算器」是列表项，不是按钮。点击走 `selection_item`，不是 `uia_invoke`，也没有 SendInput。
- 标准模式下没有 π。切到科学模式后按 π，显示是上面的圆周率。再切回标准模式，π 消失。
- 同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。没有移动系统光标。
- 没有覆盖三角函数或对数，不写成完整科学计算器。

## CU-WIN-SESSION-012 会话科学模式计算常用对数

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_log.ps1`
- 结果：`CU-WIN-SESSION-012 OK win:Calculator:18668 ... log10(100)=2 restored`
- 切到科学模式后按「一」「零」「零」「对数」。显示从「显示为 100」变成「显示为 2」。
- 模式项是 `selection_item`，数字和对数是 `uia_invoke`。没有 SendInput，没有移动系统光标。
- 切回标准模式后「对数」按钮消失。同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。
- 没有覆盖自然对数和三角函数。

## CU-WIN-SESSION-013 会话科学模式计算自然对数

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_ln.ps1`
- 结果：`CU-WIN-SESSION-013 OK win:Calculator:18668 ... ln(e)=1 restored`
- 切到科学模式后按「Euler 数字」「自然对数」。显示是「显示为 1」。
- 模式项是 `selection_item`，Euler 数和自然对数是 `uia_invoke`。没有 SendInput，没有移动系统光标。
- 切回标准模式后「自然对数」按钮消失。同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。
- 没有覆盖三角函数。

## 还没做

- 不把这次会话写成完整 `vcu session` 产品 CU。
- 科学模式验证了切换、π、log10(100) 和 ln(e)。没有覆盖三角函数。
- 没有在 100% DPI 和其他机器上复测。
- 不放行整个 `ApplicationFrameHost`。
