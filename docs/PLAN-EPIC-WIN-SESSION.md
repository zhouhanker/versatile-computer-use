# Windows 产品会话史诗

更新：2026-09-26。作者：本机真机记录。状态：会话点击、1+1、12+7、记忆、科学模式、对数、sin、cos、tan、弧度模式、自建窗口截图、空白遮挡拒绝、等待、写入、按钮点击、按钮像素点击、同名消歧、提取、按角色等待、复选框、单选按钮、单选按钮点击、下拉列表、列表框、列表行点击、滑块、标签页、树节点、树节点文字点击、树展开、树折叠、列表视图、进度条、勾选列表和取消勾选、勾选框点击、取消勾选点击、日期和数字框和列表视图勾选、列表视图复选框像素点击、列表视图取消勾选像素点击、列表视图行像素点击、链接点击、菜单项点击和场景已复测。不是完整 Windows 产品 CU，也没有新的 GitHub Release。

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


## CU-WIN-SESSION-030 会话选择列表框的一行

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 列表框点一下不会按名字选中。`vcu type` 用 `LB_SETCURSEL` 选中同名行，再发反射的选择变化，让 WinForms 的 `SelectedItem` 跟着变。没有匹配的行不会报成功。
- 场景值会带上当前选中行的文字，所以等待能读到选中项。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，列表里有 A 和 B，初始选中 A。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_list.ps1`
- 结果：`CU-WIN-SESSION-030 OK win:powershell:15776 ... ref=e2 path=list_select selected=VCU-LIST-B cursor=1187,239`
- 写入后等到 `VCU-LIST-B`。不存在的行被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点某一行像素，也不写成完整产品 CU。



## CU-WIN-SESSION-031 会话设置滑块位置

状态：**2026-09-26 本机复测通过。** 不是 RangeValuePattern，也不是完整 Windows 产品 CU。

- WinForms 滑块点一下不会按数字改值。`vcu type` 在类名含 trackbar 的控件上解析整数，用 `TBM_SETPOS` 写入，再发反射滚动，让 WinForms 的 `Value` 跟着变。超出范围或不是整数不会报成功。
- 场景值带 `track=` 加当前位置，所以等待能读到滑块位置。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，滑块范围是 0 到 100，初始是 10。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_track.ps1`
- 结果：`CU-WIN-SESSION-031 OK win:powershell:18460 ... ref=e2 path=track_select value=40 cursor=1187,239`
- 写入后标签变成 `VCU-TRACK-40`，场景值是 `track=40`。超出范围和非整数被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 RangeValuePattern，也不写成拖动滑块拖块，也不写成完整产品 CU。


## CU-WIN-SESSION-032 会话按名字选择标签页

状态：**2026-09-26 本机复测通过。** 不是展开后点标签，也不是完整 Windows 产品 CU。

- 未选中的标签页不在 UIA 树里。`vcu type` 用辅助功能按名字找到标签，选中后再发反射的选择变化，让 WinForms 的 `SelectedIndexChanged` 跟着跑。没有同名标签不会报成功。
- 场景值带 `tab=` 加当前选中标签的名字。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有 VCU-TAB-A 和 VCU-TAB-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tab.ps1`
- 结果：`CU-WIN-SESSION-032 OK win:powershell:18444 ... ref=e3 path=tab_select value=VCU-TAB-B cursor=1187,239`
- 选中后标签变成 `VCU-TAB-SHOW-B`，场景值是 `tab=VCU-TAB-B`。不存在的标签被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点标签像素，也不写成完整产品 CU。


## CU-WIN-SESSION-033 会话按名字选中树节点

状态：**2026-09-26 本机复测通过。** 不是点节点像素，也不是完整 Windows 产品 CU。

- 树节点不在 UIA 子项里。`vcu type` 在类名含 TreeView 的控件上按名字查节点，用 `TVM_SELECTITEM` 选中。这会让 WinForms 的 `AfterSelect` 跟着跑。没有同名节点不会报成功。
- 场景值带 `tree=` 加当前选中节点的文字。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有 VCU-TREE-A 和子节点 VCU-TREE-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tree.ps1`
- 结果：`CU-WIN-SESSION-033 OK win:powershell:11472 ... ref=e3 path=tree_select value=VCU-TREE-B cursor=1187,239`
- 选中后标签变成 `VCU-TREE-SHOW-B`，场景值是 `tree=VCU-TREE-B`。不存在的节点被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点节点像素，也不写成完整产品 CU。


## CU-WIN-SESSION-034 会话按名字选中列表视图的一行

状态：**2026-09-26 本机复测通过。** 不是点行像素，也不是完整 Windows 产品 CU。

- 列表视图不是列表框。`vcu type` 在类名含 ListView 的控件上按行文字查找，先清掉其他行的选中，再用 `LVM_SETITEMSTATE` 选中这一行。这会让 WinForms 的选择事件跟着跑。没有同名行不会报成功。
- 场景值带 `lv=` 加当前选中行的文字。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有 VCU-LV-A 和 VCU-LV-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_listview.ps1`
- 结果：`CU-WIN-SESSION-034 OK win:powershell:2792 ... ref=e3 path=listview_select value=VCU-LV-B cursor=1187,239`
- 选中后标签变成 `VCU-LV-PICKED-B`，场景值是 `lv=VCU-LV-B`。不存在的行被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点行像素，也不写成列表框的 `list_select`，也不写成完整产品 CU。


## CU-WIN-SESSION-035 会话设置进度条位置

状态：**2026-09-26 本机复测通过。** 不是托管的 `ProgressBar.Value`，也不是完整 Windows 产品 CU。

- `vcu type` 在类名含 Progress 的控件上解析整数，用 `PBM_SETPOS` 写入，再用 `PBM_GETPOS` 读回。读回不一致就不报成功。不是整数也不报成功。
- 场景值带 `progress=` 加原生位置。这不代表托管属性已经改变。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，范围是 0 到 100，初始是 10。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_progress.ps1`
- 结果：`CU-WIN-SESSION-035 OK win:powershell:7548 ... ref=e2 path=progress_set value=40 cursor=1187,239`
- 写入后场景值是 `progress=40`。`400` 和 `nope` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成托管 `Value` 已经变化，也不写成完整产品 CU。


