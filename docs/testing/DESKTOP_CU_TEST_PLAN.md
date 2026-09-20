# 桌面 Computer Use 测试计划

对应 [`../ROADMAP-CU.md`](../ROADMAP-CU.md)。ID 前缀 **TC-D**。浏览器用例仍以 `BROWSER_TEST_PLAN.md` / `BROWSER_TEST_CASES.md` 为准，桌面切片不得让它们变红。

## 1. 原则

- 策略类（微信、光标、Allow、盲目 Return、无 HUD 拒绝会话）**必须单测**，不依赖真窗口。
- 真机只用脚本创建的 TextEdit / Notes / 临时文件夹窗口；用完关掉。
- **禁止**操作使用者已有的 Edge 标签组（含标题 `1` / `3`）。
- **禁止**真机微信、代点 Allow、warp 系统光标。
- 失败要诚实：`os_cursor_used=false`；AX 非零不得包装成 click 成功。
- 网页按钮继续走 extension；桌面像素点到 AXWebArea 只允许报 WebArea。

## 2. 门禁命令

每个桌面 PR：

```sh
cargo test --workspace --offline
node --test extension/tests/*.test.cjs
```

每个 CU-D 阶段结束另加：

```sh
make check                          # 浏览器冻结
cargo test -p vcu-server app::
```

阶段 2 起受控真机（需辅助功能）：

```sh
bash scripts/poc_desktop_textedit.sh   # CU-D-023 TextEdit 真写入
bash scripts/poc_desktop_notes.sh      # CU-D-023 Notes AXPress（勿点添加/新建）
```

现有相关：

```sh
bash scripts/poc_app_macos.sh       # 列窗 POC，不是 Actuator 完成证据
bash scripts/poc_login_state.sh     # 登录态策略；含 os_cursor / Allow 文案
```

## 3. 策略用例（阶段 1 就必须绿）

| ID | 标题 | 期望 | 自动化 |
| --- | --- | --- | --- |
| TC-D-001 | 微信窗口拒绝 Scene/Actuator | AppDenied；无动作 | 已有 `wechat_is_hard_denied`，保持 |
| TC-D-002 | os_cursor / HID 主路径拒绝 | OsCursorDenied | 已有 mock / login-state |
| TC-D-003 | next_action 永不出现 click Allow | 文案无 Allow debugging | 已有 |
| TC-D-004 | Return 无 confirm_send 拒绝 | FocusPolicyViolation | 已有 key 门禁 |
| TC-D-005 | desktop 会话无 Stage 则失败 | 不得 act | `desktop_rejects_hidden_stage` + HTTP `stage_hud` |
| TC-D-006 | Abort 后无残留 HUD/Guide | 会话消失、hud=false | `session_abort_removes_desktop_session`；Escape 与 abort 文件同路径 |
| TC-D-007 | 缺辅助功能 repair 指向系统设置 | 不提 Edge Allow | doctor 单测扩展 |

## 4. 观察用例（阶段 1）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-010 | allowlist 列窗 | Edge/Finder/TextEdit 可出现；微信不出现 |
| TC-D-011 | 截帧用窗口 ID | 失败不 ok:true；不拿被挡屏幕区域冒充 |
| TC-D-012 | screenshot_scale | 仅 1/2/3 |
| TC-D-013 | Scene 含 AX 摘要 | 无权限软降级并标明，不假绿 |

## 5. 动作用例（阶段 2）

| ID | 标题 | 前置 | 期望 |
| --- | --- | --- | --- |
| TC-D-020 | AXPress 非零不得成功 | 单测 | `ax_ref_press_succeeded` 拒绝 ok-click / 非零码 |
| TC-D-021 | TextEdit live 写入一行 | 用户已授辅助功能 | **通过** 2026-09-20：标记在文档内；os_cursor_used=false；脚本 `scripts/poc_desktop_textedit.sh` |
| TC-D-022 | AXPress 非零 | mock | 失败，不 pressed=true |
| TC-D-023 | 像素点 WebArea | 浏览器窗 | hit 为 WebArea 或明确失败，不报 extension_dom |
| TC-D-024 | Guide overlay | live | overlay true；不移动物理鼠标 |

