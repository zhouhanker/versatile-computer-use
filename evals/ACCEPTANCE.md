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
| Gate | poc_mock / poc_extra / poc_cdp / poc_app_macos | exit 0 | PASS |

## Commands to re-verify

```bash
cargo test --workspace
bash scripts/poc_mock_flow.sh
bash scripts/poc_actions_extra.sh
bash scripts/poc_cdp_smoke.sh
bash scripts/poc_app_macos.sh   # Darwin
cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp
```


## 2026-09-17 late — installer + MCP

| Item | Evidence | Status |
| --- | --- | --- |
| curl/file install without cargo | `scripts/poc_install_curl.sh` | PASS |
| MCP Content-Length + tools/call HTTP | `cargo test -p vcu-mcp --test mcp_stdio` | PASS |
| Agent protocol doc | `docs/design/05-agent-integration.md` | PASS |
| macOS LaunchAgent scripts | `scripts/macos/install-launch-agent.sh` | PASS (script present) |