## CU-WIN-SESSION-036 会话按名字展开树节点

状态：**2026-09-26 本机复测通过。** 不是点节点像素，也不是完整 Windows 产品 CU。

- 树节点展开不是点展开图标。`vcu type` 在类名含 TreeView 的控件上，文本以 `expand:` 开头时按后面的名字查节点，用 `TVM_EXPAND`（`0x1102`，`TVE_EXPAND=2`）展开。读回必须带 `TVIS_EXPANDED`（`0x20`）。这会让 WinForms 的 `AfterExpand` 跟着跑。没有同名节点不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，根节点是 VCU-EXP-A，子节点是 VCU-EXP-B，初始未展开。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tree_expand.ps1`
- 结果：`CU-WIN-SESSION-036 OK win:powershell:12240 01M3DHBAP2TS79R1128HRKY8PY ref=e3 path=tree_expand value=VCU-EXP-A cursor=1187,239`
- 展开后标签变成 `VCU-EXP-OPEN`。`expand:VCU-EXP-MISSING` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点节点像素，也不写成 `tree_select`，也不写成完整产品 CU。



## CU-WIN-SESSION-037 会话按名字折叠树节点

状态：**2026-09-26 本机复测通过。** 不是点折叠图标，也不是完整 Windows 产品 CU。

- 只发 `TVM_EXPAND` / `TVE_COLLAPSE` 能清掉 `TVIS_EXPANDED`，但 WinForms 不会跑 `AfterCollapse`。不能把这种结果写成事件已经发生。
- `vcu type` 在类名含 TreeView 的控件上，文本以 `collapse:` 开头时按后面的名字查节点。节点必须已经展开。先发 `TVE_COLLAPSE`（`0x1102`，`wParam=1`），读回不再带 `TVIS_EXPANDED`，再向控件反射 `TVN_ITEMEXPANDEDW`（`0x204E`，码 `-455`，状态不含展开位）。这会让 `AfterCollapse` 跟着跑。没有同名节点不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，根节点 VCU-COL-A 已展开，子节点是 VCU-COL-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tree_collapse.ps1`
- 结果：`CU-WIN-SESSION-037 OK win:powershell:17308 01M3DJ149EVV4APX07B2CMEKFG ref=e3 path=tree_collapse value=VCU-COL-A cursor=1187,239`
- 折叠后标签变成 `VCU-COL-CLOSED`。`collapse:VCU-COL-MISSING` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点折叠图标，也不写成 `tree_select` 或 `tree_expand`，也不写成完整产品 CU。



## CU-WIN-SESSION-038 会话按名字勾选列表项

状态：**2026-09-26 本机复测通过。** 不是点复选框像素，也不是列表框的 `list_select`，也不是完整 Windows 产品 CU。

- WinForms 勾选列表的项目不在 UIA 子项里。`vcu type` 在类名含 ListBox 的控件上，文本以 `check:` 开头时按行文字查找。用注册消息 `LBC_SETCHECKSTATE` 勾选，再用 `LBC_GETCHECKSTATE` 读回。读回不是已勾选就不报成功。这会让 `ItemCheck` 跟着跑。没有同名行不会报成功。普通列表框不接这个消息，也不会被写成 `list_select`。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有未勾选的 VCU-CHK-A 和 VCU-CHK-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_checked_list.ps1`
- 结果：`CU-WIN-SESSION-038 OK win:powershell:1464 01M3DJDC1HJ1M2Y1YFQPN52NYG ref=e1 path=check_set value=VCU-CHK-B cursor=1187,239`
- 勾选后标签变成 `VCU-CHK-ON-B`。只有 B 的 `ItemCheck` 会改这个标签。`check:VCU-CHK-MISSING` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点复选框像素，也不写成取消勾选，也不写成完整产品 CU。



## CU-WIN-SESSION-039 会话按名字取消勾选列表项

状态：**2026-09-26 本机复测通过。** 不是点复选框像素，也不是 `check_set` 或 `list_select`，也不是完整 Windows 产品 CU。

- `vcu type` 在类名含 ListBox 的控件上，文本以 `uncheck:` 开头时按行文字查找。该行必须已经勾选。用 `LBC_SETCHECKSTATE` 把状态设成 0，再用 `LBC_GETCHECKSTATE` 读回。读回不是未勾选就不报成功。这会让 `ItemCheck` 跟着跑。没有同名行不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-CHK-A 未勾选，VCU-CHK-B 已勾选。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_unchecked_list.ps1`
- 结果：`CU-WIN-SESSION-039 OK win:powershell:18556 01M3DJKX6FBW8EAJ5MNCFGNC8C ref=e1 path=uncheck_set value=VCU-CHK-B cursor=1187,239`
- 取消后标签变成 `VCU-CHK-OFF-B`。只有 B 的取消 `ItemCheck` 会改这个标签。`uncheck:VCU-CHK-MISSING` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点复选框像素，也不写成 `check_set`，也不写成完整产品 CU。



## CU-WIN-SESSION-040 会话设置自建日期

状态：**2026-09-26 本机复测通过。** 不是点日历像素，也不是设置时刻，也不是完整 Windows 产品 CU。

