# macOS browser takeover

## NEW browser vs TAKEOVER

| Mode | How | Cookies / login |
| --- | --- | --- |
| NEW | `session start --backend cdp` against headless/temp `--user-data-dir` | **No** — empty profile |
| TAKEOVER | CDP attach to **already running** Chrome/Edge with remote debugging on | **Yes** — same profile |
| PARALLEL | MV3 extension Agent Window in same browser | Same browser process; user tabs need `tabs borrow` |

## Enable TAKEOVER (Edge example)

1. Keep your normal Edge windows open (logged-in sites).
2. Visit `edge://inspect/#remote-debugging` → enable remote debugging (or relaunch Edge once with `--remote-debugging-port=9222`).
3. `vcu browser discover --json`
4. `vcu config set-cdp http://127.0.0.1:<port>`
5. `vcu session start --backend cdp --browser edge --json`
6. Navigate/snapshot/extract — actions use **page DOM**, not OS cursor.

## Codex-like interaction goals

- Prefer dedicated agent tab/window; don’t thrash user tabs without borrow.
- No OS cursor hijack (`os_cursor=deny`).
- Structured refs from snapshot; scroll/wait helpers.
- Future: optional on-screen agent cursor overlay (not shipping yet).

## Never automate

- WeChat / 微信
- Codex Computer Use app (do not uninstall or modify)
