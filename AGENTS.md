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

Authoritative sources:

- `.awr/intake/GOALS.md`
- `.awr/intake/work-ledger.yaml`
- `docs/research/*` and `docs/design/*` (design authority for scope decisions)

## Working rules

- Current phase is **research + design only** until the design package is accepted.
- Do not implement runtime/browser/extension product code before design acceptance is recorded in the ledger.
- Prefer editing authoritative Markdown/YAML sources; reindex with `awr source reindex`.
- Preserve unrelated edits; keep commits focused.
