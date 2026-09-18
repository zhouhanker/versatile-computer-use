# CDP takeover: Edge/Chrome UI remote-debugging mode

Date: 2026-09-18

## Symptom

User enables Edge remote debugging via `edge://inspect/#remote-debugging`
("Server running at: 127.0.0.1:9222"). Port listens, but:

- `GET /json/version` → **404**
- `GET /json/list` → empty/404
- `ws://127.0.0.1:9222/devtools/page/<id>` → **403 Connection rejected**
- `ws://127.0.0.1:9222/devtools/browser` → **101** OK

So classic HTTP discovery used by older automation clients fails, while the
**browser-level** DevTools WebSocket works.

## Proven commands (browser WS)

- `Browser.getVersion` → Edg/153…
- `Target.getTargets` → real user pages (Etherscan, GitHub, Feishu, …)
- `Target.createTarget` / `Target.attachToTarget({flatten:true})`
- Session-scoped `Page.navigate`, `Runtime.evaluate`

Visited `edge://inspect/#remote-debugging` via attach and read body text
confirming "Server running at: 127.0.0.1:9222".

## How others do it

| Project | Approach |
| --- | --- |
| chrome-devtools-mcp | `puppeteer.connect({browserWSEndpoint})`; autoConnect reads `DevToolsActivePort` (port + ws path) |
| Playwright | `connectOverCDP(endpointURL)` — HTTP or WS |
| Puppeteer | `connect({browserURL})` uses `/json/version`; or direct `browserWSEndpoint` |
| domdomegg/computer-use-mcp | OS-level screenshot/mouse (not browser CDP takeover) |
| deedy/mac_computer_use | cliclick + screencapture (OS GUI) |
| iFurySt/open-codex-computer-use | Accessibility-first CUA, not CDP |
| google-gemini/computer-use-preview | Playwright-managed Chrome (NEW browser) |
| openai/codex Computer Use | Separate app / AX + virtual cursor (do not modify) |

## VCU fix

1. Discover: accept either HTTP `/json/version` **or** WS `/devtools/browser`.
2. Connect: fall back to `ws://host:port/devtools/browser` when HTTP fails.
3. Pages: `Target.getTargets` when `/json/list` unavailable.
4. Actions: always prefer **flat attach** on browser WS (page WS may 403).
5. Agent surface: `Target.createTarget(about:blank)` so TAKEOVER does not thrash user tabs.
6. Doctor: dual probe (HTTP + WS).

## TCC note

`~/Library/Application Support/Microsoft Edge/DevToolsActivePort` exists but
may be unreadable without Full Disk Access. Port scan + WS probe remains the
primary discovery path on macOS agents.

## Allow debugging dialogs (vs Codex CU)

Chromium/Edge **UI remote-debugging** may show an **Allow debugging** prompt when a
new DevTools client connects. `chrome-devtools-mcp --autoConnect` documents the same.

**Codex Computer Use does not use this path** — it runs a long-lived local helper with
OS Accessibility granted once in System Settings, plus an overlay cursor. No per-action
browser dialog.

### VCU policy

| Backend | Permission model |
| --- | --- |
| `extension` | Load unpacked once + pair token; **no CDP Allow** (Codex-like for browser) |
| `cdp` + persistent browser WS | **At most one** Allow when session starts; reuse connection for all actions |
| `cdp` + temp profile / flag launch | Usually no UI Allow |
| `app` (macOS AX) | Accessibility once in System Settings (Codex-like for apps) |

**Do not** open a new browser WebSocket per navigate/snapshot/click — that re-triggers Allow.

If handshake hangs (`browser_ws_pending`): user likely has an unanswered Allow/Deny dialog in Edge — dismiss or Allow once.
