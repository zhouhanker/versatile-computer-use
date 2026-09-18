# 09 — macOS second Edge instance for Agent profile

Date: 2026-09-18

## Symptom

`nohup "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge" --user-data-dir=$HOME/.vcu/edge-agent-profile ...` started from a non-GUI agent parent:

- logs `Trying to load the allocator multiple times. This is *not* supported`
- crashpad cannot write the *user* Crashpad DB (`Operation not permitted`)
- the process exits in a few seconds
- `GET /v1/health` can still show `extension_polling=true` for up to 15s after the last poll, which looks healthy while the browser is already dead

`pgrep -f "Microsoft Edge --user-data-dir=$PROF"` is also wrong after Chromium reorders argv. Match `$PROF` instead.

## Fix

```bash
open -n -a "Microsoft Edge" --args \
  --user-data-dir="$PROF" \
  --disable-extensions-except="$EXT" \
  --load-extension="$EXT" \
  --no-first-run --no-default-browser-check \
  about:blank
```

LaunchServices keeps a GUI-capable second instance. User Edge pid (e.g. 1168) stays on the default profile without `--load-extension`.

## Watchdog

`scripts/overnight_watchdog.sh` restarts daemon / Agent Edge if health or polling drops. Do not install LaunchAgent KeepAlive while flock is enabled.