- 只发 `DTM_SETSYSTEMTIME` 能改控件上的日期，但 WinForms 不会跑 `ValueChanged`。不能把这种结果写成事件已经发生。
- `vcu type` 在类名含 SysDateTimePick32 的控件上，文本以 `date:` 开头时解析 `YYYY-MM-DD`。格式不对或不是真实日期就拒绝。日期已经是这一天也拒绝。先写入并用 `DTM_GETSYSTEMTIME` 读回，读回不一致就不报成功。再反射 `DTN_DATETIMECHANGE`，让托管的 `Value` 变成这一天并跑 `ValueChanged`。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，初始日期是 2026-01-02。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_date.ps1`
- 结果：`CU-WIN-SESSION-040 OK win:powershell:8844 01M3DK28BJJNC13NENMC01ATY8 ref=e1 path=date_set value=2026-03-15 cursor=1187,239`
- 事件把标签改成 `VCU-DTP-2026-03-15`，这是托管 `Value` 的日期，不只是原生控件文字。再写同一天、`date:2026-02-31` 和 `date:nope` 都被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点日历像素，也不写成设置时分秒，也不写成完整产品 CU。



## CU-WIN-SESSION-041 会话设置自建数字框

状态：**2026-09-26 本机复测通过。** 不是点上下箭头，也不是普通文本框的 `wm_settext`，也不是完整 Windows 产品 CU。

- 只对编辑框发 `WM_SETTEXT` 不会让数字框跑 `ValueChanged`。那种结果不能报成功。
- `vcu type` 文本以 `number:` 开头时，只处理旋转按钮里的编辑框：它的父窗口不是会话主窗口，并且父窗口还有一个 Window 类按钮。整数必须和写入文本一致。当前已经是这个数就拒绝。写入后发 `WM_KILLFOCUS`，再读回编辑框。读回不是请求的整数就不报成功。这会让 `ValueChanged` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，范围是 0 到 100，初始是 10。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_numeric.ps1`
- 结果：`CU-WIN-SESSION-041 OK win:powershell:5880 01M3DKDZP47ETTJQQM99H7RHFC ref=e1 path=number_set value=40 cursor=1187,239`
- 事件把标签改成 `VCU-NUM-40`。再写 40、`number:400` 和 `number:nope` 都被拒绝。超出范围会被控件夹到 100，读回不是 400，所以不算成功。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成普通文本框，也不写成小数，也不写成点箭头，也不写成完整产品 CU。



## CU-WIN-SESSION-042 会话按名字勾选列表视图的一行

状态：**2026-09-26 本机复测通过。** 不是点复选框像素，也不是 `listview_select`，也不是完整 Windows 产品 CU。

- `vcu type` 在类名含 ListView 的控件上，文本以 `lvcheck:` 开头时按行文字查找。这一行必须还没有勾选。用 `LVM_SETITEMSTATE` 把状态图设成 2，再用 `LVM_GETITEMSTATE` 读回。读回不是 2 就不报成功。这会让 `ItemCheck` 跟着跑。没有同名行不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有未勾选的 VCU-LV-A 和 VCU-LV-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_listview_check.ps1`
- 结果：`CU-WIN-SESSION-042 OK win:powershell:740 01M3DKVF36AXJPXHV05CXE11HR ref=e1 path=listview_check value=VCU-LV-B cursor=1187,239`
- 勾选后标签变成 `VCU-LV-ON-B`。只有 B 的 `ItemCheck` 会改这个标签。再勾选同一行和 `lvcheck:VCU-LV-MISSING` 都被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点复选框像素，也不写成取消勾选，也不写成 `listview_select`，也不写成完整产品 CU。



## CU-WIN-SESSION-043 会话点击自建链接

状态：**2026-09-26 本机复测通过。** 不是 `bm_click`，也不是点链接像素，也不是完整 Windows 产品 CU。

- 以前 `vcu click` 对链接标签发 `BM_CLICK` 并返回 `bm_click`。标签仍是 `VCU-LINK-OLD`，`LinkClicked` 没有发生。那种结果不能报成功。鼠标消息打在控件里也没有事件。
- 现在先找辅助功能子项。角色 30 且名字匹配时，走 `accDoDefaultAction`。这会让 `LinkClicked` 跟着跑。路径是 `link_click`。普通静态标签没有这个子项，不再用 `BM_CLICK` 假报成功。按钮仍是 `bm_click`。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有链接 VCU-LINK-B 和普通标签 VCU-PLAIN。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_link.ps1`
- 结果：`CU-WIN-SESSION-043 OK win:powershell:10124 01M3DM49R1FZNSVHDFWXKFP69G ref=e3 path=link_click value=VCU-LINK-B cursor=1187,239`
- 点击后标签变成 `VCU-LINK-HIT`。普通标签的点击被拒绝。没有 SendInput，没有移动系统光标。
- 按钮回归：`scripts/poc_win_session_click_label.ps1` 仍是 `CU-WIN-SESSION-022 OK ... path=bm_click`。
- 不做：不把这次写成点链接像素，也不写成菜单项已点中。菜单项仍不在场景树里。也不写成完整产品 CU。



## CU-WIN-SESSION-044 会话按名字点击自建菜单项

状态：**2026-09-26 本机复测通过。** 不是点菜单像素，也不是菜单项已经出现在场景树里，也不是完整 Windows 产品 CU。

- 菜单项仍然不在 UIA 场景树里，`wait --name` 找不到 VCU-MENU-B。不能把这一点写成已经能观察到菜单项。
- `vcu type` 文本以 `menu:` 开头时，在窗口和子窗口的辅助功能树里找角色 12 且名字匹配的项，再走 `accDoDefaultAction`。这会让 WinForms 菜单项的 `Click` 跟着跑。没有同名项不会报成功。路径是 `menu_click`。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，顶层项是 VCU-MENU-FILE，子项是 VCU-MENU-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_menu.ps1`
- 结果：`CU-WIN-SESSION-044 OK win:powershell:9540 01M3DMFA7ES06SKKSB3KX7W87F ref=e1 path=menu_click value=VCU-MENU-B cursor=1187,239`
- 点击后标签变成 `VCU-MENU-HIT`。`menu:VCU-MENU-MISSING` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点菜单像素，也不写成场景里已经有菜单项，也不写成完整产品 CU。



## CU-WIN-SESSION-045 会话能在场景里看到菜单项

状态：**2026-09-26 本机复测通过。** 不是 UIA 原生子项，也不是点菜单像素，也不是完整 Windows 产品 CU。

- 菜单项仍然不是 UIA 控件子项。观察菜单条时，把辅助功能角色 12 的名字插进场景，角色是 `ControlType.MenuItem`。点击和写入的编号跟这些插入项对齐，不会把后面的控件点错。
- `wait --name` 能找到菜单项。点击这个 ref 走 `menu_click`，并让 `Click` 发生。不存在的名字不会被编进场景。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，顶层项是 VCU-SCENE-FILE，子项是 VCU-SCENE-B。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_menu_scene.ps1`
- 结果：`CU-WIN-SESSION-045 OK win:powershell:19984 01M3DN44P2P8GDVRTQR2P7TG96 ref=e5 path=menu_click value=VCU-SCENE-B cursor=1187,239`
- 点击后标签变成 `VCU-SCENE-HIT`。`VCU-SCENE-MISSING` 等不到。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 UIA 原生子项，也不写成点菜单像素。同名菜单项会点中第一个。也不写成完整产品 CU。



