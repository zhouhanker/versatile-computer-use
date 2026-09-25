# Windows 产品会话史诗

更新：2026-09-26。作者：本机真机记录。状态：第一扇门已复测。不是完整 Windows 产品 CU，也没有新的 GitHub Release。

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

## 还没做

- 不把这次会话写成完整 `vcu session` 产品 CU。
- 只验证了按一次「七」。没有做连续运算，也没有把这写成完整计算器产品。
- 没有在 100% DPI 和其他机器上复测。
- 不放行整个 `ApplicationFrameHost`。
