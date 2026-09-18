# Acceptance matrix (phase-1 MVP)

Verified on 2026-09-17 (local macOS Apple Silicon) unless noted.

| ID | Requirement | Evidence | Status |
| --- | --- | --- | --- |
| R0 | RTK + AWR hosting | `AGENTS.md`, `awr status`, awr 0.4.0 | PASS |
| R1 | Pluggable CU, not tied to vendor model/host | CLI/MCP/Skill; backends mock/cdp/extension | PASS |
| R2 | Cross-platform language + macOS/Windows delivery path | Rust; `docs/INSTALL.md`; CI windows-latest | PASS (Win UIA deep tree = later) |
| R3a | Browser first Chrome/Edge | CDP smoke real Chrome; Edge binary supported in scripts | PASS |
| R3b | No OS cursor hijack | `OsCursorDenied` in mock/cdp/app pocs | PASS |
| R3c | Take over / existing browser | CDP attach; extension Agent Window + borrow | PASS |
| R3d | Scrape | `extract` + snapshot a11y/dom/text | PASS |
| R4 | Easy agent integration | `vcu` shell, `vcu-mcp`, `install-skill` | PASS |
| R5 | Research then design before code | `docs/research/*`, `docs/design/*` | PASS |
| R6 | Vision config for non-multimodal main agent | `vcu init model`, mock+HTTP providers | PASS |
| R6b | Main/sub collaboration hooks | blackboard, `agent spawn-vision` | PASS |
| R7 | AWR lessons | session/checkpoint/doctor/CLI=MCP/source_first | PASS |
| P1 | `vcu init` / doctor / daemon | poc_mock_flow | PASS |
| P1 | session navigate/click/type/snapshot/screenshot | poc_mock + poc_cdp | PASS |
| P1 | tabs borrow required | poc_mock 409 BorrowRequired | PASS |
| P1 | audit log optional | VCU_AUDIT=1 poc_actions_extra | PASS |
| P1 | extension package loadable | `extension/manifest.json` MV3 | PASS (manual load) |
| Gate | `cargo test --workspace` | local run | PASS |
| Gate | poc_mock / poc_extra / poc-login / poc_app_macos / poc-feishu-scene | exit 0 | PASS (CDP not required) |

## Commands to re-verify

```bash
cargo test --workspace
bash scripts/poc_mock_flow.sh
bash scripts/poc_actions_extra.sh
bash scripts/poc_login_state.sh
bash scripts/poc_app_macos.sh          # Darwin
bash scripts/poc_feishu_scene.sh       # Darwin; never sends; AX has no 发送
# poc_cdp_smoke.sh is leftover — not a login-state / Feishu gate
cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp
```


## 2026-09-17 late — installer + MCP

| Item | Evidence | Status |
| --- | --- | --- |
| curl/file install without cargo | `scripts/poc_install_curl.sh` | PASS |
| MCP Content-Length + tools/call HTTP | `cargo test -p vcu-mcp --test mcp_stdio` | PASS |
| Agent protocol doc | `docs/design/05-agent-integration.md` | PASS |
| macOS LaunchAgent scripts | `scripts/macos/install-launch-agent.sh` | PASS (script present) |

## Lifecycle / local product tests (2026-09-18)

| Test | Result |
| --- | --- |
| Local install to ~/.local | PASS |
| `vcu self info/update/uninstall` | PASS (`poc_self_lifecycle.sh`) |
| Codex CU untouched | PASS (path still exists after uninstall test on temp prefix) |
| Feishu send "Test" to 张北北 | `osa_out=ok` **FALSE GREEN** — user did not receive; lark-cli is not product | **FAIL** |
| Etherscan labelcloud | PASS mode=NEW_HEADLESS_NO_LOGIN, login_wall=true, labels_flat saved under ~/vcu-etherscan-labels |
| WeChat not automated | PASS (allowed=false; no scripts) |
| browser discover | PASS (0 CDP on live profile without remote-debug enable) |


## Overnight 2026-09-18 01:14–02:03 CST

| Test | Result |
| --- | --- |
| `POST /v1/extension/bootstrap` loopback, no auth | PASS (`poc_extension_bootstrap.sh`) |
| `vcu daemon start` already_running / flock | PASS |
| SIGHUP detach (daemon survives `bash -lc`) | PASS |
| Zero-click Agent Edge pairing (`extension_polling=true`) | PASS |
| `poc_extension_session` example.com snapshot | PASS |
| `cargo test --workspace` | PASS (19) |
| Etherscan L1/L2/L3 via extension | PARTIAL (public first page; Sign In wall for full L3) |
| Feishu Test to 张北北 | BLOCKED (2 exact-name candidates; see `~/vcu-feishu-poc/decision.json`) |
| CDP 9222 Allow | NOT CLICKED (WS timeout) |
| Codex CU / WeChat | untouched |

## Overnight 2026-09-18 02:05–02:29 CST

| Test | Result |
| --- | --- |
| Bootstrap Origin https://evil.example | 401 |
| Bootstrap chrome-extension Origin | PASS (test) |
| Health pid + last_poll_age_ms | PASS |
| cargo test --workspace | PASS (20) |
| Agent Edge via open -n | PASS (polling=true, user Edge 1168 untouched) |
| poc_extension_session example.com | PASS (snapshot/extract Example Domain) |
| Etherscan L1 hrefs via extension | PASS (800) |
| Etherscan L3 public pages | PARTIAL (~11 addrs/page; Sign In wall for full list) |
| Feishu Test to 张北北 | BLOCKED (2 exact names; IM evidence in decision.json) |
| CDP 9222 Allow | NOT CLICKED |
| Codex CU / WeChat | untouched |


