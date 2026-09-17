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
| Vision providers (HTTP + mock) | done | `vcu init model` / vision_mock test |
| MCP stdio Content-Length | done | `cargo test -p vcu-mcp` |
| App list windows (allowlist) | done | `vcu app windows` / `config set-app-allowlist` |
| App snapshot AX | done | soft-degrade without Accessibility |
| App invoke gated AXPress | done | `VCU_ALLOW_APP_INVOKE=1` |
| App focus default deny | done | `FocusPolicyViolation` |
| Audit log | done | `VCU_AUDIT=1` |
| Skills multi-harness | done | `vcu install-skill` |
| CI packages macos-arm64/x64 | done | `.github/workflows/ci.yml` + `release.yml` |

## Deferred (needs Apple cert / policy)

- Notarization / Developer ID signing
