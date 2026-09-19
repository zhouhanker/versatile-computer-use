---
name: vcu
description: USER Chrome/Edge computer use with named native tab groups, DOM actions and screenshot-bound clicks; does not move the OS cursor.
---

# VCU browser skill

Use VCU CLI or MCP on the user's logged-in Chrome/Edge through VCU Browser Bridge. This version is browser-only. Host vision is sufficient; do not require model configuration when the host can see images.

## Workflow

1. `vcu browser ping --json` must pong; reload an outdated Bridge if needed.
2. `vcu browser tabs --json`, choose the exact `tab_id`. `browser select --tab ID` explicitly selects and expands its group.
3. For visual webpage actions, `browser screenshot --tab ID --json`. View the returned PNG before using coordinates.
4. `browser click --space viewport --capture CAPTURE_ID --pixel-x X --pixel-y Y`. Use pixels from that exact PNG, which excludes browser chrome. Captures expire after 60 seconds and one real action consumes them.
5. For DOM, use unique selectors and explicit `--tab`: click/type/extract/scroll. `source` must be `extension_dom`.
6. Observe/extract after acting to verify the intended outcome. A dispatched click is not proof of business success.

```sh
vcu browser open --url https://example.com --session-name '🔎 Research'
vcu browser tabs --json
vcu browser screenshot --tab <id> --json
# View screenshot_path, then:
vcu browser click --space viewport --capture <capture_id> --pixel-x <x> --pixel-y <y>
vcu browser type --tab <id> --selector 'input[name=q]' --text 'hello'
vcu browser extract --tab <id> --selector '#result'
vcu browser group-update --group <group_id> --collapsed true
```

## Interaction rules

- Explicit missing tabs fail; never substitute another page. Selectors must resolve uniquely and must not be hidden/disabled/occluded. Input only targets editable controls.
- On changed layout/document/scroll or timeout, capture again and inspect before deciding whether another action is needed. Never blindly replay an uncertain mutation.
- DOM events report `trusted=false`; iframe/canvas targets requiring native gestures are unsupported. Do not call this successful native input.
- `browser observe` captures the whole native browser window. Its AX `window/webview` coordinates are distinct from extension `viewport` screenshot coordinates. Do not mix them.
- Group only explicitly selected same-window tabs. `open --new-window` starts a separate USER-profile task window; `--background` preserves the current focus.
- Do not require a desktop session/HUD. Mock sessions and borrow APIs are for the separate test/session path.
- Never click debugging Allow, use CDP takeover, automate WeChat, move the OS cursor, or press blind Return. Do not modify the Codex CU installation.

Full protocol and examples: `playbooks/user-browser.md`.