## CU-WIN-SESSION-046 会话按名字取消勾选列表视图的一行

状态：**2026-09-26 本机复测通过。** 不是点复选框像素，也不是 `listview_check`，也不是完整 Windows 产品 CU。

- 042 只做了勾选。这次 `vcu type` 在类名含 ListView 的控件上，文本以 `lvuncheck:` 开头时按行文字查找。这一行的状态图必须已经是 2。用 `LVM_SETITEMSTATE` 把状态图设成 1，再用 `LVM_GETITEMSTATE` 读回。读回不是 1 就不报成功。这会让 `ItemCheck` 跟着跑。没有同名行、或这一行还没勾选，都不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-LV-B 初始已勾选。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_listview_uncheck.ps1`
- 结果：`CU-WIN-SESSION-046 OK win:powershell:3988 01M3DNW6EVX45Z41A2AN95MSM3 ref=e1 path=listview_uncheck value=VCU-LV-B cursor=1187,239`
- 不做：不把这次写成点复选框像素，也不写成 `listview_select`，也不写成完整产品 CU。同名行会取消勾选第一个。



## CU-WIN-SESSION-047 会话设置自建时间

状态：**2026-09-26 本机复测通过。** 不是点时间箭头，也不是 `date_set`，也不是完整 Windows 产品 CU。

- 040 只设置 `YYYY-MM-DD`。这次 `vcu type` 在类名含 SysDateTimePick32 的控件上，文本以 `time:` 开头时解析 `HH:mm:ss`。小时超过 23、分秒超过 59，或格式不对，都拒绝。当前已经是这个时刻也拒绝。先保留原日期，写入时分秒并用 `DTM_GETSYSTEMTIME` 读回。读回的时刻不一致就不报成功。再反射带时刻的 `DTN_DATETIMECHANGE`，让托管 `Value` 变成这个时刻并跑 `ValueChanged`。只改原生时间而没有事件不算成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，格式是 `HH:mm:ss`，初始是 08:00:00。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_time.ps1`
- 结果：`CU-WIN-SESSION-047 OK win:powershell:14128 01M3DP9E46XG0PM2QBRGC6PV2X ref=e1 path=time_set value=15:30:45 cursor=1187,239`
- 事件把标签改成 `VCU-TIME-15:30:45`。再写同一时刻、`time:25:00:00` 和 `time:nope` 都被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点箭头，也不写成设置日期，也不写成完整产品 CU。



## CU-WIN-SESSION-048 会话设置自建数字框的小数

状态：**2026-09-26 本机复测通过。** 不是点上下箭头，也不是整数 `number_set`，也不是完整 Windows 产品 CU。

- 041 只接受整数。这次 `vcu type` 文本以 `decimal:` 开头时，只处理旋转按钮里的编辑框。文本必须是带小数点的十进制数，用不变区域性解析。没有小数点、格式不对，或当前数值已经等于这个数，都拒绝。写入后发 `WM_KILLFOCUS`，再按不变区域性或当前区域性读回。读回数值不是请求值就不报成功，所以超出范围被夹断也不算成功。这会让 `ValueChanged` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，小数位是 2，范围 0 到 100，初始是 10.25。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_decimal.ps1`
- 结果：`CU-WIN-SESSION-048 OK win:powershell:3908 01M3DPMGM1K7M5GTWNJH65XKAX ref=e1 path=decimal_set value=12.5 cursor=1187,239`
- 事件把标签改成 `VCU-DEC-12.5`。再写 12.5、`decimal:150.5`、`decimal:nope` 和 `decimal:40` 都被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成点箭头，也不写成整数路径，也不写成完整产品 CU。



## CU-WIN-SESSION-049 会话按上下箭头改自建数字框

状态：**2026-09-26 本机复测通过。** 不是 `number_set`，也不是 `decimal_set`，也不是完整 Windows 产品 CU。

- `vcu type` 文本以 `spin:up` 或 `spin:down` 开头时，找到旋转按钮里的编辑框，再找到旁边的按钮窗口。上箭头点按钮客户区上半，下箭头点下半。消息是 `WM_LBUTTONDOWN` 和 `WM_LBUTTONUP`，不是 SendInput，也不读取或移动系统光标。点完读回编辑框数值。向上没有变大，或向下没有变小，就不报成功。到顶或到底因此被拒绝。这会让 `ValueChanged` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，范围 0 到 11，初始是 10。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_spin.ps1`
- 结果：`CU-WIN-SESSION-049 OK win:powershell:13744 01M3DPXNP7N3TTQQ9BZS8D4QB6 ref=e1 path=spin_up value=11 cursor=1187,239`
- 向上后标签变成 `VCU-SPIN-11`。到顶再向上被拒绝。向下后标签变成 `VCU-SPIN-10`。`spin:side` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成直接写数值，也不写成拖动，也不写成完整产品 CU。



## CU-WIN-SESSION-050 会话展开下拉层后点选

状态：**2026-09-26 本机复测通过。** 不是 `combo_select`，也不是完整 Windows 产品 CU。

