# AGENTS.md — versatile-computer-use (VCU)

## Shell / RTK

This project requires [RTK](https://github.com/rtk-ai/rtk) for agent shell commands.

- Always prefix shell commands with `rtk` (see `~/.codex/RTK.md`).
- If `rtk` is missing: `curl -fsSL https://raw.githubusercontent.com/rtk-ai/rtk/refs/heads/master/install.sh | sh`
- Verify: `rtk --version` and `which rtk`.

## AWR project hosting

Work state is hosted by [AWR](https://github.com/originoneai/agent-work-runtime) (`awr` ≥ 0.4.0).

```sh
awr --project . status
awr --project . ready
awr --project . intake inspect --json
awr --project . context compile --work <ID> --goal 'goal#vcu' --budget 5000
```

Authoritative sources (in order):

- `docs/PLAN.md` — current version plan (browser-only)
- `.awr/intake/GOALS.md`
- `.awr/intake/work-ledger.yaml`
- `docs/HANDOFF.md` — session continuity (do not treat stacked snapshots as the plan)
- `docs/testing/BROWSER_TEST_PLAN.md` / `BROWSER_TEST_CASES.md`

## Working rules

- **This version is browser-only Computer Use.** Feishu/Lark client, Finder, WeChat, CDP Allow, OS cursor warp are out of scope.
- Login-state path: USER Edge/Chrome + VCU Browser Bridge (`~/.vcu/lens-extension`). Host vision (Grok) is enough; do not require `vcu init model`.
- DOM extract/click/type/scroll must be `source=extension_dom` when using the extension. AX chrome is not HTML DOM.
- Never click Edge Allow debugging. Never automate WeChat. Never warp the OS cursor.
- Prefer editing authoritative Markdown/YAML; reindex with `awr source reindex`.
- Preserve unrelated edits; keep commits focused.

## Session continuity

On a new session, read **`docs/PLAN.md` then `docs/HANDOFF.md`**.
