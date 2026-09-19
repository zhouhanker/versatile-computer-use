# VCU 会话交接

更新：2026-09-20。桌面史诗进行中。

## 本轮

- CU-D-023 **真机通过**：TextEdit 文档出现 `VCU-D-023-*` 标记；`source=ax_scene`；`os_cursor_used=false`；`input_path=ax_set_value`；ref=`e8` AXTextArea。
- 根因：session `type` 无 `--tab`；`resolve_tab` 曾优先第一个 agent 窗口（常为 Edge）；POC 把 `AXScrollArea`（e1）当成文本区。
- 修复：CLI `--tab`；resolve 优先 `active_tab_id`；AXSetValue 拒绝非文本并下钻 text area；TextEdit 可回退 `text area 1`；Notes Scene 不进入 SplitGroup。
- Notes：真机 `AXPress` `e8` `axpress:0` + Guide overlay。POC 曾误点「添加文件夹」（脚本 `name` 字段覆盖导致 report.ok 假红）。后续 POC 改选「开关边栏」等，不再点添加/新建。
- `vcu session abort <sid>` 是位置参数，不是 `--id`。

## 下一刀

CU-D-032 浏览器回归：ping / tabs / open 现窗新标签 / screenshot click。不要碰 Edge 组 1/3。
