# macOS feature matrix

| Feature | Status | How to use / test |
| --- | --- | --- |
| curl prebuilt install | done | `scripts/poc_install_curl.sh` |
| vcu / daemon / mcp binaries | done | `vcu --version` |
| doctor (config/daemon/ax/browsers) | done | `vcu doctor --json` |
| LaunchAgent service | done | `vcu service install\|status\|uninstall` |
| Browser mock backend | done | `poc_mock_flow.sh` |
| Browser CDP Chrome | done | `poc_cdp_smoke.sh` |
| Browser CDP Edge | done | `poc_cdp_edge.sh` |
| Extension bundle + pair protocol | done | load `extension/`; daemon `/v1/extension/*` |
| tabs borrow / no OS cursor | done | mock+cdp pocs |
| extract / scroll / wait / screenshot | done | CLI + tests |
| Extension zero-click bootstrap | done | `POST /v1/extension/bootstrap`; Origin allowlist |
| Virtual cursor overlay | done | click/hover inject `#vcu-virtual-cursor`; no OS cursor |
| Agent Edge second instance | done | `scripts/start_agent_edge.sh` via `open -n` |
| Vision providers (HTTP + mock) | done | `vcu init model` / vision_mock test |
| MCP stdio Content-Length | done | `cargo test -p vcu-mcp` |
| App list windows (allowlist) | done | `vcu app windows` / `config set-app-allowlist` |
| App snapshot AX | done | soft-degrade without Accessibility |
| App invoke AXPress | done | WeChat denylist; no `VCU_ALLOW_APP_INVOKE` gate |
| Desktop surface + Stage HUD | done | `session start --surface desktop`; capsule 420×44 + Guide |
| Stage Esc/click abort | done | sidecar `*.abort`; doctor `stage_helper` |
| Desktop AX scroll | done | `act type=scroll`; Finder splitter; no OS cursor |
| Desktop key / Return gate | done | Return needs `confirm_send` + Send `ref`; no HID; Esc stays HUD |
| Desktop Scene wait | done | `act type=wait` ms or until ref/name/role; MCP `vcu_wait` |
| Desktop Guide hover | done | `act type=hover` moves overlay only; wait honors Stage abort |
| Bounded AX snapshot | done | BFS depth 13 (Finder 4), cap 80, 4.5s timeout; no `entire contents`; unnamed kept |
| App snapshot pixels | done | `vcu app snapshot <id> --pixels`; no HUD; WeChat denied |
| AX description names | done | empty name filled from description/help (关闭按钮) |
| Scene webview hint | done | snapshot `webview` + `webview_ref` picks largest messenger pane |
| Frame hit (no OS cursor) | done | `click`/`hit` on webview_ref → `ax_frame_hit`; Guide overlay; no CGWarp/HID |
| Webview pixel crop | done | `app snapshot --pixels` writes `webview_screenshot_*` for messenger frame |
| Scene full webview crop | done | desktop `mode=full` Observation has `webview_screenshot_ref` |
| Login-state user browser | done | `vcu browser login-state`; desktop prefers user Edge |
| Login-state observe (no HUD) | done | `vcu browser observe`; MCP `vcu_browser_observe` |
| Login-state next action | done | `vcu browser next`; health `login_next_action` + `lens_dir` |
| User-profile extension lens | done | `vcu browser install-lens`; hello `likely_user_profile` |
| Compact Apple HUD | done | `vcu-stage` 312×32 hudWindow material |
| Pixel→AX Guide mapping | done | `act` `pixel_x/pixel_y` + `screenshot_scale` |
| Pack ships `vcu-stage` (macOS) | done | `pack-release.sh` / `install.sh`; Windows omits helper |
| App focus default deny | done | `FocusPolicyViolation` |
| Audit log | done | `VCU_AUDIT=1` |
| Skills multi-harness | done | `vcu install-skill` |
| CI packages macos-arm64/x64 | done | `.github/workflows/ci.yml` + `release.yml` |

## Deferred (needs Apple cert / policy)

- Notarization / Developer ID signing