## 6. 统一循环（阶段 3）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-030 | 同一会话 Observation 带 surface | desktop vs browser 可区分 |
| TC-D-031 | HTTP 页 click selector | source=extension_dom |
| TC-D-032 | 原生按钮 click ref | source 为 AX 路径，非 extension_dom |
| TC-D-033 | 浏览器 open 仍是现有窗口新标签 | 0.2.8 行为 |

## 7. 真机操作卡

1. 确认辅助功能已授权给 `vcu` / `vcu-stage` / 终端。  
2. 不要把个人文档、邮件、聊天当靶。  
3. 脚本创建 `VCU-D-TEST-*` 标题窗口。  
4. 先 `--dry-run` JSON，再 live。  
5. `trap` 里关闭自己创建的窗口；Abort 测试后检查无 HUD。  
6. 证据进 `.local/desktop-cu/`（不提交仓库）。  

## 7.1 Finder（CU-D-040）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-040 | 脚本自建文件夹窗出现在 Scene | **通过** CGWindow 标题；`scripts/poc_desktop_finder.py` |
| TC-D-041 | reveal + open_path 打开子文件夹 | **通过** `nsworkspace_reveal` / `nsworkspace_open`；非 AXPress、非 Return |

## 7.2 Terminal（CU-D-041）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-042 | 抛弃型 Terminal 窗可列 CGWindow | **通过** `scripts/poc_desktop_terminal.py` |
| TC-D-043 | type 无换行出现在终端 | **通过** `ax_menu_paste`；未执行命令 |
| TC-D-044 | 换行 type 与盲目 Return 拒绝 | **通过** FocusPolicyViolation |

## 7.3 飞书客户端（CU-D-042）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-045 | 已运行的 Feishu 窗出现在 Scene | **通过** CGWindow「飞书」；`scripts/poc_desktop_feishu.py` |
| TC-D-046 | 不自动发送 | **通过** 无 发送点击；Return 门禁；mock `e_send` 拒绝 |

## 7.4 系统设置（CU-D-043）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-047 | 只读观察 System Settings | **通过** `scripts/poc_desktop_settings.py` |
| TC-D-048 | 不自动改 TCC | **通过** click 拒绝；doctor 无 tccutil |

## 7.5 Stage HUD（CU-D-050）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-050 | HUD 文案为 VCU | **通过** `hud_copy_is_vcu_not_chatgpt_or_codex` |
| TC-D-051 | Abort 拆 HUD | **通过** 真机 abort `hud=false` |

## 7.6 Windows UIA（CU-D-060 / CU-D-070）

