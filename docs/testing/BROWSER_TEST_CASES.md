# 浏览器测试用例

> 对应 `docs/testing/BROWSER_TEST_PLAN.md`  
> ID 前缀 TC-B。本版本不含 App。

| ID | 标题 | 前置 | 步骤 | 期望 | 自动化 |
| --- | --- | --- | --- | --- | --- |
| TC-B-001 | 用户 Edge 命令行判为 User | 无 | `classify_browser_command("Microsoft Edge", 无 --user-data-dir/.vcu)` | `BrowserProfile::User` | `classifies_user_agent_helper` |
| TC-B-002 | Agent 空 profile 判为 Agent | 无 | 命令行含 `edge-agent-profile` 或 `/.vcu/` | `BrowserProfile::Agent` | 同上 |
| TC-B-003 | Helper/Renderer 不是登录态 | 无 | `--type=renderer` | `Helper` | 同上 |
| TC-B-004 | 选 tab 优先 user Edge 而非 Agent | 无 | tabs 含 TextEdit、Agent Edge、User Edge、WeChat | 选 user Edge | `pick_prefers_user_edge_over_agent_and_textedit` |
| TC-B-005 | 扩展挂在 Agent 不是登录态 | 无 | user 无 load-extension，agent 有 | `extension_profile=agent` | `extension_on_agent_profile_is_not_login_state` |
| TC-B-006 | 扩展挂在 User 才是登录态 | 无 | user 命令行含 load-extension | `user` | 同上 |
| TC-B-007 | Allow 对话框可检测但不得引导点击 | 无 | `classify_debug_ui("Allow debugging…")` 为 allow；`login_next_action(..., allow=true)` | 检测为 true；文案无 `click Allow debugging` / `User: click Allow` | `debug_ui_detects_infobar_and_allow` + `next_action_never_asks_to_click_allow` |
| TC-B-008 | 无用户浏览器时 next_action 要求打开用户窗 | 无 | `login_next_action(true, …)` | 提到 USER Chrome/Edge，不提点 Allow | `next_action_never_asks_to_click_allow` |
| TC-B-009 | inspect 恒禁止 Allow/光标/微信 | 无 | `inspect_login_browsers()` | `never_click_allow/os_cursor/wechat` 全 true | `inspect_always_denies_allow_cursor_wechat` |
| TC-B-010 | Return 无 confirm_send 拒绝 | 无 | `plan_login_key("return", false, true)` | `FocusPolicyViolation` | `plan_login_key_gates_return` |
| TC-B-011 | Return 有 confirm 无 Send ref 拒绝 | 无 | `plan_login_key("enter", true, false)` | `FocusPolicyViolation` | 同上 |
| TC-B-012 | Escape 不注入 | 无 | `plan_login_key("escape", …)` | `FocusPolicyViolation` | 同上 |
| TC-B-013 | HTTP Return dry-run 无 confirm 为 blocked | daemon | `POST /v1/browser/key` return dry_run | `blocked=true` `pressed=false` `hid_injected=false` | `browser_gates::key_return_dry_run_blocked` |
| TC-B-014 | HTTP Return 非 dry-run 无 confirm 错误 | daemon | 同上无 dry_run | HTTP 冲突/错误码 FocusPolicyViolation | `browser_gates::key_return_live_rejected` |
| TC-B-015 | HTTP login-state 政策位 | daemon | `GET /v1/browser/login-state` | never_* true；next_action 不含 click Allow debugging | `browser_gates::login_state_policy` |
| TC-B-016 | Retina scale 识别 | 无 | 2848×2076 vs 1424×1038 pt | scale=2 | `pixel_scale_snaps_retina_and_rejects_junk` |
| TC-B-017 | 非 1/2/3 scale 拒绝 | 无 | 200×50 vs 100×100 | None | 同上 |
| TC-B-018 | 像素 (0,0) webview → frame 原点 | 无 | map webview 0,0 scale 2 | ax=frame origin | `ax_point_from_pixel_retina_feishu_webview` + `map_window_and_oob` |
| TC-B-019 | 像素中心 → origin+size/2 | 无 | 宽高一半 | 误差 <0.6pt | 同上 + `poc_login_state.sh` |
| TC-B-020 | 越界像素拒绝 | 无 | pixel 超过 frame*scale | None | `map_window_and_oob` |
| TC-B-021 | window space 与 webview space 不同原点 | 无 | 同一像素两种 space | ax 不同 | `map_window_and_oob` |
| TC-B-022 | mock session 禁止未授权 | daemon | 无 token start | 401 | `http_poc` / `browser_gates::mock_session_core` |
| TC-B-023 | mock session os_cursor deny | daemon | act os_click | OsCursorDenied | 同上 |
| TC-B-024 | mock 用户 tab 未 borrow 不能 navigate | daemon | navigate user tab | BorrowRequired 409 | 同上 |
| TC-B-025 | mock borrow 后可 navigate | daemon | borrow 再 navigate | 200 | 同上 |
| TC-B-026 | mock click os_cursor_used=false | daemon | click ref | false | 同上 |
| TC-B-027 | mock type / extract / scroll / wait | daemon | 各 act | ok 且无 OS 光标 | `browser_gates::mock_session_core` |
| TC-B-028 | mock snapshot 有 dom_refs | daemon | snapshot full | len≥1 | 同上 |
| TC-B-029 | 真机 observe 无 HUD 有 PNG | 用户 Edge | `vcu browser observe` | hud=false；login-latest.png>1000B；scale∈{1,2,3} | `poc_login_state.sh` |
| TC-B-030 | 真机 click dry-run 不按下 | 用户 Edge | click --dry-run --guide | pressed=false；有 ax_point；无残留 vcu-stage | `poc_login_state.sh` |
| TC-B-031 | 真机 type dry-run 不写入 | 用户 Edge | type --dry-run | typed=false；可读地址栏 | `poc_login_state.sh` |
| TC-B-032 | 真机 wait AXWebArea | 用户 Edge | wait --role AXWebArea | found_ref 非空 | `poc_login_state.sh` |
| TC-B-033 | 真机 scroll dry-run 不滚 | 用户 Edge | scroll --dry-run | scrolled=false | `poc_login_state.sh` |
| TC-B-034 | vision_handoff.must_view 在有 PNG 时出现 | 构造 JSON | stamp 含 screenshot_path | must_view 含该路径 | `stamp_vision_handoff_lists_pngs` |
| TC-B-035 | MCP 有 PNG 时 content 含 image | 临时 png | mcp_vision_content | type=image | `mcp_vision_content_attaches_png` |
| TC-B-036 | MCP 无 PNG 仅 text | 无 | 同上 | len=1 text | `mcp_vision_content_without_png_is_text_only` |
| TC-B-037 | 扩展 bootstrap 无需 auth | daemon | POST /v1/extension/bootstrap | token | `extension_bootstrap_returns_token_without_auth` |
| TC-B-038 | 扩展非 extension Origin 拒绝 | daemon | Origin https://evil | 401 | `extension_bootstrap_rejects_non_extension_origin` |
| TC-B-039 | 微信不是浏览器登录态 | 无 | classify Feishu/WeChat 命令 | App | `classifies_user_agent_helper` |
| TC-B-040 | L5 非 dry-run 像素点击 | 用户授权真点 | LOGIN-LIVE | 本版本登记、不进 check | 无（显式延期） |
| TC-B-041 | 无扩展时 extract 失败 | daemon 无 poll | POST /v1/browser/extract | ok=false ExtensionDisconnected/ActionFailed | `extract_requires_user_extension` |
| TC-B-042 | 真机 USER 扩展 extract 当前页 | extension_profile=user 且 ping pong | `vcu browser extract --selector a` | hud=false login_state `source=extension_dom`；禁止 ax_scene_fallback；不 navigate | `poc_browser_extract.sh` |
| TC-B-043 | 假扩展 ping pong | daemon + poller | POST /v1/browser/ping | pong=true | `browser_gates::ping_with_fake_extension` |
| TC-B-044 | ping unknown method 视为 stale | daemon + poller 返回 unknown method ping | POST /v1/browser/ping | ok=false；文案含 Reload/stale；无 click Allow | `browser_gates::ping_unknown_method_is_stale` |
| TC-B-045 | extract 超时不得 AX 假绿 | daemon + poller 只答 list_tabs | POST /v1/browser/extract | ok=false；无 ax_scene_fallback | `browser_gates::extract_timeout_is_error_not_ax_ok` |
| TC-B-046 | 微信窗口盖住像素则拒绝真点 | 构造 window 表 | denied_app_covering_point | 命中 WeChat 返回 Some；仅 Edge 返回 None | `wechat_is_hard_denied` |
| TC-B-047 | 真点 system-wide 先判 pid | JXA 文案 | denied-app → AppDenied；other-pid 不穿透 | `hit_error_code` + `login_scroll_script_prefers_webarea` |
| TC-B-048 | Edge scroll 优先 AXWebArea | 源码 | 脚本含 ok-webarea / first scroll area | `login_scroll_script_prefers_webarea` |
| TC-B-049 | MCP 列出 ping/extract 登录态工具 | mcp stdio | tools/list | 含 vcu_browser_ping 与 vcu_browser_extract | `mcp_stdio` |
| TC-B-050 | login-state 标记 stale SW | daemon + unknown method ping | GET /v1/browser/login-state | extension_sw_stale=true；next_action 含 Reload；无 click Allow | `login_state_marks_stale_sw` |
| TC-B-051 | Etherscan POC 禁止 CDP | 无/有扩展 | `poc_etherscan_labels.sh` | 不 set-cdp；stale 或非 etherscan 则 SKIP 并写 blocker；成功须 source=extension_dom | `scripts/poc_etherscan_labels.sh` |
| TC-B-052 | 无扩展时 selector type 失败 | daemon 无 poll | POST /v1/browser/type selector | ok=false | `type_selector_requires_user_extension` |
| TC-B-053 | 假扩展 DOM type source=extension_dom | daemon + poller | POST type selector | hud=false os_cursor=false source=extension_dom | `type_selector_with_fake_extension` |
| TC-B-054 | 无扩展时 selector click 失败 | daemon 无 poll | POST /v1/browser/click selector | ok=false | `click_selector_requires_user_extension` |
| TC-B-055 | 假扩展 DOM click source=extension_dom | daemon + poller | POST click selector dry_run | hud=false os_cursor=false source=extension_dom | `click_selector_with_fake_extension` |
| TC-B-056 | 假扩展 DOM scroll source=extension_dom | daemon + poller | POST /v1/browser/scroll | scrolled=true source=extension_dom 无 OS 光标 | `scroll_with_fake_extension_uses_dom` |
| TC-B-057 | selector click 回传 tab_id/page_url/focused | daemon + poller | POST click selector dry_run | tab_id、page_url、focused 在 data 上 | `click_selector_with_fake_extension` |

## 本版本不收的用例（App，禁止当浏览器绿）

| ID | 原因 |
| --- | --- |
| TC-A-* 飞书 Scene/发送 | 本版本放弃 App |
| WeChat 自动化 | 硬禁止 |
| CDP Allow 一次点击 | 已抛弃 |
