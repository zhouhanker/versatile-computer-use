# Windows 产品会话史诗

更新：2026-09-26。作者：本机真机记录。状态：会话点击、1+1、12+7、记忆、科学模式、对数、sin、cos、tan、弧度模式、自建窗口截图、等待、写入、按钮点击、同名消歧、提取、按角色等待、复选框、单选按钮和下拉列表已复测。不是完整 Windows 产品 CU，也没有新的 GitHub Release。

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

## CU-WIN-SESSION-014 会话科学模式计算正弦

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_sin.ps1`
- 结果：`CU-WIN-SESSION-014 OK win:Calculator:18668 ... sin(30)=0.5 restored`
- 「三角学」外层列表项没有 Invoke。里面的同名按钮有 TogglePattern。点击走 `toggle`，展开后按「三」「零」「正弦」。
- 显示是「显示为 0.5」。如果第一次不是 0.5，会按一次「度数切换」再算。这次第一次就是 0.5。
- 没有 SendInput，没有移动系统光标。同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。
- 没有覆盖余弦、正切和弧度模式的完整矩阵。

## CU-WIN-SESSION-015 会话科学模式计算余弦

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_cos.ps1`
- 结果：`CU-WIN-SESSION-015 OK win:Calculator:18668 ... cos(0)=1 restored`
- 切到科学模式后按「零」，用 `toggle` 展开「三角学」，再按「余弦」。显示是「显示为 1」。
- 没有 SendInput，没有移动系统光标。同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。
- 没有覆盖正切和弧度模式。

## CU-WIN-SESSION-016 会话科学模式计算正切

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_tan.ps1`
- 结果：`CU-WIN-SESSION-016 OK win:Calculator:18668 ... tan(45)=1 restored`
- 切到科学模式后按「四」「五」，确认「显示为 45」，用 `toggle` 展开「三角学」，再按「正切」。显示是「显示为 1」。
- 没有 SendInput，没有移动系统光标。同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。
- 没有覆盖弧度模式。

## CU-WIN-SESSION-017 会话科学模式比较角度和弧度

状态：**2026-09-26 本机复测通过。** 不是完整科学计算器，也不是完整 Windows 产品 CU。

- 开始前没有已打开的计算器窗口。脚本自己启动。测完切回标准模式，再只关闭 CalculatorApp。
- 命令：`powershell -File scripts/poc_win_session_calc_rad.ps1`
- 结果：`CU-WIN-SESSION-017 OK win:Calculator:18668 ... degree=显示为 0.05480366514878953088774871353983 radian=显示为 0 restored`
- 切到科学模式后按 π，用 `toggle` 展开「三角学」，再按「正弦」。然后按名称包含「度」且角色是 Button 的控件切换角度，再算一次。
- 两次结果不同。角度是「显示为 0.05480366514878953088774871353983」，弧度是「显示为 0」。
- 三角菜单打开时「清除」可能不在树里。算完先把「三角学」再 toggle 一次，再找「清除」或「清除条目」。
- 没有 SendInput，没有移动系统光标。同一 pid 的 `win:ApplicationFrameHost` 仍然被拒绝。
- 没有覆盖完整科学函数矩阵。

## CU-WIN-SESSION-018 会话截图自建窗口

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个 420x280 的品红窗口，标题 `VCU-SHOT-018`。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_shot.ps1`
- 结果：`CU-WIN-SESSION-018 OK win:powershell:19184 ... 420x280 bytes=3120 pixel=220,30,160 cursor=1187,239`
- `PrintWindow` 对这个 WinForms 窗口给出全黑图。空白时只复制这块窗口矩形，不扫整张桌面。遮挡物可能出现在图里。没有 SendInput，没有移动系统光标。
- 中心像素是窗口颜色 `220,30,160`，不是黑图。PNG 能看到标题 `VCU-SHOT-018`。
- 不做：不把这次写成被挡住时仍能采到窗口内容，也不写成完整产品 CU。


