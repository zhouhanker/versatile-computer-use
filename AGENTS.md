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

- `docs/PLAN.md` — current released version (browser Bridge 0.2.8)
- `docs/WINDOWS-DEV.md` — Windows checkout, Edge-only test, and unpacked extension update. Do not publish the extension to a store.
- `docs/ROADMAP-CU.md` — next epic: Codex-like CU including desktop
- `docs/testing/DESKTOP_CU_TEST_PLAN.md` — desktop CU tests
- `.awr/intake/GOALS.md`
- `.awr/intake/work-ledger.yaml`
- `docs/HANDOFF.md` — session continuity (do not treat stacked snapshots as the plan)
- `docs/testing/BROWSER_TEST_PLAN.md` / `BROWSER_TEST_CASES.md`

## Working rules

- **Released 0.2.8 is browser Computer Use.** Next epic is desktop CU per `docs/ROADMAP-CU.md`. Do not implement a desktop slice until that ID is the active work; do not describe unfinished desktop as done.
- Login-state web path stays USER Edge/Chrome + `~/.vcu/lens-extension`. Host vision (Grok) is enough; do not require `vcu init model`.
- DOM extract/click/type/scroll must be `source=extension_dom` when using the extension. AX chrome is not HTML DOM.
- Never click Edge Allow debugging. Never automate WeChat. Never warp the OS cursor. Local `/reference/` (gitignored) may be reverse-engineered for architecture notes. Do not commit the .app, decompiled sources, or extracted official assets. Do not edit the live `~/.codex/computer-use/` install. Reimplement in VCU with original code under `assets/`.
- Browser extension stays unpacked in the user profile. Do not publish it to the Chrome Web Store or Edge Add-ons.
- Prefer editing authoritative Markdown/YAML; reindex with `awr source reindex`.
- Preserve unrelated edits; keep commits focused.

## Session continuity

On a new session, read **`docs/PLAN.md`**, then **`docs/ROADMAP-CU.md`** if doing desktop work, then **`docs/HANDOFF.md`**.
