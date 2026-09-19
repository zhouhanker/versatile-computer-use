# Desktop surface（最短环）

登录态网页仍走 Browser Bridge。这段只用于 **macOS 真窗口**（TextEdit / Notes / Finder），不搬物理鼠标。

```sh
vcu daemon start
vcu session start --surface desktop --app-id proc:TextEdit:<pid> --json
# 必须 stage_hud=true；否则 ErrorCode=StageRequired

vcu snapshot --session <sid> --json
# kind=desktop.scene  surface=desktop  source=ax_scene
# 不是 extension_dom

vcu type --session <sid> --ref e2 --text 'hello'
vcu session abort --id <sid>
```

USER Edge 的 HTML 控件：`vcu browser click --selector`，`source=extension_dom`。不要对 AXWebArea 假装 DOM 点击。

Abort：`vcu session abort --id <sid>`（与 Stage 上 Escape 同一路径）。