## CU-WIN-SESSION-019 会话等待控件名

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个窗口，里面有按钮「VCU-WAIT-019」。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_wait.ps1`
- 结果：`CU-WIN-SESSION-019 OK win:powershell:11804 ... ref=e1 miss=timeout cursor=1187,239`
- `vcu wait --name` 把名字交给已有的 `scene_wait`。找到时 `input_path=scene_wait`，`os_cursor_used=false`，并带回 `found_ref`。
- 不存在的名字在 500ms 内诚实超时，不会报成功。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成完整产品 CU，也不放宽超时。


## CU-WIN-SESSION-020 会话等待文本值

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个窗口，文本框内容是「VCU-VAL-020」，窗口标题不是这个值。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_wait_value.ps1`
- 结果：`CU-WIN-SESSION-020 OK win:powershell:3524 ... ref=e2 miss=timeout cursor=1187,239`
- `vcu wait --value` 走已有的 `scene_wait`。找到的是文本框 `e2`，不是窗口标题。`os_cursor_used=false`。
- 不存在的值在 500ms 内诚实超时。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成完整产品 CU。


## CU-WIN-SESSION-021 会话写入文本框后再等待

状态：**2026-09-26 本机复测通过。** 不是 ValuePattern，也不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个窗口，文本框先写着「VCU-BOX-021」。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_type_wait.ps1`
- 结果：`CU-WIN-SESSION-021 OK win:powershell:13808 ... ref=e2 path=wm_settext value=VCU-TYPED-021 cursor=1187,239`
- 先按名字找到文本框，再 `vcu type` 写入新值。路径是 `wm_settext`，不是剪贴板粘贴，也不是 ValuePattern。
- 随后 `vcu wait --value` 读到「VCU-TYPED-021」。`os_cursor_used=false`，`hid_injected=false`。没有 SendInput，没有移动系统光标。
- 不做：不把 `WM_SETTEXT` 写成 ValuePattern，也不写成完整产品 CU。


## CU-WIN-SESSION-022 会话点击自建按钮

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个窗口，按钮名是「VCU-CLICK-022」，窗口标题是「VCU-CLICK-HOST」，避免名字撞上窗口。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_click_label.ps1`
- 结果：`CU-WIN-SESSION-022 OK win:powershell:9444 ... ref=e2 path=bm_click label=VCU-CLICKED-022 cursor=1187,239`
- 点击走 `bm_click`，不是 SendInput。按钮的 Click 处理把标签改成「VCU-CLICKED-022」，随后 `vcu wait --name` 读到。`os_cursor_used=false`，`hid_injected=false`。
- 窗口标题如果和按钮同名，`wait --name` 会先命中窗口。这次故意用了不同的标题。
- 不做：不把这次写成完整产品 CU，也不移动系统光标。


## CU-WIN-SESSION-023 同名时优先点控件

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 022 里窗口标题如果和按钮同名，`wait --name` 会先返回窗口，接着 `bm_click` 打在窗口上，标签不变。
- 现在没有指定 ref 时，同名匹配优先返回非窗口控件。显式 `--ref` 仍按请求的节点返回。窗口角色只认 `window`、`AXWindow` 和 `ControlType.Window`，不把 `WindowsForms` 当成窗口。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，标题和按钮都是「VCU-SAME-023」。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_same_name.ps1`
- 结果：`CU-WIN-SESSION-023 OK win:powershell:15664 ... ref=e2 path=bm_click label=VCU-SAME-CLICKED cursor=1187,239`
- 返回的是 `e2`，不是窗口 `e1`。点击后标签变成「VCU-SAME-CLICKED」。没有 SendInput，没有移动系统光标。
- 单测 `scene_wait_match_filters_ref_name_role` 覆盖同名窗口和按钮。
- 不做：不把这次写成完整产品 CU。只有窗口自己匹配时，仍返回窗口。


## CU-WIN-SESSION-024 会话提取文本框的值

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个窗口，标题是「VCU-EXTRACT-HOST」，文本框的值是「VCU-EXTRACT-024」。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_extract.ps1`
- 结果：`CU-WIN-SESSION-024 OK win:powershell:11760 ... ref=e2 value=VCU-EXTRACT-024 source=desktop.scene miss=0 cursor=1187,239`
- `vcu extract --selector` 按名字或值匹配场景节点。命中的是文本框 `e2`，`source=desktop.scene`。角色字符串里有 `WindowsForms`，那不是窗口。
- 不存在的选择器返回空列表，`ok` 仍是 true，不是假命中。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成完整产品 CU，也不把 `WindowsForms` 当成窗口。