- 029 只用 `CB_SETCURSEL`，不展开下拉层。这次 `vcu type` 文本以 `drop:` 开头时，先按名字找到条目。已经选中的和不存在的都不展开。然后 `CB_SHOWDROPDOWN` 打开列表，列表窗口必须可见。再给可见列表项发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。选中项读回必须是这一条，然后收起列表。列表没打开、点不到、或选中项没变，都不报成功。这会让 `SelectedIndexChanged` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，列表里有 A 和 B，初始选中 A。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_combo_drop.ps1`
- 结果：`CU-WIN-SESSION-050 OK win:powershell:12724 01M3DQ8NYSK54M5FN4WY449RDN ref=e3 path=combo_drop selected=VCU-COMBO-B cursor=1187,239`
- 点选后标签变成 `VCU-DROP-VCU-COMBO-B`。再点同一项和不存在的项被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 `CB_SETCURSEL`，也不写成完整产品 CU。



## CU-WIN-SESSION-051 会话点中同名菜单的后一项

状态：**2026-09-26 本机复测通过。** 不是 UIA 原生子项，也不是点菜单像素，也不是完整 Windows 产品 CU。

- 045 的同名菜单项会点中第一个。场景里同名项仍各占一个编号。点击后面的编号时，按这个名字在辅助功能树里的出现次序调用 `accDoDefaultAction`，不再总是第一个。第一次出现仍走 `menu_click`。第二次及以后走 `menu_nth`。次数对不上就不报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，两个子项都叫 VCU-SAME。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_menu_nth.ps1`
- 结果：`CU-WIN-SESSION-051 OK win:powershell:6232 01M3DQQJ27APR372W41YDCV7H3 ref=e6 path=menu_nth value=VCU-SAME cursor=1187,239`
- 点第二个编号后标签变成 `VCU-SAME-2`。再点第一个编号，标签变成 `VCU-SAME-1`，路径仍是 `menu_click`。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 UIA 原生子项，也不写成点菜单像素，也不写成完整产品 CU。



## CU-WIN-SESSION-052 被挡住的窗口截图不能采到挡板

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 018 在 `PrintWindow` 空白时复制窗口矩形，挡板可能进图。这次先采样图。图不是空白就直接用，不复制屏幕。图是空白时，窗口中心如果属于别的窗口，就返回 `OCCLUDED`，不复制屏幕矩形。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动品红窗口，再用绿色置顶窗盖住它的中心。测完只关闭这两个进程。
- 命令：`powershell -File scripts/poc_win_session_shot_occluded.ps1`
- 结果：`CU-WIN-SESSION-052 OK win:powershell:14320 01M3DR2C9K6F7XGC57G4V25NV5 mode=window-pixels cursor=1187,239`
- 挡板确实盖住了中心。截图中心仍是窗口颜色，不是绿色挡板。这一轮走的是窗口像素，不是 `OCCLUDED` 拒绝。未覆盖窗口的 018 回归仍能读到 `220,30,160`。
- 不做：不把这次写成任何遮挡都能采到窗口内容，也不写成完整产品 CU。



## CU-WIN-SESSION-053 会话点树节点的展开图标

状态：**2026-09-26 本机复测通过。** 不是 `tree_expand`，也不是完整 Windows 产品 CU。

- 036 用 `TVM_EXPAND`，不点图标。这次 `vcu type` 文本以 `treeicon:` 开头时按名字找节点。节点已经展开就拒绝。用 `TVM_GETITEMRECT` 读文字矩形，在文字左侧发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。读回必须带 `TVIS_EXPANDED`。点完没展开就不报成功，也不改走 `TVM_EXPAND`。这会让 `AfterExpand` 跟着跑。没有同名节点不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，根节点是 VCU-EXP-A，子节点是 VCU-EXP-B，初始未展开。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tree_icon.ps1`
- 结果：`CU-WIN-SESSION-053 OK win:powershell:8264 01M3DRCRNV45Y35PFJCB7H2H69 ref=e3 path=tree_icon value=VCU-EXP-A cursor=1187,239`
- 点图标后标签变成 `VCU-EXP-OPEN`。再点同一节点和不存在的节点被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 `TVM_EXPAND`，也不写成折叠图标，也不写成完整产品 CU。



## CU-WIN-SESSION-054 会话点树节点的折叠图标

状态：**2026-09-26 本机复测通过。** 不是 `tree_collapse`，也不是完整 Windows 产品 CU。

- 037 用 `TVE_COLLAPSE` 再反射通知，不点图标。这次 `vcu type` 文本以 `foldicon:` 开头时按名字找节点。节点必须已经展开。用 `TVM_GETITEMRECT` 读文字矩形，在文字左侧发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。读回不能再带 `TVIS_EXPANDED`。点完仍展开就不报成功，也不改走 `TVM_EXPAND` 或反射 `TVN_ITEMEXPANDEDW`。这会让 `AfterCollapse` 跟着跑。没有同名节点不会报成功。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，根节点 VCU-COL-A 已展开。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tree_fold.ps1`
- 结果：`CU-WIN-SESSION-054 OK win:powershell:15624 01M3DRKNBHRE3KJVWXS2P86SFZ ref=e3 path=fold_icon value=VCU-COL-A cursor=1187,239`
- 点图标后标签变成 `VCU-COL-CLOSED`。再点同一节点和不存在的节点被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 `tree_collapse`，也不写成完整产品 CU。



## CU-WIN-SESSION-055 会话点可见菜单行

状态：**2026-09-26 本机复测通过。** 不是 `accDoDefaultAction`，也不是完整 Windows 产品 CU。

- 044 走辅助功能默认动作，不点菜单像素。这次 `vcu type` 文本以 `menupix:` 开头时，先点菜单条打开下拉层，再在弹出窗口里按名字找行。弹出层里没有这个名字就拒绝，不改走 `accDoDefaultAction`。点的是弹出窗口对应行的中心，消息是 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。点中点不属于这个弹出窗口就不报成功。这会让菜单项的 `Click` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，顶层项是 VCU-MENU-FILE，下面有 VCU-MENU-A 和 VCU-MENU-B。只有 B 的点击会改标签。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_menu_pixel.ps1`
- 结果：`CU-WIN-SESSION-055 OK win:powershell:8852 01M3DS2P3SXJN240KPXGCZD3EW ref=e1 path=menu_pixel value=VCU-MENU-B cursor=1187,239`
- 点中后标签变成 `VCU-MENU-HIT`。不存在的项被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 UIA 原生子项，也不写成完整产品 CU。菜单条上点不到的项仍会拒绝。



## CU-WIN-SESSION-056 会话拖动自建滑块

状态：**2026-09-26 本机复测通过。** 不是 `TBM_SETPOS`，也不是 RangeValuePattern，也不是完整 Windows 产品 CU。

- 031 用 `TBM_SETPOS` 写位置。这次 `vcu type` 文本以 `trackdrag:` 开头时解析整数。已经是这个位置、超出范围或不是整数都拒绝。读滑道和拖块矩形，在拖块上按下，再移到目标位置松开。消息是 `WM_LBUTTONDOWN`、`WM_MOUSEMOVE`、`WM_LBUTTONUP`。读回 `TBM_GETPOS` 必须等于请求值。对不上就不报成功，也不改走 `TBM_SETPOS`。这会让 `ValueChanged` 跟着跑。跨进程鼠标消息在这台 144 DPI 机器上要按 `AppliedDPI/96` 放大，否则会点到拖块左边。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，范围 0 到 100，初始是 10。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_track_drag.ps1`
- 结果：`CU-WIN-SESSION-056 OK win:powershell:11908 01M3DSK7X4CHHQ9FJSHXDWYXFT ref=e2 path=track_drag value=40 cursor=1187,239`
- 拖动后标签变成 `VCU-TRACK-40`。再拖同一位置、`trackdrag:400` 和 `trackdrag:nope` 被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 `TBM_SETPOS`，也不写成完整产品 CU。



