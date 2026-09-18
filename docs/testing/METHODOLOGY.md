# VCU testing methodology

## This version
**Browser only.** Desktop App (Feishu client, Finder, etc.) is out of scope.

## Hard bans
- **No WeChat / 微信** automation in any test.
- **Never uninstall/modify Codex Computer Use**.
- **Never click Edge Allow debugging**.
- Do not treat App/Feishu POCs as browser-version gates.
- Do not treat AX chrome (`source=ax_scene_fallback`) as HTML DOM extract.

## Layers
1. Unit/integration: `cargo test --workspace` (includes TC-B policy/coord/mock session/extract/ping)
2. Mock browser POC: `scripts/poc_mock_flow.sh`
3. Live login-state dry-run: `scripts/poc_login_state.sh` (user Edge)
4. Live DOM extract: `scripts/poc_browser_extract.sh` (ping pong + `source=extension_dom`)
5. Install lifecycle: `vcu self info|update|uninstall`
6. App / Feishu / CDP: **not in make check this version**

## Authority
- Plan: `docs/PLAN.md`
- Browser test plan: `docs/testing/BROWSER_TEST_PLAN.md`
- Cases: `docs/testing/BROWSER_TEST_CASES.md`

## False greens (do not repeat)
- `osa_out=ok` / lark-cli send is not VCU.
- Mock Send ref is not a real composer Send.
- `poc-login` dry-run is not a live click (TC-B-040 / LOGIN-LIVE).
- CDP smoke is not login-state.
- `vcu browser extract` with `source=ax_scene_fallback` is not DOM extract.
- `unknown method ping` is a stale service worker, not a working lens.
- Clicking through a WeChat/微信 overlay is not browser CU; live press must AppDenied.
