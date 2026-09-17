.PHONY: build release pack test poc poc-cdp poc-edge poc-app poc-extra poc-install check

build:
	cargo build

release:
	cargo build --release -p vcu-cli -p vcu-daemon -p vcu-mcp

pack:
	bash scripts/pack-release.sh

test:
	cargo test --workspace

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

check:
	cargo test --workspace
	bash scripts/poc_mock_flow.sh
	bash scripts/poc_actions_extra.sh
	bash scripts/poc_cdp_smoke.sh
	bash scripts/poc_cdp_edge.sh
	@if [ "$$(uname -s)" = "Darwin" ]; then bash scripts/poc_app_macos.sh; fi
	bash scripts/pack-release.sh
	bash scripts/poc_install_curl.sh