## CU-WIN-SESSION-057 会话点标签页标题

状态：**2026-09-26 本机复测通过。** 不是 `tab_select`，也不是完整 Windows 产品 CU。

- 032 用辅助功能选中标签，再反射选择变化。这次 `vcu type` 文本以 `tabpix:` 开头时按名字找标签。已经选中的和不存在的都拒绝。用 `TCM_GETITEMRECT` 读标题矩形，按这台机器的 `AppliedDPI/96` 放大后发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。读回 `TCM_GETCURSEL` 必须是这一页。对不上就不报成功，也不改走 `accSelect` 或 `TCM_SETCURSEL`。这会让 `SelectedIndexChanged` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有 VCU-TAB-A 和 VCU-TAB-B，初始选中 A。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tab_pixel.ps1`
- 结果：`CU-WIN-SESSION-057 OK win:powershell:2928 01M3DSYQQW072MFSSF9Z292ER7 ref=e3 path=tab_pixel value=VCU-TAB-B cursor=1187,239`
- 点中后标签变成 `VCU-TAB-SHOW-B`。再点同一页和不存在的页被拒绝。没有 SendInput，没有移动系统光标。
- 不做：不把这次写成 `tab_select`，也不写成完整产品 CU。



## CU-WIN-SESSION-058 会话点列表框的一行

状态：**2026-09-26 本机复测通过。** 不是 `list_select`，也不是 `LB_SETCURSEL`，也不是完整 Windows 产品 CU。

- 030 用 `LB_SETCURSEL`，不点行。这次 `vcu type` 文本以 `listpix:` 开头时按行文字找列表框。已经选中的行和不存在的行都拒绝。`LB_GETITEMRECT` 跨进程返回 `LB_ERR`，不拿它报成功。用 `LB_GETITEMHEIGHT` 和客户区宽度算行中心。行不在可见区域时先 `LB_SETTOPINDEX`，再发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。这台机器的 user32 列表框坐标是逻辑坐标，`GetDpiForWindow` 是 96。按 `AppliedDPI/96` 放大要点空，所以不放大。读回 `LB_GETCURSEL` 必须是这一行。对不上就不报成功，也不改走 `LB_SETCURSEL`。这会让 `SelectedIndexChanged` 跟着跑。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有 VCU-LIST-A 和 VCU-LIST-B，初始选中 A。另开一个 40 行窗口，点滚出视口的 VCU-LIST-FAR。测完只关闭这些进程。
- 命令：`powershell -File scripts/poc_win_session_list_pixel.ps1`
- 结果：`CU-WIN-SESSION-058 OK win:powershell:13496 01M3DV958FK8PYVH0V4SQK75SJ ref=e3 path=list_pixel selected=VCU-LIST-B far=VCU-LIST-FAR cursor=1187,239`
- 点中后标签变成 `VCU-LIST-HIT`。滚出视口的行标签变成 `VCU-LIST-FAR-HIT`。再点同一行和不存在的行被拒绝。`scripts/poc_win_session_list.ps1` 仍是 `list_select`。没有 SendInput，没有移动系统光标。Edge 扩展仍 `pong`。
- 不做：不把这次写成 `list_select`，也不写成完整产品 CU。




## CU-WIN-SESSION-059 会话点树节点文字

状态：**2026-09-26 本机复测通过。** 不是 `tree_select`，也不是 `TVM_SELECTITEM`，也不是完整 Windows 产品 CU。

- 033 用 `TVM_SELECTITEM`，不点文字。这次 `vcu type` 文本以 `treepix:` 开头时按名字找节点。已经选中的和不存在的都拒绝。用 `TVM_GETITEMRECT` 读文字矩形，点文字中心。跨进程 `SendMessage` 发 `WM_LBUTTONDOWN` 会卡在树控件内部等松开，所以改用 `PostMessage` 按下再松开。这台机器的文字矩形已经是逻辑坐标，不按 `AppliedDPI/96` 放大。读回 `TVGN_CARET` 必须是这个节点。对不上就不报成功，也不改走 `TVM_SELECTITEM`。这会让 `AfterSelect` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，根节点是 VCU-TREE-A，子节点是 VCU-TREE-B，初始选中 A。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_tree_pixel.ps1`
- 结果：`CU-WIN-SESSION-059 OK win:powershell:9516 01M3DVR9NAJFT39CANQEHBDBY1 ref=e3 path=tree_pixel value=VCU-TREE-B cursor=1187,239`
- 点中后标签变成 `VCU-TREE-SHOW-B`。再点同一节点和不存在的节点被拒绝。`scripts/poc_win_session_tree.ps1` 仍是 `tree_select`。没有移动系统光标。Edge 扩展仍 `pong`。
- 不做：不把这次写成 `tree_select`，也不写成点展开图标，也不写成完整产品 CU。




## CU-WIN-SESSION-060 空白且被挡住时拒绝复制屏幕

状态：**2026-09-26 本机复测通过。** 不是完整 Windows 产品 CU。

