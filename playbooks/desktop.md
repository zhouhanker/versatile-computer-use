# Desktop surface（最短环）

登录态网页仍走 Browser Bridge。这段只用于 **macOS 真窗口**（TextEdit / Notes / Finder），不搬物理鼠标。

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
