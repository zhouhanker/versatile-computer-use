# MV3 service worker keepalive (2026-09-18)

## Failures observed

- `"background": { "type": "module" }` without imports: poll loop often never ran.
- Hello-only `extension_connected` ≠ commands work. Need `/poll` heartbeat (`extension_polling`, 15s).
- `bash -lc 'vcu-daemon &'` and `bash -lc 'Edge &'` die on SIGHUP; need `process_group(0)` / `nohup` / persistent PTY.
- `startPollLoop` sticky `polling=true` prevented restart; use generation counter.
- `agentWindowId` in SW memory is lost on restart; persist in `chrome.storage.local`.
- `chrome.alarms` 1-minute period is slower than 15s poll window; offscreen `setInterval` 20s + hello/poll restart is what kept `extension_polling=true` across daemon restarts (~2s).

## Do not

Click Edge Allow. Do not touch Codex CU or WeChat.