- 052 的品红窗口能被 `PrintWindow` 采到像素，盖住以后仍走窗口像素，不是拒绝。这次自建一个 `WS_EX_NOREDIRECTIONBITMAP` 窗口，`PrintWindow` 中心是 `0,0,0`。再用绿色置顶窗盖住中心。会话截图必须拒绝，文案包含 covered，不复制屏幕矩形，也不把挡板采进图。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动空白窗口和挡板。测完只关闭这两个进程。
- 命令：`powershell -File scripts/poc_win_session_shot_blank_occluded.ps1`
- 结果：`CU-WIN-SESSION-060 OK win:powershell:4584 01M3DVZDTZQCZC4QARZSJCA7PF mode=OCCLUDED cursor=1187,239`
- 同轮回归 `scripts/poc_win_session_shot_occluded.ps1` 仍是 `mode=window-pixels`。没有移动系统光标。
- 不做：不把这次写成任何遮挡都会拒绝，也不写成完整产品 CU。有像素的窗口被挡住时仍可以读到窗口自己的颜色。




## CU-WIN-SESSION-061 会话点勾选列表的复选框

状态：**2026-09-26 本机复测通过。** 不是 `check_set`，也不是完整 Windows 产品 CU。

- 038 用注册消息 `LBC_SETCHECKSTATE`，不点复选框。这次 `vcu type` 文本以 `checkpix:` 开头时按行文字查找。已经勾选的行和不存在的行都拒绝。用 `LB_GETITEMHEIGHT` 算行中心，点左侧复选框，大约 x=8。新鲜行的第一次点击只选中，不勾选，所以没勾上就再点一次，第三次不再点，避免把刚勾上的取消。消息是 `PostMessage` 的 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。读回 `LBC_GETCHECKSTATE` 必须是已勾选。对不上就不报成功，也不改走 `LBC_SETCHECKSTATE`。这会让 `ItemCheck` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，里面有 VCU-CHK-A 和 VCU-CHK-B，初始都没勾选。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_checked_list_pixel.ps1`
- 结果：`CU-WIN-SESSION-061 OK win:powershell:16568 01M3DWNXHDN9B58H2DSEX6X9KE ref=e1 path=check_pixel value=VCU-CHK-B cursor=1187,239`
- 点中后标签变成 `VCU-CHK-ON-B`。再点同一行和不存在的行被拒绝。`scripts/poc_win_session_checked_list.ps1` 仍是 `check_set`。没有移动系统光标。Edge 扩展仍 `pong`。
- 不做：不把这次写成 `check_set`，也不写成完整产品 CU。




## CU-WIN-SESSION-062 会话点复选框取消勾选

状态：**2026-09-26 本机复测通过。** 不是 `uncheck_set`，也不是完整 Windows 产品 CU。

- 039 用注册消息把勾选写成 0，不点复选框。这次 `vcu type` 文本以 `uncheckpix:` 开头时按行文字查找。没勾选的行和不存在的行都拒绝。点左侧复选框，大约 x=8。已勾选的新鲜行第一次点击只选中，不取消，所以没取消就再点一次，第三次不再点。读回 `LBC_GETCHECKSTATE` 必须是未勾选。对不上就不报成功，也不改走注册消息。这会让 `ItemCheck` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-CHK-B 初始已勾选。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_unchecked_list_pixel.ps1`
- 结果：`CU-WIN-SESSION-062 OK win:powershell:11472 01M3DWWRQJ573ERY7V1MGHCWV9 ref=e1 path=uncheck_pixel value=VCU-CHK-B cursor=1187,239`
- 点中后标签变成 `VCU-CHK-OFF-B`。再点同一行和不存在的行被拒绝。`scripts/poc_win_session_unchecked_list.ps1` 仍是 `uncheck_set`。没有移动系统光标。Edge 扩展仍 `pong`。
- 不做：不把这次写成 `uncheck_set`，也不写成完整产品 CU。




## CU-WIN-SESSION-063 会话点单选按钮

状态：**2026-09-26 本机复测通过。** 不是 `bm_click`，也不是完整 Windows 产品 CU。

- 028 的 `vcu click` 走 `BM_CLICK`。这次 `vcu type` 文本以 `radiopix:` 开头时按窗口文字找按钮。已经选中的和不存在的都拒绝。`BM_GETCHECK` 读不到 WinForms 单选状态，所以用辅助功能状态里的勾选位。点客户区中心，消息是 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`，不是 `BM_CLICK`。读回必须带勾选位。对不上就不报成功，也不改走 `BM_CLICK`。这会让 `CheckedChanged` 跟着跑，另一颗同组按钮被清掉。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-RADIO-A 初始选中，VCU-RADIO-B 未选中。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_radio_pixel.ps1`
- 结果：`CU-WIN-SESSION-063 OK win:powershell:680 01M3DX5Y9476SV1ZZTAGRKW9DX ref=e4 path=radio_pixel value=VCU-RADIO-B cursor=1187,239`
- 点中后 B 是 `toggle-on`，A 是 `toggle-off`，标签变成 `VCU-RADIO-SHOW-B`。再点同一颗和不存在的按钮被拒绝。`scripts/poc_win_session_radio.ps1` 仍是 `bm_click`。没有移动系统光标。Edge 扩展仍 `pong`。
- 不做：不把这次写成 `bm_click`，也不写成完整产品 CU。




## CU-WIN-SESSION-064 会话点普通按钮

状态：**2026-09-26 本机复测通过。** 不是 `bm_click`，也不是完整 Windows 产品 CU。

- 022 的 `vcu click` 走 `BM_CLICK`。这次 `vcu type` 文本以 `btnpix:` 开头，格式是按钮名和点击后应出现的标签，中间用 `|` 分开。找不到按钮就拒绝。标签已经是目标文本就拒绝，避免把没点的结果写成成功。点客户区中心，消息是 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`，不是 `BM_CLICK`。点完必须能读到那个标签。读不到就不报成功，也不改走 `BM_CLICK`。这会让按钮的 `Click` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，按钮是 VCU-CLICK-022，标签初始是 ready。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_button_pixel.ps1`
- 结果：`CU-WIN-SESSION-064 OK win:powershell:15068 01M3DXQ9YCQ3EAW7WA7NGCYABN ref=e2 path=button_pixel value=VCU-CLICK-022 cursor=1187,239`
- 点中后标签变成 `VCU-CLICKED-022`。再点同一条和不存在的按钮被拒绝。`scripts/poc_win_session_click_label.ps1` 仍是 `bm_click`。没有移动系统光标。
- 不做：不把这次写成 `bm_click`，也不写成完整产品 CU。列表视图复选框的像素点击见 CU-WIN-SESSION-065，不是这次的 button_pixel。




