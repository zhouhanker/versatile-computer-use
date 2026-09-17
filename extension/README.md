# VCU Browser Extension (Chrome / Edge)

MV3 extension that:

- Opens a **non-focused Agent Window** (does not steal user focus)
- Lists tabs and marks agent-owned vs user tabs
- Pairs with local `vcu-daemon` via pairing token

## Load unpacked

1. `vcu init` and copy `pairing_token` from `~/.vcu/config.json`
2. `vcu daemon start`
3. Chrome: `chrome://extensions` → Developer mode → Load unpacked → select this `extension/` folder
4. Edge: `edge://extensions` → same
5. Open the extension popup, paste token, Save

## Notes

- Full DOM click/type relay through the extension is wired on the daemon `extension` backend stub; production path currently recommends **mock** (tests) or **cdp** (real pages) while the extension completes message relay in a follow-up.
- The extension never injects OS-level cursor events.