| ID | 标题 | 期望 |
| --- | --- | --- |
| TC-D-060 | CI Notepad UIA 列窗 | **通过** run `35462329205` `UIA_OK`；`scripts/poc_desktop_windows.ps1` |
| TC-D-061 | CI PrintWindow PNG | **通过** `PRINTWINDOW_OK` 魔数；无 SendInput |
| TC-D-070 | CI Notepad 写入并读回 | **通过** run `35463098806` `SETVALUE_OK path=wm_settext`；无 SendInput；非 ValuePattern |
| TC-D-080 | backend set_value 接受 wm_settext | **通过（单测）** `input_path=wm_settext` `os_cursor_used=false` |
| TC-D-090 | vcu desktop Notepad type | **通过** run `35465535542` `STAGE_OK` + `SNAP_OK source=uia_scene` + `TYPE_OK path=wm_settext` |
| TC-D-100 | vcu click 抛弃型按钮 | **通过** run `35466230837` `INVOKE_OK path=bm_click`；无 SendInput；非 InvokePattern |
| TC-D-110 | vcu screenshot PrintWindow | **通过** run `35466641180` `SHOT_OK` 768x519 PNG |
| TC-D-120 | vcu open_path Explorer | **通过** run `35467498771` `OPEN_OK path=explorer_open` |
| TC-D-130 | vcu reveal Explorer | **通过** run `35467970349` `REVEAL_OK path=explorer_reveal` |
| TC-D-140 | cmd type without Return | **通过** run `35469208163` `TYPE_OK path=clipboard_paste`；`NEWLINE_DENIED`；无 SendInput |
| TC-D-150 | Windows playbook names clipboard_paste | **通过** `playbooks/desktop.md` 含 `clipboard_paste` / `win:cmd:`；无 SendInput |
| TC-D-160 | Calculator click without HID | **通过** run `35470610934` `INVOKE_OK path=bm_click`；win32calc e14；无 SendInput |
| TC-D-170 | Windows Settings observe-only | **通过** run `35471043411` `CLICK_DENIED` / `TYPE_DENIED`；SystemSettings |
| TC-D-180 | Notepad scroll without HID | **通过** run `35471497693` `SCROLL_OK path=wm_vscroll`；无 SendInput |
| TC-D-190 | extract typed Notepad value | **通过** run `35471859281` `EXTRACT_OK count=1`；无 SendInput |
| TC-D-200 | wait until Notepad value | **通过** run `35472199783` `WAIT_OK path=scene_wait found_ref=e2`；无 SendInput |
| TC-D-210 | wait miss times out honestly | **通过** run `35472851832` `WAIT_MISS_OK` / `WAIT_REF_MISS_OK`；`ActionFailed`；无 SendInput |
| TC-D-220 | Notepad key return denied | **通过** run `35473210457` `KEY_DENIED` FocusPolicyViolation；无 SendInput |
| TC-D-230 | PowerShell type without Return | **通过** run `35473547564` `TYPE_OK path=clipboard_paste`；`NEWLINE_DENIED`；无 SendInput |
| TC-D-240 | Windows abort tears down HUD | **通过** run `35473997888` `ABORT_OK hud=false` / `HUD_GONE`；`SessionNotFound`；无 SendInput |
| TC-D-250 | Windows Guide hover overlay | **通过** run `35474340766` `HOVER_OK path=guide_hover` / `GUIDE_FILE_OK`；无 SendInput |
| TC-D-260 | hover then click still bm_click | **通过** run `35474689948` `HOVER_OK` then `INVOKE_OK path=bm_click`；无 SendInput |
| TC-D-270 | doctor Windows scope is honest | **通过** run `35475108584` `SCOPE_OK` / `BACKEND_OK`；无 AXPress 假绿 |
| TC-D-280 | MCP hover/wait value/abort | **通过** run `35476413803` `TOOLS_OK hover=vcu_hover abort=vcu_session_abort wait.value=true`；`CU-D-280 OK`。无 SendInput |
| TC-D-290 | MCP tools/call live hover | **通过** run `35477727787` `HOVER_OK path=guide_hover os_cursor_used=False via=mcp_tools_call` / `ABORT_OK via=mcp_tools_call` / `CU-D-290 OK`。无 SendInput |
| TC-D-300 | MCP tools/call live click | **通过** run `35478128021` `INVOKE_OK path=bm_click os_cursor_used=False via=mcp_tools_call` / `CU-D-300 OK`。无 SendInput |
| TC-D-310 | MCP tools/call live wait | **通过** run `35478526783` `WAIT_OK path=scene_wait via=mcp_tools_call` / `CU-D-310 OK`。无 SendInput |
| TC-D-320 | MCP tools/call live type | **通过** run `35479050203` `TYPE_OK path=wm_settext os_cursor_used=False via=mcp_tools_call` / `CU-D-320 OK`。无 SendInput |
| TC-D-330 | MCP tools/call live type newline denied | **通过** run `35479929439` `NEWLINE_DENIED via=mcp_tools_call` / `CU-D-330 OK`。无 SendInput |
| TC-D-340 | MCP tools/call live scroll | **通过** run `35480404968` `SCROLL_OK path=wm_vscroll os_cursor_used=False via=mcp_tools_call` / `CU-D-340 OK`。无 SendInput |
| TC-D-350 | MCP tools/call live extract | **通过** run `35481212302` `EXTRACT_OK count=1 via=mcp_tools_call` / `CU-D-350 OK`。无 SendInput |
| TC-D-360 | MCP tools/call live screenshot | **通过** run `35481626341` `SHOT_OK mime=image/png via=mcp_tools_call` / `CU-D-360 OK`。无 CopyFromScreen / SendInput |
| TC-D-370 | MCP tools/call live key return denied | **通过** run `35482034139` `KEY_DENIED via=mcp_tools_call` / `CU-D-370 OK`。无 SendInput |
| TC-D-380 | MCP tools/call live wait miss | **通过** run `35482614685` `WAIT_MISS_OK via=mcp_tools_call` / `WAIT_REF_MISS_OK via=mcp_tools_call` / `CU-D-380 OK`。无 SendInput |
| TC-D-390 | MCP tools/call live doctor | **通过** run `35483037087` `SCOPE_OK via=mcp_tools_call` / `BACKEND_OK via=mcp_tools_call` / `STAGE_OK via=mcp_tools_call` / `CU-D-390 OK`。无 SendInput |
| TC-D-400 | Dual-browser live lens hello | **通过** `CHROME_HELLO_OK` / `EDGE_HELLO_OK` / `MERGE_OK browser_count=2` / `CU-D-400 OK`。组 1/3 未动 |
| TC-D-410 | Drop placeholder lens client | **通过** health chrome+edge count=2，无占位 `browser` |
| TC-D-420 | Allowlist process Chrome | **通过** observe `proc:Chrome` allowed 无 AppDenied；`poc_login_state.sh` PASS；`make check` 0。无 SendInput |
| TC-D-430 | Observe merges extension tabs | **通过** `tabs_source=extension_tabs` tabs=5；HTTP `snapshot_merges_extension_tabs_for_empty_ax_edge`；不是 TC-B-040 |
| TC-D-440 | Daemon login-state observe | **通过** `/v1/browser/observe` 标 `tab_id`；CLI/MCP 同路径；`poc_login_state.sh` PASS tab_id；`make check` 0 |
| TC-D-450 | Bind DOM act to last observe | **通过** `tab_id_source=last_observe`；HTTP observe 后 click 无 tab_id 仍打 42；`poc_login_state.sh` PASS |
| TC-D-460 | Target lens by observe browser | **通过** `browser_hint_targets_client_when_tab_unknown`；重启 daemon 后 selector click 无 wrong_extension_browser |
| TC-D-470 | Screenshot binds last observe | **通过** HTTP screenshot tab_id=42 `tab_id_source=last_observe`；poc screenshot tab 与 observe 相同 |
| TC-D-480 | Open tab in observe browser | **通过** `open_tab_hint_targets_observe_browser`；poc 后台开 example.com 于 Chrome 后关闭 |
| TC-D-490 | Desktop scene browser tabs | **通过** `desktop_scene_attaches_extension_tabs_for_edge` source=ax_scene browser_tabs；close 定向 last observe |
| TC-D-500 | MCP observe-bind instructions | **通过** initialize 含 vcu_browser_observe / last observe / Never CDP Allow |
| TC-D-510 | login-state next_action observe | **通过** next_action 含 observe / last observe；polling=user；poc 打印新文案 |
| TC-D-520 | Dual-browser tabs include Edge | **通过** 同一次 tabs chrome=6 edge=7 `browsers_failed=[]`；`scripts/poc_cu_d_520.py` CU-D-520 OK。组 1/3 未动 |
| TC-D-530 | Observe frontmost USER browser | **通过** 前台 Edge → observe `proc:Microsoft_Edge` `frontmost_matched=true`；`scripts/poc_cu_d_530.py` CU-D-530 OK。组 1/3 未动 |
| TC-D-540 | login-state lists frontmost first | **通过** 前台 Edge → `user_browsers[0]` Edge pid 60318；`scripts/poc_cu_d_540.py` CU-D-540 OK。组 1/3 未动 |

## 8. 未通过不得宣称完成

- TC-B-040 通用 AX 网页像素真点（浏览器计划已延期）  
- 飞书客户端自动发送  
- 任何微信窗口上的动作  
- Windows `vcu session` 产品路径 / Stage HUD / live Invoke
- 把 WM_SETTEXT 写成 UIA ValuePattern