## CU-WIN-SESSION-065 会话点列表视图复选框

状态：**2026-09-26 本机复测通过。** 不是 `listview_check`，也不是完整 Windows 产品 CU。

- 042 用 `LVM_SETITEMSTATE` 把状态图写成已勾选，不点复选框。这次 `vcu type` 文本以 `lvcheckpix:` 开头时按行文字查找。已勾选的行和不存在的行都拒绝。点行矩形左侧，大约 x=left+6。同一个进程先加载 UIAutomation 后再发鼠标消息，勾选状态不会变，所以由没碰 UIA 的 powershell 发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。读回 `LVM_GETITEMSTATE` 的状态图必须是已勾选。对不上就不报成功，也不改走 `LVM_SETITEMSTATE`。这会让 `ItemCheck` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-LV-B 初始未勾选。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_listview_check_pixel.ps1`
- 结果：`CU-WIN-SESSION-065 OK win:powershell:16076 01M3DYCKXST33VPFEGAGVEV5A8 ref=e1 path=lvcheck_pixel value=VCU-LV-B cursor=1079,665`
- 点中后标签变成 `VCU-LV-ON-B`。再点同一行和不存在的行被拒绝。`scripts/poc_win_session_listview_check.ps1` 仍是 `listview_check`。没有移动系统光标。Edge 扩展仍 `pong`。单测 `windows_backend_platform_and_denials` 通过。
- 不做：不把这次写成 `listview_check`，也不写成完整产品 CU。


## CU-WIN-SESSION-066 会话取消列表视图勾选

状态：**2026-09-26 本机复测通过。** 不是 `listview_uncheck`，也不是完整 Windows 产品 CU。

- 046 用 `LVM_SETITEMSTATE` 把状态图写成未勾选，不点复选框。这次 `vcu type` 文本以 `lvuncheckpix:` 开头时按行文字查找。未勾选的行和不存在的行都拒绝。点行矩形左侧，大约 x=left+6。同一个进程先加载 UIAutomation 后再发鼠标消息，勾选状态不会变，所以由没碰 UIA 的 powershell 发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。已勾选的新鲜行第一次点击可能只选中，所以没取消就再点一次，第三次不再点。读回 `LVM_GETITEMSTATE` 的状态图必须是未勾选。对不上就不报成功，也不改走 `LVM_SETITEMSTATE`。这会让 `ItemCheck` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-LV-B 初始已勾选。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_listview_uncheck_pixel.ps1`
- 结果：`CU-WIN-SESSION-066 OK win:powershell:13028 01M3DYXNEVA0M5SK7WSMTMWWRF ref=e1 path=lvuncheck_pixel value=VCU-LV-B cursor=789,599`
- 点中后标签变成 `VCU-LV-OFF-B`。再点同一行和不存在的行被拒绝。`scripts/poc_win_session_listview_uncheck.ps1` 仍是 `listview_uncheck`。没有移动系统光标。Edge 扩展仍 `pong`。单测 `windows_backend_platform_and_denials` 通过。
- 不做：不把这次写成 `listview_uncheck`，也不写成完整产品 CU。


## CU-WIN-SESSION-067 会话点列表视图一行

状态：**2026-09-26 本机复测通过。** 不是 `listview_select`，也不是完整 Windows 产品 CU。

- 034 用 `LVM_SETITEMSTATE` 选中一行，不点文字。这次 `vcu type` 文本以 `lvrowpix:` 开头时按行文字查找。已经选中的行和不存在的行都拒绝。点行矩形中心，不是左侧复选框。同一个进程先加载 UIAutomation 后再发鼠标消息，选中状态不会变，所以由没碰 UIA 的 powershell 发 `WM_LBUTTONDOWN` / `WM_LBUTTONUP`。第一次没选中就再点一次，第三次不再点。读回 `LVM_GETITEMSTATE` 的选中位必须置上。对不上就不报成功，也不改走 `LVM_SETITEMSTATE`。这会让 `ItemSelectionChanged` 跟着跑。没有 SendInput，没有移动系统光标。
- 开始前没有借用用户已开的记事本或计算器。脚本自己启动窗口，VCU-LV-A 初始选中。测完只关闭这个进程。
- 命令：`powershell -File scripts/poc_win_session_listview_row_pixel.ps1`
- 结果：`CU-WIN-SESSION-067 OK win:powershell:18808 01M3DZ9F4KW8AR5CRKWBX1XZ99 ref=e3 path=lvrow_pixel value=VCU-LV-B cursor=789,599`
- 点中后标签变成 `VCU-LV-PICKED-B`。再点同一行和不存在的行被拒绝。`scripts/poc_win_session_listview.ps1` 仍是 `listview_select`。没有移动系统光标。Edge 扩展仍 `pong`。单测 `windows_backend_platform_and_denials` 通过。
- 不做：不把这次写成 `listview_select`，也不写成完整产品 CU。


## 还没做

- 不把已复测的会话切片写成完整 `vcu session` 产品 CU。
- 树展开是 `tree_expand`，树折叠是 `tree_collapse`。折叠不是点图标。只清展开位而没有 `AfterCollapse` 不算成功。树节点文字点击是 `tree_pixel`，不是 `tree_select`。展开图标仍是 `tree_icon`。
- 数字上下控件不在本切片。跨进程改文字如果没有对应变更事件，不能报成功。
- 科学模式验证了切换、π、log10(100)、ln(e)、sin(30°)、cos(0)、tan(45°)，以及 sin(π) 在角度和弧度下的不同结果。没有覆盖完整科学函数矩阵。
- 052 在 PrintWindow 仍有像素时走窗口像素。060 已实测空白且中心被挡住时拒绝复制屏幕。
- 没有在 100% DPI 和其他机器上复测。
- 不放行整个 `ApplicationFrameHost`。
- 跨源 iframe、trusted 手势、`TC-B-040` 仍未做。
