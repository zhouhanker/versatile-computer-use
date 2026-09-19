.PHONY: build release pack test poc poc-cdp poc-edge poc-app poc-extra poc-install poc-login poc-extract poc-feishu-scene check

build:
	cargo build

release:
	cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp

pack:
	bash scripts/pack-release.sh

test:
	cargo test --workspace
	node --test extension/tests/*.test.cjs

poc:
	bash scripts/poc_mock_flow.sh

poc-cdp:
	bash scripts/poc_cdp_smoke.sh

poc-edge:
	bash scripts/poc_cdp_edge.sh

poc-app:
	bash scripts/poc_app_macos.sh

poc-extra:
	bash scripts/poc_actions_extra.sh

poc-install:
	bash scripts/poc_install_curl.sh

poc-login:
	bash scripts/poc_login_state.sh

poc-extract:
	bash scripts/poc_browser_extract.sh

poc-feishu-scene:
	bash scripts/poc_feishu_scene.sh

# Browser-only version: no poc-app / poc-feishu / poc-cdp.
# poc-extract is L4.5 (EXTRACT-002) and not in check until live DOM is green.
check:
	cargo test --workspace
	node --test extension/tests/*.test.cjs
	bash scripts/poc_mock_flow.sh
	bash scripts/poc_actions_extra.sh
	bash scripts/poc_login_state.sh
	bash scripts/pack-release.sh
	bash scripts/poc_install_curl.sh
