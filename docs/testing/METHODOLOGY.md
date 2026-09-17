# VCU testing methodology

## Hard bans
- **No WeChat / 微信** automation in any test.
- **Never uninstall/modify Codex Computer Use**.

## Layers
1. Unit/integration: `cargo test --workspace`
2. Install lifecycle: `vcu self info|update|uninstall`
3. Browser mock / CDP Chrome / CDP Edge
4. Browser takeover discover (NEW vs TAKEOVER)
5. App macOS windows/snapshot + Feishu (not WeChat)
6. Domain: Etherscan labelcloud login detection
7. Regression: pack + curl install + make check

## Differential oracle
Same URL in user Edge vs VCU session → compare `login_wall.json` and `mode.txt`.
