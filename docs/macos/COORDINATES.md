# Screenshot pixels ↔ AX points

Retina: Accessibility frames are **points**; `screencapture` PNG is often 2× pixels.

VCU reports:

- `screenshot_scale` / `webview_screenshot_scale` ∈ {1, 2, 3}
- `screenshot_ref` / `webview_screenshot_ref`

Map a pixel in that PNG back to an AX point:

```
ax_x = frame[0] + pixel_x / scale
ax_y = frame[1] + pixel_y / scale
```

`frame` is `[x, y, w, h]` in points, origin top-left of the primary display, y down.

Desktop `click` accepts either:

- `target.ref` — AXPress / webview `ax_frame_hit`
- `args.pixel_x` + `args.pixel_y` + optional `args.space` = `window` | `webview`

Guide moves to the AX point. OS cursor is never warped.

Host vision models (Grok, etc.) should use these fields; no extra VCU vision model is required.

Login-state Edge/Chrome: Scene prefers the `AXWebArea` frame (reported as `webview_*` when present). Map pixels from `webview_screenshot_*` with `space=webview`. The address-bar URL is `page_url` in observe / `login-latest.json` — not a CDP URL.

## Login-state click without HUD

`vcu browser click` maps pixels from `login-latest.json`:

- `space=window` → `screenshot_frame` + `screenshot_scale`
- `space=webview` → `webview_screenshot_frame` + `webview_screenshot_scale`

`--dry-run` does not AXPress. Live press uses `press_at_point` (no Stage HUD, no OS cursor).
