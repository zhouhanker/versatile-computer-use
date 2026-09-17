# Reverse notes: Codex Computer Use & OriginOne computer-helper

**Policy: never uninstall or modify Codex Computer Use or OriginOne gpt-bridge installs.**

## 1. Codex Computer Use (`~/.codex/computer-use/`)

### Layout
- `Codex Computer Use.app` → binary `SkyComputerUseService` (Mach-O arm64)
- `config.json` — UI locale strings ("ChatGPT is using your computer", Esc to cancel)
- Bundle resources: `Package_ComputerUse.bundle` with:
  - **LensSequence/** PNG frames — animated “lens/cursor” overlay assets
  - **AppInstructions/*.md** — per-app operational tips (Slack, Notion, Spotify, …)
  - Skysight summarizer / memory instruction prompts (treat on-screen text as untrusted)

### Runtime signals (from binary symbols / class names)
| Concept | Evidence |
| --- | --- |
| Virtual / software cursor | `ComputerUseCursor`, `AgentCursor`, `FogCursorViewModel`, `SoftwareCursorStyle`, `CursorView` |
| Overlay UI | `RecordAndReplayOverlay*`, `LockScreenOverlayPresenter`, `OverlayWindowLevelReasons` |
| Accessibility | `FocusedUIElementAccessibilityInfo`, `AccessibilitySPI` |
| IPC | `ComputerUseIPCJSONRPCSocket*`, unix socket under Group Container `com.openai.sky.CUAService` |
| Click model | `ClickType`, cursor motion path measurement |

### Logic (inferred)
1. Host (ChatGPT/Codex) drives a **dedicated CU service process**, not a generic CLI.
2. Service presents a **visible overlay** (“using your computer”) and a **virtual cursor** layer (not necessarily moving the real OS cursor exclusively — fog/agent cursor models).
3. Observes UI via **Accessibility SPI** + screenshots; actions via structured click/type APIs.
4. App-specific markdown instructions reduce mis-clicks (e.g. Slack `set_value` vs `type_text` so Return does not send).
5. Summarizer path treats page/content as **untrusted** (prompt-injection hardened memory).

### Difference vs VCU
- Codex CU is **vendor-locked** to OpenAI host + local Sky service.
- VCU is **host/model agnostic** (CLI/MCP/daemon), browser-first CDP/extension, optional app AX.

## 2. OriginOne gpt-bridge “computer-helper”

Path: `~/Library/Application Support/ai.originone.gpt-bridge/`

| Dir/File | Role |
| --- | --- |
| `computer-helper/` | Empty placeholder dir in this install |
| `computer-capability/capability.json` | `"enabled": false` |
| `computer-policy/policy.json` | App access allow/deny by bundle id (WeChat allow, 1Password deny, System Settings ask) |
| `config.json` | Profiles, OpenAI tunnel MCP, generations/cursor handles |
| `macos-privacy/` | Privacy acknowledgment |
| `managed-tunnel/`, `credentials/` | Tunnel auth (not inspected beyond existence) |

### Logic (inferred)
- Bridge sits between ChatGPT web and local machine; computer-use is a **capability flag** currently disabled.
- Policy engine gates which apps an agent may touch (bundle-id ACL).
- Not the same binary as Codex SkyComputerUseService; do not remove.

## 3. Product implications for VCU
- Support **attach CDP** + **extension Agent Window** for browser takeover with login.
- Optional **virtual cursor overlay** is a future UX layer (Codex-like), not required for DOM actions.
- Keep **hard denylist** for WeChat in VCU tests/docs even if other tools allow it.
