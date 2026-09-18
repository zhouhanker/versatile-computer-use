# macOS browser takeover

> **2026-09-18：CDP 路径已抛弃。** 登录态不要 `set-cdp`、不要 `edge://inspect`、不要让用户点 Allow。  
> 权威路径：USER Edge Load unpacked `~/.vcu/lens-extension`（VCU Browser Bridge）→ `vcu browser observe` / extension extract。


## NEW browser vs TAKEOVER

| Mode | How | Cookies / login | Allow dialogs |
| --- | --- | --- | --- |
| NEW | `session start --backend cdp` against headless/temp `--user-data-dir` | **No** — empty profile | Usually none |
| TAKEOVER (CDP) | CDP attach to **already running** Chrome/Edge with remote debugging on | **Yes** — same profile | **At most once** per VCU daemon session (persistent browser WS) |
| PARALLEL (extension) | MV3 extension Agent Window in same browser | Same browser process; user tabs need `tabs borrow` | **None** (Codex-like; preferred for daily use) |

## Why Codex Computer Use does not ask “Allow debugging” every click

Codex CU is a **long-lived local helper** using Accessibility + overlay cursor — not a fresh CDP client on every action.

VCU mirrors that for CDP:

1. `session start --backend cdp` opens **one** browser WebSocket.
2. All navigate / snapshot / click reuse that connection + cached `Target.attachToTarget` sessions.
3. You should only see Edge/Chrome “Allow debugging” when that session first connects (if the browser requires it) — **not** on every action.

Prefer **extension** backend when you want zero CDP prompts.

## Enable TAKEOVER (Edge example)

1. Keep your normal Edge windows open (logged-in sites).
2. Visit `edge://inspect/#remote-debugging` → enable remote debugging **once**
   (page may show `Server running at: 127.0.0.1:9222`).
3. `vcu browser discover --json`  
   - Expect `mode=browser_ws` if HTTP `/json/version` is 404 (common on UI enable).  
   - Expect `mode=http_json` if launched with `--remote-debugging-port`.
4. `vcu config set-cdp http://127.0.0.1:<port>`
5. `vcu session start --backend cdp --browser edge --json`  
   - If a single Allow dialog appears, click **Allow** once.
6. Navigate/snapshot/extract — actions use **page DOM**, not OS cursor; **no per-action Allow**.

### Note on HTTP 404

UI-enabled remote debugging often:

- serves `ws://127.0.0.1:9222/devtools/browser` (OK)
- returns 404 on `/json/version` and 403 on `/devtools/page/<id>`

VCU handles this via browser WS + flat attach (see `docs/research/07-cdp-takeover-edge-ui-mode.md`).

## Codex-like interaction goals

- Prefer dedicated agent tab/window; don’t thrash user tabs without borrow.
- No OS cursor hijack (`os_cursor=deny`).
- Structured refs from snapshot; scroll/wait helpers.
- One permission grant model (Accessibility / extension / single CDP Allow).
- Future: optional on-screen agent cursor overlay (not shipping yet).

## Never automate

- WeChat / 微信
- Codex Computer Use app (do not uninstall or modify)

## Zero-click extension path

See `docs/macos/EXTENSION_PAIRING.md`. Daily use should be `--backend extension` (no CDP Allow). CDP UI remote-debugging may still  hang on Allow; do not click unless the user asks.