## Stage + Steward slice 1（2026-09-18）

| Item | Evidence | Status |
| --- | --- | --- |
| Design 06 accepted | `docs/design/06-stage-steward.md`, ledger STEW-001 | PASS |
| `surface=desktop` session | `crates/vcu-server/tests/desktop_session.rs` | PASS |
| WeChat `AppDenied` | mock desktop + app_http | PASS |
| AXPress without OS cursor | `os_cursor_used=false` | PASS |
| Accessibility doctor hint | 系统设置 → 隐私与安全 → 辅助功能 | PASS |
| Codex CU / WeChat untouched | constraint | PASS |

## Stage + Steward slice 2（2026-09-18）

| Item | Evidence | Status |
| --- | --- | --- |
| Guide overlay before AXPress | click `detail.guide.overlay=true`, `os_cursor_used=false` | PASS |
| Scene AX frames | `dom_refs[].frame` | PASS |
| Mock window capture | full snapshot `screenshot_ref` | PASS |
| Live screencapture gated | `VCU_ALLOW_SCREENCAPTURE` | PASS |
| WeChat / Codex CU / no UI click | constraint | PASS |


## Stage HUD STEW-003（2026-09-18 13:30 CST）

| Item | Evidence | Status |
| --- | --- | --- |
| Capsule HUD 420×44, not full-width | CGWindow bounds Width=420 Height=44 X=2478 Y=38 | PASS |
| Guide 56×56 overlay, AX center match | window (2248,177) center (2276,205) vs click guide (2276.5,205); os_cursor_used=false | PASS |
| WeChat AppDenied / os_click OsCursorDenied | live session 01M2SFW7F0ZDK8B0Q1BVVZ5QST | PASS |
| session stop tears down helper | pgrep vcu-stage empty after stop | PASS |
| cargo test --workspace | 29 passed | PASS |
| Codex CU / WeChat / no UI click | constraint | PASS |


## Stage Esc abort STEW-004（2026-09-18 13:40 CST）

| Item | Evidence | Status |
| --- | --- | --- |
| Abort sidecar stops session | write `*.abort` → session list empty, vcu-stage gone | PASS |
| Esc/HUD click write abort | `helpers/vcu-stage/main.swift` keyCode 53 + HUD click | PASS (key path unit+code; live used sidecar to avoid injecting Esc) |
| cargo test --workspace | 30 passed | PASS |


## Desktop scroll STEW-006（2026-09-18 13:46 CST）

| Item | Evidence | Status |
| --- | --- | --- |
| Mock act scroll | desktop_session.rs input_path=ax_scroll | PASS |
| Abort watcher removes session | api.rs abort_watch_removes_session | PASS |
| Live Finder AX scroll | ok-split-sa, os_cursor_used=false, then session stop | PASS |
| cargo test --workspace | 31 passed | PASS |


## Login-state / Codex-like desktop (2026-09-18)

Host vision (Grok) does not need `vcu init model`. Gate: `make poc-login`.

| ID | Requirement | Evidence | Status |
| --- | --- | --- | --- |
| L1 | User vs Agent Edge classified | `vcu browser login-state` | PASS |
| L2 | Observe USER window without HUD | `vcu browser observe`; `hud=false` | PASS |
| L3 | Retina scale + latest png/json | `~/.vcu/captures/login-latest.{png,json}` | PASS |
| L4 | Pixel to AX | `ax_point_from_pixel`; `vcu click --pixel-x` | PASS |
| L5 | Compact Apple HUD | live 220×28 hudWindow size-to-fit | PASS |
| L6 | User-profile extension lens | user installed VCU Browser Bridge; `extension_profile=user` | PASS |
| L7 | CDP Allow never auto-clicked | policy | PASS |
| L8 | Feishu verified send | FEISHU-001 App+vision; Scene only so far | **FAIL** (not sent) |
| L9 | AX page_url + tabs + WebArea without CDP | `vcu browser observe` `page_url`/`tabs`/`webview` | PASS (DOM lens still L6) |
| L10 | No-HUD pixel map on login-state | `vcu browser click --dry-run` ax=webview origin | PASS |
| L11 | Guide×retina flash without HUD | `--guide` overlay at ax, leftover stage=0 | PASS |
| L12 | Wait + center-pixel map | wait AXWebArea e85; center ax=origin+size/2 | PASS |
| L13 | Default no-HUD loop + Return gate | `make poc-login` observe/click/type/wait/key blocked | PASS |


## 2026-09-18 22:10 — honesty

| Item | Status |
| --- | --- |
| Product plan | `docs/PLAN.md` is the sequence |
| CDP login-state | abandoned |
| Feishu App send via vision | NOT DONE |
| `cargo test --workspace` | 59 passed; does **not** prove Feishu delivery |

## 2026-09-18 22:20 — TEST-001 P0 closed

| Gate | Result |
| --- | --- |
| Feishu send in ACCEPTANCE | FAIL until chat screenshot |
| `ax_exposes_send_control` unit test | live-like AX names have no 发送 |
| `poc_feishu_scene` | asserts AX no 发送, never sends |
| `poc_feishu_message.sh` | exits 2; refuses lark-cli/osascript |
| `make check` / `release_check.sh` | poc-login + poc-feishu-scene; CDP not required |
