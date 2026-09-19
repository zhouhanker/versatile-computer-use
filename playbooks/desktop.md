# Desktop surface（最短环）

登录态网页仍走 Browser Bridge。macOS 真窗口（TextEdit / Notes / Finder）与 Windows CI 切片（Notepad / Explorer / cmd）都不搬物理鼠标。

```sh
vcu daemon start
vcu session start --surface desktop --app-id proc:TextEdit:<pid> --json
# 必须 stage_hud=true；否则 ErrorCode=StageRequired

vcu snapshot --session <sid> --tab proc:TextEdit:<pid> --json
# kind=desktop.scene  surface=desktop  source=ax_scene
# 不是 extension_dom
# 选 AXTextArea / AXTextField，不要用 AXScrollArea（e1 常常是滚动区）

vcu type --session <sid> --tab proc:TextEdit:<pid> --ref <text-ref> --text 'hello' --json
# data.ok=true  data.detail.os_cursor_used=false  data.detail.input_path=ax_set_value

vcu session abort <sid>
```

USER Edge 的 HTML 控件：`vcu browser click --selector`，`source=extension_dom`。不要对 AXWebArea 假装 DOM 点击。

Abort：`vcu session abort <sid>`（与 Stage 上 Escape 同一路径）。不要传 `--id`。

真机脚本：`scripts/poc_desktop_textedit.sh`（CU-D-023，文档必须出现标记才算过）。

Finder（CU-D-040）：AX 不暴露文件夹图标。列窗用 Scene `CGWindow`；选中/打开用 Launch Services：

```sh
# action.json: {"type":"open_path","target":{"tab_id":"proc:Finder:<pid>"},"args":{"path":"/tmp/VCU-D-040/OPENME","tab_id":"proc:Finder:<pid>"}}
vcu act --session <sid> --action-json action.json --json
# detail.input_path=nsworkspace_open  os_cursor_used=false
```

不要对 Finder 图标宣称 AXPress；不要用 Return 当打开（Return 仍是 Send 门禁）。

Terminal（CU-D-041）：只操作脚本新开的 **Terminal.app**，不要打用户 Ghostty。

```sh
vcu type --session <sid> --tab proc:Terminal:<pid> --ref w1 --text 'echo-not-run' --json
# input_path=ax_menu_paste  —— 无换行，不会执行
```

换行或 `act type=key return` 无 `confirm_send` → FocusPolicyViolation。

飞书客户端（CU-D-042）：只观察已打开的窗。消息在 Electron webview，AX 列不出气泡。禁止点发送 / 盲目 Return。

系统设置（CU-D-043）：只观察。缺辅助功能时看 `vcu doctor` 的 hint（系统设置 → 隐私与安全），不要让 agent 去勾选。


## Windows（CU-D-090…140，CI 真机，不是产品会话）

Server 2022 上的诚实路径：Notepad 写入是 `wm_settext`（Edit 常是 `ControlType.Pane`，**不是** ValuePattern）；按钮是 `bm_click`（**不是** InvokePattern）；Explorer 是 `explorer_open` / `explorer_reveal`；cmd 是 `clipboard_paste`。禁止 SendInput / SendKeys。

cmd 的可见窗在 conhost 上，`Get-Process.MainWindowHandle` 经常是 0。Scene 往往没有 Edit，用窗口 ref。

```sh
# 抛弃型 cmd：scripts/poc_cu_d_140.ps1
vcu session start --surface desktop --app-id win:cmd:<pid> --json
vcu snapshot --session <sid> --tab win:cmd:<pid> --json
# source=uia_scene；选窗口 ref（常见 e1），不要假报 Edit
vcu type --session <sid> --tab win:cmd:<pid> --ref e1 --text echo-not-run --json
# input_path=clipboard_paste  os_cursor_used=false
```

换行或 Return 无 `confirm_send` → FocusPolicyViolation。不要把这段写成已执行命令，也不要写成完整 Windows 产品 CU。
