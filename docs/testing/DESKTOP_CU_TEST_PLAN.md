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

## 8. 未通过不得宣称完成

- TC-B-040 通用 AX 网页像素真点（浏览器计划已延期）  
- 飞书客户端自动发送  
- 任何微信窗口上的动作  
- Windows `vcu session` 产品路径 / Stage HUD / live Invoke
- 把 WM_SETTEXT 写成 UIA ValuePattern