## CU-WIN-SESSION-026 会话按角色等待

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 开始前没有借用用户已开的记事本或计算器。脚本自己启动一个窗口，按钮名是「VCU-ROLE-026」，标题是「VCU-ROLE-HOST」。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_wait_role.ps1`
- 结果：`CU-WIN-SESSION-026 OK win:powershell:7708 ... ref=e2 role=ControlType.Pane miss=window cursor=1187,239`
- `vcu wait --role` 要和场景里的角色字符串完全一致，不区分大小写。这个 WinForms 按钮的角色是 `ControlType.Pane`，不是 `ControlType.Button`。
- 用 `ControlType.Window` 去等这个按钮会超时，不会误命中。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成完整产品 CU。WinForms 复选框的勾选状态还不能靠会话点击翻转。


## CU-WIN-SESSION-027 会话点击翻转复选框

状态：**2026-09-26 本机复测通过。** 不是 TogglePattern，也不是完整 Windows 产品 CU。

- WinForms 复选框没有 TogglePattern。`BM_GETCHECK` 也读不到勾选状态。场景值改用 `IAccessible`：角色 44 是复选框，状态位 16 表示已勾选。普通按钮不会被标成复选框。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有一个按钮和一个未勾选的复选框。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_checkbox.ps1`
- 结果：`CU-WIN-SESSION-027 OK win:powershell:1300 ... ref=e3 path=bm_click on-then-off cursor=1187,239`
- 点击路径是 `bm_click`。第一次点击后等到 `toggle-on`，第二次回到 `toggle-off`。按钮的值里没有 `toggle-`。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 TogglePattern，也不写成完整产品 CU。


## CU-WIN-SESSION-028 会话点击切换单选按钮

状态：**2026-09-26 本机复测通过。** 不是 TogglePattern，也不是完整 Windows 产品 CU。

- 单选按钮和复选框一样走 `IAccessible`。角色 45 是单选按钮，状态位 16 表示选中。点击路径仍是 `bm_click`。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，A 初始选中，B 未选中。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_radio.ps1`
- 结果：`CU-WIN-SESSION-028 OK win:powershell:12836 ... ref=e3 path=bm_click b-on a-off cursor=1187,239`
- 点击 B 之后，B 是 `toggle-on`，A 变成 `toggle-off`。互斥成立。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 TogglePattern，也不写成完整产品 CU。


## CU-WIN-SESSION-029 会话选择下拉列表项

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 下拉列表点一下不会展开，也不会改选中项。`vcu type` 在组合框上按条目文字调用 `CB_SETCURSEL`。没有匹配的条目不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，列表里有 A 和 B，初始选中 A。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_combo.ps1`
- 结果：`CU-WIN-SESSION-029 OK win:powershell:13856 ... ref=e2 path=combo_select selected=VCU-COMBO-B cursor=1187,239`
- 写入后场景名字变成 `VCU-COMBO-B`。不存在的条目被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成展开下拉层后点选，也不写成完整产品 CU。

## 还没做

- 不把这次会话写成完整 `vcu session` 产品 CU。
- 科学模式验证了切换、π、log10(100)、ln(e)、sin(30°)、cos(0)、tan(45°)，以及 sin(π) 在角度和弧度下的不同结果。没有覆盖完整科学函数矩阵。
- 会话截图在 `PrintWindow` 为空白时复制窗口矩形。窗口被挡住时，图里可能是挡住它的东西。
- 会话等待可以按控件名找到自建按钮，也可以按文本框的值找到控件。缺失时超时。
- 会话可以向自建文本框写入，再按新值等到。这是 `wm_settext`，不是 ValuePattern。
- 会话可以点击自建按钮并看到标签变化。同名时优先返回控件；显式 ref 仍按请求的节点返回。
- 会话可以按选择器提取自建文本框的值。没有命中时是空列表，不是假成功。
- 会话可以按完整角色字符串等待。WinForms 按钮这里是 `ControlType.Pane`。会话点击可以翻转自建复选框，也可以切换单选按钮并保持互斥。状态来自 `IAccessible`，不是 TogglePattern。
- 会话可以向自建下拉列表按条目文字选中。这是 `combo_select`，不是展开后点选。
- 没有在 100% DPI 和其他机器上复测。
- 不放行整个 `ApplicationFrameHost`。
