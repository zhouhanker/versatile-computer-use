---
name: vcu
harness: cursor
description: Versatile Computer Use — browser automation without hijacking the user OS cursor.
---

# VCU skill

Use the `vcu` CLI for browser computer-use that is model/host agnostic.

## Rules

1. Run `vcu doctor --json` before the first session.
2. Prefer `--backend mock` only for tests; for real Chrome/Edge use extension or CDP.
3. Always keep a `session_id` from `vcu session start --json`.
4. Never attempt OS cursor moves; if you see `OsCursorDenied`, use page refs.
5. User tabs require `vcu tabs borrow` before click/type.
6. If the main model has no vision and DOM is insufficient, ask the user to run `vcu init model` or call `vcu agent spawn-vision`.

## Minimal flow

```sh
vcu init
vcu daemon start
SID=$(vcu session start --backend mock --json | jq -r .data.session_id)
vcu navigate --session "$SID" --url https://example.com
vcu snapshot --session "$SID" --mode a11y --json
vcu click --session "$SID" --ref e3
vcu session stop "$SID"
```


## Desktop apps (macOS)

```sh
vcu app windows --json
vcu app snapshot 'proc:TextEdit:123' --json
```

- Default allowlist: TextEdit, Notes, Safari, Terminal, Ghostty, Finder
- `app focus` / `app invoke` are **denied by default** (no focus steal / no OS cursor)
- Full UI hierarchy needs macOS Accessibility permission for the terminal/daemon host
