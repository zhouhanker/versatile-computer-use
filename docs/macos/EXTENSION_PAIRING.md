# VCU extension pairing (zero-click)

Codex-like daily path: **no CDP Allow dialogs**.

## What pairs

1. Daemon listens on `http://127.0.0.1:17890`.
2. Unauthenticated **loopback-only** `POST /v1/extension/bootstrap` returns `{ token, endpoint }`.
3. Bootstrap **rejects** `Origin` unless it is `chrome-extension://`, `moz-extension://`, `safari-web-extension://`, or omitted (curl/native). A normal webpage Origin gets 401 so the pairing token cannot be stolen via CORS.
4. Extension service worker stores the token and long-polls `/v1/extension/poll`.
5. `GET /v1/health` reports `extension_connected`, `extension_polling`, `pid`, `last_poll_age_ms`.

## Start (no UI clicks)

```bash
vcu daemon start --json          # no-op if already running
bash scripts/start_agent_edge.sh # open -n isolated profile + --load-extension
vcu session start --backend extension --browser edge --json
```

Agent profile: `~/.vcu/edge-agent-profile`  
Extension files: `~/.local/share/vcu/extension`

On macOS, **do not** exec the Edge binary from a non-GUI parent (`nohup .../Microsoft Edge ...`). That process often exits in seconds (`Trying to load the allocator multiple times`). `open -n -a "Microsoft Edge" --args --user-data-dir=...` keeps a real instance.

This profile has **no user cookies**. Login-state takeover needs either:

- CDP attach to the *user* Edge (may show Allow **once**; do not spam reconnects), or
- load the unpacked extension into the user profile (one-time).

LaunchAgent `KeepAlive` is **false** in the template. flock + KeepAlive=true crash-loops if a second daemon cannot take the lock.

## Do not

- Click the user’s Edge Allow/Deny dialog unless the user asked.
- Automate WeChat.
- Touch `~/.codex/computer-use/`.
