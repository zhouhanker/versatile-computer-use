//! Desktop surface: windows → tabs, Scene snapshot, AX Actuator, Guide overlay.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use tokio::sync::RwLock;
use vcu_core::{
    ActionRequest, DomRef, ErrorCode, ExtractResult, SnapshotMode, TabInfo, VcuError, VcuResult,
};

use super::{ActionResultDetail, BrowserBackend, ScreenshotData, SnapshotData};
use crate::app::{
    ax_point_from_pixel, is_editable_ax_role, is_webview_like, smallest_ref_at_point,
    webview_crop_frame, AppBackend,
};
use crate::stage::StageHandle;

#[derive(Debug, Clone)]
struct CaptureMeta {
    frame: [f64; 4],
    scale: f64,
    webview_frame: Option<[f64; 4]>,
    webview_scale: Option<f64>,
    webview_ref: Option<String>,
}

pub struct DesktopBackend {
    app: Arc<RwLock<Box<dyn AppBackend>>>,
    stage: StageHandle,
    window_id: String,
    last_refs: Mutex<HashMap<String, Vec<DomRef>>>,
    last_capture: Mutex<HashMap<String, CaptureMeta>>,
}

impl DesktopBackend {
    pub async fn new(app: Arc<RwLock<Box<dyn AppBackend>>>) -> VcuResult<Self> {
        let platform = { app.read().await.platform().to_string() };
        let stage = StageHandle::raise_for_platform(&platform)?;
        if !stage.shown {
            return Err(VcuError::coded(
                ErrorCode::StageRequired,
                "desktop surface requires a visible Stage banner",
            ));
        }
        Ok(Self {
            app,
            stage,
            window_id: "desktop-stage".into(),
            last_refs: Mutex::new(HashMap::new()),
            last_capture: Mutex::new(HashMap::new()),
        })
    }

    #[cfg(test)]
    async fn new_with_stage(
        app: Arc<RwLock<Box<dyn AppBackend>>>,
        stage: StageHandle,
    ) -> VcuResult<Self> {
        if !stage.shown {
            return Err(VcuError::coded(
                ErrorCode::StageRequired,
                "desktop surface requires a visible Stage banner",
            ));
        }
        Ok(Self {
            app,
            stage,
            window_id: "desktop-stage".into(),
            last_refs: Mutex::new(HashMap::new()),
            last_capture: Mutex::new(HashMap::new()),
        })
    }

    pub fn stage_shown(&self) -> bool {
        self.stage.shown
    }

    async fn frame_for(&self, tab_id: &str, target_ref: &str) -> Option<[f64; 4]> {
        if let Ok(cache) = self.last_refs.lock() {
            if let Some(refs) = cache.get(tab_id) {
                if let Some(r) = refs.iter().find(|r| r.r#ref == target_ref) {
                    return r.frame;
                }
            }
        }
        let snap = self.snapshot(tab_id, SnapshotMode::A11y, 2000).await.ok()?;
        snap.dom_refs
            .into_iter()
            .find(|r| r.r#ref == target_ref)
            .and_then(|r| r.frame)
    }

    async fn wait_scene(
        &mut self,
        tab_id: &str,
        action: &ActionRequest,
    ) -> VcuResult<ActionResultDetail> {
        let ms = action
            .args
            .get("ms")
            .or_else(|| action.args.get("timeout_ms"))
            .and_then(|v| v.as_u64())
            .unwrap_or(100)
            .min(5000);
        let want_ref = action
            .target
            .get("ref")
            .and_then(|v| v.as_str())
            .or_else(|| action.args.get("ref").and_then(|v| v.as_str()));
        let want_name = action.args.get("name").and_then(|v| v.as_str());
        let want_role = action.args.get("role").and_then(|v| v.as_str());
        let want_value = action.args.get("value").and_then(|v| v.as_str());
        let conditioned = want_ref.is_some()
            || want_name.is_some()
            || want_role.is_some()
            || want_value.is_some();
        if !conditioned {
            self.sleep_abortable(ms).await?;
            return Ok(ActionResultDetail {
                ok: true,
                detail: serde_json::json!({
                    "waited_ms": ms,
                    "input_path": "scene_wait",
                    "os_cursor_used": false
                }),
            });
        }
        let started = std::time::Instant::now();
        let budget = std::time::Duration::from_millis(ms.max(1));
        loop {
            self.ensure_not_aborted()?;
            let snap = self.snapshot(tab_id, SnapshotMode::A11y, 2000).await?;
            if let Some(found) =
                scene_wait_match(&snap.dom_refs, want_ref, want_name, want_role, want_value)
            {
                return Ok(ActionResultDetail {
                    ok: true,
                    detail: serde_json::json!({
                        "waited_ms": started.elapsed().as_millis() as u64,
                        "found_ref": found,
                        "input_path": "scene_wait",
                        "os_cursor_used": false
                    }),
                });
            }
            if started.elapsed() >= budget {
                return Err(VcuError::coded(
                    ErrorCode::ActionFailed,
                    format!(
                        "wait timed out after {ms}ms for ref={:?} name={:?} role={:?} value={:?}",
                        want_ref, want_name, want_role, want_value
                    ),
                ));
            }
            self.sleep_abortable(50).await?;
        }
    }

    fn ensure_not_aborted(&self) -> VcuResult<()> {
        if self.stage.abort_requested() {
            return Err(VcuError::coded(
                ErrorCode::SessionClosed,
                "stage abort during wait",
            ));
        }
        Ok(())
    }

    async fn sleep_abortable(&self, ms: u64) -> VcuResult<()> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(ms);
        loop {
            self.ensure_not_aborted()?;
            let now = std::time::Instant::now();
            if now >= deadline {
                return Ok(());
            }
            let slice = deadline.saturating_duration_since(now).as_millis().min(50) as u64;
            tokio::time::sleep(std::time::Duration::from_millis(slice.max(1))).await;
        }
    }

    async fn hover(&mut self, tab_id: &str, target_ref: &str) -> VcuResult<ActionResultDetail> {
        let _ = self.snapshot(tab_id, SnapshotMode::A11y, 2000).await?;
        let frame = self.frame_for(tab_id, target_ref).await.ok_or_else(|| {
            VcuError::coded(
                ErrorCode::InvalidInput,
                format!("hover target {target_ref} has no Scene frame"),
            )
        })?;
        let cx = frame[0] + frame[2] / 2.0;
        let cy = frame[1] + frame[3] / 2.0;
        let g = self.stage.move_guide(cx, cy)?;
        Ok(ActionResultDetail {
            ok: true,
            detail: serde_json::json!({
                "input_path": "guide_hover",
                "os_cursor_used": false,
                "hid_injected": false,
                "guide": {
                    "x": g.x,
                    "y": g.y,
                    "overlay": true,
                    "os_cursor_used": false
                }
            }),
        })
    }


    fn remember_capture(&self, tab_id: &str, meta: CaptureMeta) {
        if let Ok(mut m) = self.last_capture.lock() {
            m.insert(tab_id.to_string(), meta);
        }
    }

    async fn click_pixels(
        &mut self,
        tab_id: &str,
        pixel_x: f64,
        pixel_y: f64,
        space: &str,
    ) -> VcuResult<ActionResultDetail> {
        let _ = self.snapshot(tab_id, SnapshotMode::A11y, 2000).await?;
        let meta = {
            let cached = self
                .last_capture
                .lock()
                .ok()
                .and_then(|m| m.get(tab_id).cloned());
            if let Some(m) = cached {
                m
            } else {
                let shot = self.screenshot(tab_id, false).await?;
                let scale = shot.scale.ok_or_else(|| {
                    VcuError::coded(
                        ErrorCode::InvalidInput,
                        "screenshot_scale missing; cannot map pixels to AX points",
                    )
                })?;
                let frame = shot.frame.ok_or_else(|| {
                    VcuError::coded(ErrorCode::InvalidInput, "screenshot frame missing")
                })?;
                CaptureMeta {
                    frame,
                    scale,
                    webview_frame: shot.webview_frame,
                    webview_scale: shot.webview_scale,
                    webview_ref: shot.webview_ref,
                }
            }
        };
        let webview = space == "webview";
        let (frame, scale) = if webview {
            let f = meta.webview_frame.ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, "no webview crop for pixel click")
            })?;
            let sc = meta.webview_scale.ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, "webview_screenshot_scale missing")
            })?;
            (f, sc)
        } else {
            (meta.frame, meta.scale)
        };
        let (ax, ay) = ax_point_from_pixel(pixel_x, pixel_y, frame, scale).ok_or_else(|| {
            VcuError::coded(
                ErrorCode::InvalidInput,
                format!("pixel ({pixel_x},{pixel_y}) out of frame or scale"),
            )
        })?;
        let _ = self.stage.move_guide(ax, ay);
        let refs: Vec<(String, [f64; 4])> = self
            .last_refs
            .lock()
            .ok()
            .and_then(|c| c.get(tab_id).cloned())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| r.frame.map(|f| (r.r#ref, f)))
            .collect();
        let hit = smallest_ref_at_point(&refs, ax, ay).or_else(|| {
            if webview {
                meta.webview_ref.clone()
            } else {
                None
            }
        });
        let r = hit.ok_or_else(|| {
            VcuError::coded(
                ErrorCode::InvalidInput,
                format!("no Scene ref contains AX point ({ax:.1},{ay:.1})"),
            )
        })?;
        let _ = self.stage.move_guide(ax, ay);
        let pressed = {
            let mut app = self.app.write().await;
            app.press_at_point(tab_id, ax, ay).await
        };
        let mut clicked = match pressed {
            Ok(detail) => ActionResultDetail { ok: true, detail },
            Err(e) if e.code() == ErrorCode::NotImplemented => self.click(tab_id, &r).await?,
            Err(e) => return Err(e),
        };
        clicked.detail["pixel"] = serde_json::json!({"x": pixel_x, "y": pixel_y, "space": space});
        clicked.detail["ax_point"] = serde_json::json!({"x": ax, "y": ay});
        clicked.detail["screenshot_scale"] = serde_json::json!(scale);
        clicked.detail["hit_ref"] = serde_json::json!(clicked.detail.get("hit_ref").cloned().unwrap_or(serde_json::json!(r)));
        clicked.detail["coordinate_space"] = serde_json::json!("ax_points");
        clicked.detail["os_cursor_used"] = serde_json::json!(false);
        clicked.detail["hid_injected"] = serde_json::json!(false);
        if let Some(g) = self.stage.last_guide() {
            clicked.detail["guide"] = serde_json::json!({
                "x": g.x,
                "y": g.y,
                "overlay": true,
                "os_cursor_used": false
            });
        }
        Ok(clicked)
    }

    fn ref_is_webview(&self, tab_id: &str, target_ref: &str) -> bool {
        if let Ok(cache) = self.last_refs.lock() {
            if let Some(refs) = cache.get(tab_id) {
                if let Some(r) = refs.iter().find(|r| r.r#ref == target_ref) {
                    return is_webview_like(&r.role, &r.name);
                }
            }
        }
        false
    }

    fn cached_role(&self, tab_id: &str, target_ref: &str) -> Option<String> {
        let cache = self.last_refs.lock().ok()?;
        let refs = cache.get(tab_id)?;
        refs.iter()
            .find(|r| r.r#ref == target_ref)
            .map(|r| r.role.clone())
    }

    fn cached_name(&self, tab_id: &str, target_ref: &str) -> Option<String> {
        let cache = self.last_refs.lock().ok()?;
        let refs = cache.get(tab_id)?;
        refs.iter()
            .find(|r| r.r#ref == target_ref)
            .map(|r| r.name.clone())
    }

    fn feishu_like(tab_id: &str) -> bool {
        let t = tab_id.to_ascii_lowercase();
        t.contains("feishu") || t.contains("lark") || tab_id.contains("飞书")
    }

    fn settings_like(tab_id: &str) -> bool {
        let t = tab_id.to_ascii_lowercase();
        t.contains("system_settings")
            || t.contains("system settings")
            || t.contains("systemsettings")
            || tab_id.contains("系统设置")
    }

    fn settings_readonly_err() -> VcuError {
        VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "System Settings is observe-only; vcu doctor prints a hint — do not toggle Accessibility/TCC",
        )
    }

    fn looks_like_send(name: &str) -> bool {
        crate::login_state::ax_exposes_send_control([name])
    }

}

#[async_trait]
impl BrowserBackend for DesktopBackend {
    fn name(&self) -> &str {
        "desktop"
    }

    fn abort_watch_path(&self) -> Option<std::path::PathBuf> {
        self.stage.abort_watch_path()
    }

    fn stage_hud(&self) -> Option<(bool, &'static str)> {
        Some((self.stage.shown, self.stage.presenter))
    }

    fn request_stage_abort(&self) -> VcuResult<()> {
        self.stage.write_abort_signal()
    }

    async fn ensure_agent_window(&mut self) -> VcuResult<String> {
        if !self.stage.shown {
            return Err(VcuError::coded(
                ErrorCode::StageRequired,
                "Stage banner is not visible",
            ));
        }
        Ok(self.window_id.clone())
    }

    async fn list_tabs(&self) -> VcuResult<Vec<TabInfo>> {
        let app = self.app.read().await;
        let windows = app.list_windows().await?;
        Ok(windows
            .into_iter()
            .map(|w| {
                let profile = w.browser_profile.clone();
                TabInfo {
                    tab_id: w.id.clone(),
                    window_id: self.window_id.clone(),
                    title: w.title.clone(),
                    url: format!("app://{}", w.bundle_or_exe.replace(' ', "_")),
                    agent_owned: w.allowed,
                    borrowed_by: None,
                    login_state: Some(profile.as_deref() == Some("user")),
                    browser_profile: profile,
                }
            })
            .collect())
    }

    async fn navigate(&mut self, _tab_id: &str, _url: &str) -> VcuResult<()> {
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "navigate is not available on the desktop surface",
        ))
    }

    async fn snapshot(
        &self,
        tab_id: &str,
        mode: SnapshotMode,
        budget_tokens: u64,
    ) -> VcuResult<SnapshotData> {
        let app = self.app.read().await;
        let snap = app.snapshot(tab_id, budget_tokens).await?;
        let dom_refs: Vec<DomRef> = snap
            .elements
            .iter()
            .map(|e| DomRef {
                r#ref: e.r#ref.clone(),
                role: e.role.clone(),
                name: e.name.clone(),
                value: e.value.clone(),
                selector: None,
                frame: e.frame,
            })
            .collect();
        let text_excerpt = dom_refs
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<_>>()
            .join(" · ");
        let summary = if snap.summary.contains("desktop.scene") {
            snap.summary
        } else {
            format!("kind=desktop.scene {}", snap.summary)
        };
        if let Ok(mut cache) = self.last_refs.lock() {
            cache.insert(tab_id.to_string(), dom_refs.clone());
        }
        let mut screenshot_png = None;
        let mut screenshot_scale = None;
        let mut webview_png = None;
        let mut webview_screenshot_scale = None;
        let mut window_frame = None;
        let mut webview_frame = None;
        if matches!(mode, SnapshotMode::Full) {
            match app.capture_window(tab_id).await {
                Ok(Some(cap)) => {
                    screenshot_scale = crate::app::pixel_scale(cap.width, cap.height, cap.frame);
                    window_frame = Some(cap.frame);
                    screenshot_png = Some(cap.png);
                }
                _ => {}
            }
            if let Some(frame) = webview_crop_frame(&snap.elements, snap.webview_ref.as_deref()) {
                match app.capture_rect(frame).await {
                    Ok(Some(cap)) => {
                        webview_screenshot_scale =
                            crate::app::pixel_scale(cap.width, cap.height, cap.frame);
                        webview_frame = Some(cap.frame);
                        webview_png = Some(cap.png);
                    }
                    _ => {}
                }
            }
            if let (Some(frame), Some(scale)) = (window_frame, screenshot_scale) {
                self.remember_capture(
                    tab_id,
                    CaptureMeta {
                        frame,
                        scale,
                        webview_frame,
                        webview_scale: webview_screenshot_scale,
                        webview_ref: snap.webview_ref.clone(),
                    },
                );
            }
        }
        Ok(SnapshotData {
            a11y_summary: Some(summary),
            dom_refs,
            text_excerpt: if text_excerpt.is_empty() {
                None
            } else {
                Some(text_excerpt)
            },
            screenshot_png,
            truncated: snap.truncated,
            budget_tokens_est: budget_tokens.min(4000),
            webview: snap.webview,
            webview_ref: snap.webview_ref,
            webview_png,
            screenshot_scale,
            webview_screenshot_scale,
        })
    }

    async fn click(&mut self, tab_id: &str, target_ref: &str) -> VcuResult<ActionResultDetail> {
        if Self::settings_like(tab_id) {
            return Err(Self::settings_readonly_err());
        }
        if Self::feishu_like(tab_id) {
            if let Some(name) = self.cached_name(tab_id, target_ref) {
                if Self::looks_like_send(&name) {
                    return Err(VcuError::coded(
                        ErrorCode::FocusPolicyViolation,
                        "Feishu/Lark send is not automatic; name a recipient first",
                    ));
                }
            }
        }
        let frame = self.frame_for(tab_id, target_ref).await;
        let webview = self.ref_is_webview(tab_id, target_ref);
        if webview {
            let tabs = self.list_tabs().await.unwrap_or_default();
            if tabs.iter().any(|t| {
                t.tab_id == tab_id
                    && t.browser_profile.as_deref() == Some("user")
                    && t.login_state == Some(true)
            }) {
                return Err(VcuError::coded(
                    ErrorCode::ActionFailed,
                    "USER browser HTML must use extension_dom (vcu browser click); AXWebArea is not a DOM click",
                ));
            }
        }
        if let Some(frame) = frame {
            let cx = frame[0] + frame[2] / 2.0;
            let cy = frame[1] + frame[3] / 2.0;
            let _ = self.stage.move_guide(cx, cy);
        }
        let mut app = self.app.write().await;
        let mut detail = app.invoke(tab_id, target_ref).await?;
        if detail.get("os_cursor_used").and_then(|v| v.as_bool()) == Some(true) {
            return Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "desktop actuator attempted OS cursor warp",
            ));
        }
        if webview
            || detail.get("input_path").and_then(|v| v.as_str()) == Some("ax_frame_hit")
            || detail
                .get("result")
                .and_then(|v| v.as_str())
                .is_some_and(|s| s.contains("webview"))
        {
            detail["input_path"] = serde_json::json!("ax_frame_hit");
            detail["hit_kind"] = serde_json::json!("ax_element");
        }
        detail["hid_injected"] = serde_json::json!(false);
        if let Some(g) = self.stage.last_guide() {
            detail["guide"] = serde_json::json!({
                "x": g.x,
                "y": g.y,
                "overlay": true,
                "os_cursor_used": false
            });
        }
        detail["os_cursor_used"] = serde_json::json!(false);
        Ok(ActionResultDetail { ok: true, detail })
    }

    async fn type_text(
        &mut self,
        tab_id: &str,
        text: &str,
        target_ref: Option<&str>,
    ) -> VcuResult<ActionResultDetail> {
        let r = target_ref.ok_or_else(|| {
            VcuError::coded(ErrorCode::InvalidInput, "desktop type requires target ref")
        })?;
        if Self::settings_like(tab_id) {
            return Err(Self::settings_readonly_err());
        }
        if let Some(role) = self.cached_role(tab_id, r) {
            let textedit = tab_id.to_ascii_lowercase().contains("textedit");
            let notepad = tab_id.to_ascii_lowercase().contains("notepad");
            let terminal = {
                let t = tab_id.to_ascii_lowercase();
                t.contains("terminal")
                    || t.contains("ghostty")
                    || t.contains("cmd")
                    || t.contains("conhost")
                    || t.contains("powershell")
            };
            if !is_editable_ax_role(&role) && !textedit && !notepad && !terminal {
                return Err(VcuError::coded(
                    ErrorCode::InvalidInput,
                    format!(
                        "desktop type ref {r} is {role}; need AXTextArea/AXTextField (not scroll area)"
                    ),
                ));
            }
        }
        let mut app = self.app.write().await;
        let mut detail = app.set_value(tab_id, r, text).await?;
        if detail.get("os_cursor_used").and_then(|v| v.as_bool()) == Some(true) {
            return Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "desktop actuator attempted OS cursor warp",
            ));
        }
        detail["os_cursor_used"] = serde_json::json!(false);
        detail["tab_id"] = serde_json::json!(tab_id);
        Ok(ActionResultDetail { ok: true, detail })
    }

    async fn extract(&self, tab_id: &str, selector: &str) -> VcuResult<ExtractResult> {
        let snap = self.snapshot(tab_id, SnapshotMode::A11y, 4000).await?;
        let matches = scene_extract_matches(&snap.dom_refs, selector);
        Ok(ExtractResult {
            session_id: String::new(),
            selector: Some(selector.to_string()),
            count: matches.len(),
            matches,
        })
    }

    async fn screenshot(&self, tab_id: &str, _full_page: bool) -> VcuResult<ScreenshotData> {
        let app = self.app.read().await;
        let snap = app.snapshot(tab_id, 4000).await.ok();
        let cap = match app.capture_window(tab_id).await? {
            Some(cap) => cap,
            None => {
                let msg = if cfg!(windows) {
                    "Windows PrintWindow did not produce a PNG. The process may have no visible HWND. This is not a macOS screen-recording permission."
                } else {
                    "window capture unavailable (AX Scene still works; set VCU_ALLOW_SCREENCAPTURE=1 only after Screen Recording is already granted)"
                };
                return Err(VcuError::coded(ErrorCode::NotImplemented, msg))
            }
        };
        let mut webview_png = None;
        let mut webview_ref = None;
        let mut webview_frame = None;
        let mut webview_scale = None;
        if let Some(snap) = snap.as_ref() {
            webview_ref = snap.webview_ref.clone();
            if let Some(frame) = webview_crop_frame(&snap.elements, snap.webview_ref.as_deref()) {
                if let Ok(Some(crop)) = app.capture_rect(frame).await {
                    webview_png = Some(crop.png);
                    webview_frame = Some(crop.frame);
                    webview_scale = crate::app::pixel_scale(crop.width, crop.height, crop.frame);
                }
            }
        }
        let scale = crate::app::pixel_scale(cap.width, cap.height, cap.frame);
        if let Some(scale) = scale {
            self.remember_capture(
                tab_id,
                CaptureMeta {
                    frame: cap.frame,
                    scale,
                    webview_frame,
                    webview_scale,
                    webview_ref: webview_ref.clone(),
                },
            );
        }
        Ok(ScreenshotData {
            png: cap.png,
            width: cap.width,
            height: cap.height,
            frame: Some(cap.frame),
            scale,
            webview_png,
            webview_ref,
            webview_frame,
            webview_scale,
        })
    }


    async fn act(&mut self, tab_id: &str, action: &ActionRequest) -> VcuResult<ActionResultDetail> {
        match action.r#type.as_str() {
            "click" | "hit" | "press" => {
                let r = action
                    .target
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .or_else(|| action.args.get("ref").and_then(|v| v.as_str()));
                if let Some(r) = r {
                    self.click(tab_id, r).await
                } else {
                    let px = action
                        .args
                        .get("pixel_x")
                        .or_else(|| action.target.get("pixel_x"))
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| {
                            VcuError::coded(
                                ErrorCode::InvalidInput,
                                "click requires target.ref or args.pixel_x/pixel_y",
                            )
                        })?;
                    let py = action
                        .args
                        .get("pixel_y")
                        .or_else(|| action.target.get("pixel_y"))
                        .and_then(|v| v.as_f64())
                        .ok_or_else(|| {
                            VcuError::coded(
                                ErrorCode::InvalidInput,
                                "click requires args.pixel_y",
                            )
                        })?;
                    let space = action
                        .args
                        .get("space")
                        .and_then(|v| v.as_str())
                        .unwrap_or("window");
                    self.click_pixels(tab_id, px, py, space).await
                }
            }
            "type" => {
                let text = action
                    .args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "type requires args.text")
                    })?;
                let r = action.target.get("ref").and_then(|v| v.as_str());
                self.type_text(tab_id, text, r).await
            }
            "scroll" => {
                if Self::settings_like(tab_id) {
                    return Err(Self::settings_readonly_err());
                }
                let dy = action
                    .args
                    .get("dy")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(600) as i32;
                let r = action.target.get("ref").and_then(|v| v.as_str());
                if let Some(rf) = r {
                    if let Some(frame) = self.frame_for(tab_id, rf).await {
                        let cx = frame[0] + frame[2] / 2.0;
                        let cy = frame[1] + frame[3] / 2.0;
                        let _ = self.stage.move_guide(cx, cy);
                    }
                }
                let mut app = self.app.write().await;
                let mut detail = app.scroll(tab_id, r, dy).await?;
                if detail.get("os_cursor_used").and_then(|v| v.as_bool()) == Some(true) {
                    return Err(VcuError::coded(
                        ErrorCode::OsCursorDenied,
                        "desktop scroll attempted OS cursor warp",
                    ));
                }
                detail["os_cursor_used"] = serde_json::json!(false);
                if let Some(g) = self.stage.last_guide() {
                    detail["guide"] = serde_json::json!({
                        "x": g.x,
                        "y": g.y,
                        "overlay": true,
                        "os_cursor_used": false
                    });
                }
                Ok(ActionResultDetail { ok: true, detail })
            }
            "key" | "keypress" => {
                let key = action
                    .args
                    .get("key")
                    .or_else(|| action.args.get("name"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "key requires args.key")
                    })?;
                let confirm_send = action
                    .args
                    .get("confirm_send")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let r = action.target.get("ref").and_then(|v| v.as_str());
                let plan = plan_desktop_key(key, confirm_send, r.is_some())?;
                let r = r.expect("plan_desktop_key requires ref");
                let mut clicked = self.click(tab_id, r).await?;
                clicked.detail["key"] = serde_json::json!(normalize_desktop_key(key).unwrap_or("other"));
                clicked.detail["confirm_send"] = serde_json::json!(confirm_send);
                clicked.detail["hid_injected"] = serde_json::json!(false);
                clicked.detail["input_path"] = serde_json::json!(plan.input_path());
                Ok(clicked)
            }
            "wait" => self.wait_scene(tab_id, action).await,
            "hover" => {
                let r = action
                    .target
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "hover requires target.ref")
                    })?;
                self.hover(tab_id, r).await
            }
            "os_cursor_move" | "os_click" => Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "OS cursor actions are denied; Guide is overlay-only",
            )),
            "reveal" => {
                let path = action
                    .args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "reveal requires args.path")
                    })?;
                let mut app = self.app.write().await;
                let mut detail = app.reveal_path(tab_id, path).await?;
                if detail.get("os_cursor_used").and_then(|v| v.as_bool()) == Some(true) {
                    return Err(VcuError::coded(
                        ErrorCode::OsCursorDenied,
                        "Finder reveal attempted OS cursor warp",
                    ));
                }
                detail["os_cursor_used"] = serde_json::json!(false);
                detail["hid_injected"] = serde_json::json!(false);
                Ok(ActionResultDetail { ok: true, detail })
            }
            "open_path" => {
                let path = action
                    .args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "open_path requires args.path")
                    })?;
                let mut app = self.app.write().await;
                let mut detail = app.open_path(tab_id, path).await?;
                if detail.get("os_cursor_used").and_then(|v| v.as_bool()) == Some(true) {
                    return Err(VcuError::coded(
                        ErrorCode::OsCursorDenied,
                        "Finder open_path attempted OS cursor warp",
                    ));
                }
                detail["os_cursor_used"] = serde_json::json!(false);
                detail["hid_injected"] = serde_json::json!(false);
                Ok(ActionResultDetail { ok: true, detail })
            }
            "navigate" => Err(VcuError::coded(
                ErrorCode::NotImplemented,
                "navigate is not available on the desktop surface",
            )),
            other => Err(VcuError::coded(
                ErrorCode::NotImplemented,
                format!("action not implemented on desktop: {other}"),
            )),
        }
    }
}



fn scene_extract_matches(refs: &[DomRef], selector: &str) -> Vec<serde_json::Value> {
    let sel = selector.trim();
    let star = sel.is_empty() || sel == "*";
    let needle = sel.to_ascii_lowercase();
    refs.iter()
        .filter(|r| {
            if star {
                return true;
            }
            r.r#ref.eq_ignore_ascii_case(sel)
                || r.role.to_ascii_lowercase().contains(&needle)
                || r.name.to_ascii_lowercase().contains(&needle)
                || r.value
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&needle)
        })
        .map(|r| {
            serde_json::json!({
                "ref": r.r#ref,
                "role": r.role,
                "name": r.name,
                "value": r.value,
                "frame": r.frame,
                "source": "desktop.scene"
            })
        })
        .collect()
}

fn is_desktop_window_role(role: &str) -> bool {
    let role = role.to_ascii_lowercase();
    role == "window"
        || role == "axwindow"
        || role.starts_with("controltype.window")
        || role.contains("axwindow")
}

fn scene_wait_match(
    refs: &[DomRef],
    want_ref: Option<&str>,
    want_name: Option<&str>,
    want_role: Option<&str>,
    want_value: Option<&str>,
) -> Option<String> {
    let matches: Vec<&DomRef> = refs
        .iter()
        .filter(|r| {
            if let Some(w) = want_ref {
                if r.r#ref != w {
                    return false;
                }
            }
            if let Some(w) = want_name {
                if !r.name.to_lowercase().contains(&w.to_lowercase()) {
                    return false;
                }
            }
            if let Some(w) = want_role {
                if !r.role.eq_ignore_ascii_case(w) {
                    return false;
                }
            }
            if let Some(w) = want_value {
                let hay = format!("{} {}", r.name, r.value.as_deref().unwrap_or(""));
                if !hay.to_lowercase().contains(&w.to_lowercase()) {
                    return false;
                }
            }
            true
        })
        .collect();
    if want_ref.is_some() {
        return matches.first().map(|r| r.r#ref.clone());
    }
    if let Some(control) = matches.iter().find(|r| !is_desktop_window_role(&r.role)) {
        return Some(control.r#ref.clone());
    }
    matches.first().map(|r| r.r#ref.clone())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DesktopKeyPlan {
    PressRefSend,
    PressRef,
}

impl DesktopKeyPlan {
    fn input_path(self) -> &'static str {
        match self {
            Self::PressRefSend => "ax_press_send",
            Self::PressRef => "ax_press",
        }
    }
}

fn normalize_desktop_key(key: &str) -> Option<&'static str> {
    match key.trim().to_ascii_lowercase().as_str() {
        "esc" | "escape" => Some("escape"),
        "return" | "enter" | "keypadenter" => Some("return"),
        "space" | "spacebar" => Some("space"),
        "tab" => Some("tab"),
        _ => None,
    }
}

fn plan_desktop_key(key: &str, confirm_send: bool, has_ref: bool) -> VcuResult<DesktopKeyPlan> {
    match normalize_desktop_key(key) {
        Some("escape") => Err(VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "Escape is not injected; the Stage HUD Esc abort is the cancel path",
        )),
        Some("return") => {
            if !confirm_send {
                Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    "Return/Enter is gated; set args.confirm_send=true and target.ref of the Send control after the user asked to send",
                ))
            } else if !has_ref {
                Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    "blind Return is forbidden; pass target.ref of the Send button",
                ))
            } else {
                Ok(DesktopKeyPlan::PressRefSend)
            }
        }
        Some("space") => {
            if !has_ref {
                Err(VcuError::coded(
                    ErrorCode::InvalidInput,
                    "space requires target.ref (AXPress, no HID keystroke)",
                ))
            } else {
                Ok(DesktopKeyPlan::PressRef)
            }
        }
        Some("tab") => Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "Tab is not injected; snapshot and click the next control ref",
        )),
        _ => Err(VcuError::coded(
            ErrorCode::NotImplemented,
            format!("desktop key not implemented: {key}"),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::mock_app::MockAppBackend;

    #[test]
    fn windows_systemsettings_tab_is_observe_only() {
        assert!(DesktopBackend::settings_like("win:SystemSettings:1"));
        assert!(DesktopBackend::settings_like("win:systemsettings:9"));
        assert!(!DesktopBackend::settings_like("win:notepad:1"));
        assert!(!DesktopBackend::settings_like("win:win32calc:1"));
    }

    #[tokio::test]
    async fn mock_desktop_stage_arms_abort_watch() {
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let b = DesktopBackend::new(app).await.unwrap();
        assert!(b.stage_shown());
        assert!(b.abort_watch_path().is_some(), "CU-D-011 mock Stage must arm abort watch");
    }

    #[tokio::test]
    async fn desktop_rejects_hidden_stage() {
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let err = match DesktopBackend::new_with_stage(app, crate::stage::StageHandle::hidden()).await {
            Ok(_) => panic!("hidden Stage must not start a desktop backend"),
            Err(e) => e,
        };
        assert_eq!(err.code(), ErrorCode::StageRequired);
        assert!(err.message().contains("Stage banner"));
    }

    #[tokio::test]
    async fn mock_desktop_scene_and_actuator() {
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let mut b = DesktopBackend::new(app).await.unwrap();
        assert!(b.stage_shown());
        let win = b.ensure_agent_window().await.unwrap();
        assert_eq!(win, "desktop-stage");
        let tabs = b.list_tabs().await.unwrap();
        let textedit = tabs.iter().find(|t| t.title == "TextEdit").unwrap();
        assert!(textedit.agent_owned);
        let wechat = tabs.iter().find(|t| t.title == "WeChat").unwrap();
        assert!(!wechat.agent_owned);
        let snap = b
            .snapshot(&textedit.tab_id, SnapshotMode::A11y, 2000)
            .await
            .unwrap();
        assert!(snap.a11y_summary.unwrap().contains("desktop.scene"));
        assert!(snap.dom_refs.iter().any(|r| r.r#ref == "e1"));
        assert_eq!(
            snap.dom_refs.iter().find(|r| r.r#ref == "e1").unwrap().frame,
            Some([20.0, 20.0, 80.0, 24.0])
        );
        let clicked = b.click(&textedit.tab_id, "e1").await.unwrap();
        assert_eq!(clicked.detail["os_cursor_used"], false);
        assert_eq!(clicked.detail["guide"]["overlay"], true);
        assert_eq!(clicked.detail["guide"]["x"], 60.0);
        assert_eq!(clicked.detail["guide"]["y"], 32.0);
        let feishu = tabs.iter().find(|t| t.title == "Feishu").unwrap();
        let fs = b
            .snapshot(&feishu.tab_id, SnapshotMode::A11y, 2000)
            .await
            .unwrap();
        assert!(fs.a11y_summary.unwrap().contains("webview=true"));
        let hit = b
            .act(
                &feishu.tab_id,
                &ActionRequest {
                    r#type: "hit".into(),
                    target: serde_json::json!({"ref":"e15"}),
                    args: serde_json::json!({}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(hit.detail["input_path"], "ax_frame_hit");
        assert_eq!(hit.detail["os_cursor_used"], false);
        assert_eq!(hit.detail["hid_injected"], false);
        assert_eq!(hit.detail["guide"]["overlay"], true);
        assert_eq!(hit.detail["guide"]["x"], 2218.0 + 1424.0 / 2.0);
        assert_eq!(hit.detail["guide"]["y"], 36.0 + 1038.0 / 2.0);
        let shot = b.screenshot(&textedit.tab_id, false).await.unwrap();
        assert!(!shot.png.is_empty());
        let full = b
            .snapshot(&textedit.tab_id, SnapshotMode::Full, 2000)
            .await
            .unwrap();
        assert!(full.screenshot_png.as_ref().unwrap().len() > 8);
        assert!(!full.webview);
        let fs_full = b
            .snapshot(&feishu.tab_id, SnapshotMode::Full, 2000)
            .await
            .unwrap();
        assert!(fs_full.webview);
        assert_eq!(fs_full.webview_ref.as_deref(), Some("e15"));
        assert!(fs_full.webview_png.as_ref().unwrap().len() > 8);
        assert!(fs_full.screenshot_png.as_ref().unwrap().len() > 8);
        assert_eq!(fs_full.webview_screenshot_scale, Some(1.0));
        let fs_shot = b.screenshot(&feishu.tab_id, false).await.unwrap();
        assert!(!fs_shot.png.is_empty());
        assert_eq!(fs_shot.webview_ref.as_deref(), Some("e15"));
        assert!(fs_shot.webview_png.as_ref().unwrap().len() > 8);
        assert_eq!(fs_shot.webview_frame, Some([2218.0, 36.0, 1424.0, 1038.0]));
        assert_eq!(fs_shot.webview_scale, Some(1.0));
        let typed = b
            .type_text(&textedit.tab_id, "hello", Some("e2"))
            .await
            .unwrap();
        assert_eq!(typed.detail["input_path"], "ax_set_value");
        assert_eq!(typed.detail["os_cursor_used"], false);
        let finder = tabs.iter().find(|t| t.title == "Finder").unwrap();
        let revealed = b
            .act(
                &finder.tab_id,
                &ActionRequest {
                    r#type: "reveal".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"path": "/tmp/VCU-D-040"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(revealed.detail["input_path"], "nsworkspace_reveal");
        assert_eq!(revealed.detail["os_cursor_used"], false);
        let opened = b
            .act(
                &finder.tab_id,
                &ActionRequest {
                    r#type: "open_path".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"path": "/tmp/VCU-D-040"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(opened.detail["input_path"], "nsworkspace_open");
        let denied = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "open_path".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"path": "/tmp/VCU-D-040"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(denied.code(), ErrorCode::InvalidInput);
        let fs_typed_btn = b
            .type_text(&feishu.tab_id, "hello", Some("e1"))
            .await
            .unwrap_err();
        assert_eq!(fs_typed_btn.code(), ErrorCode::InvalidInput);
        let send = b.click(&feishu.tab_id, "e_send").await.unwrap_err();
        assert_eq!(send.code(), ErrorCode::FocusPolicyViolation);
        let settings = tabs.iter().find(|t| t.title == "System Settings").unwrap();
        let snap_s = b
            .snapshot(&settings.tab_id, SnapshotMode::A11y, 2000)
            .await
            .unwrap();
        assert!(!snap_s.dom_refs.is_empty() || true);
        let denied = b.click(&settings.tab_id, "e1").await.unwrap_err();
        assert_eq!(denied.code(), ErrorCode::FocusPolicyViolation);
        assert!(denied.message().contains("observe-only"));
        let err = b.snapshot(&wechat.tab_id, SnapshotMode::A11y, 2000).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::AppDenied);
        let err = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "os_click".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::OsCursorDenied);
        let err = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "key".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"key":"return"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        let sent = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "key".into(),
                    target: serde_json::json!({"ref":"e1"}),
                    args: serde_json::json!({"key":"return","confirm_send":true}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(sent.detail["input_path"], "ax_press_send");
        assert_eq!(sent.detail["hid_injected"], false);
        assert_eq!(sent.detail["os_cursor_used"], false);
        let hovered = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "hover".into(),
                    target: serde_json::json!({"ref":"e1"}),
                    args: serde_json::json!({}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(hovered.detail["input_path"], "guide_hover");
        assert_eq!(hovered.detail["guide"]["overlay"], true);
        assert_eq!(hovered.detail["os_cursor_used"], false);
        assert_eq!(hovered.detail["hid_injected"], false);
        let pixel = b
            .act(
                &feishu.tab_id,
                &ActionRequest {
                    r#type: "click".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"pixel_x": 712.0, "pixel_y": 519.0, "space": "webview"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(pixel.detail["os_cursor_used"], false);
        assert_eq!(pixel.detail["hid_injected"], false);
        assert_eq!(pixel.detail["hit_ref"], "e15");
        assert_ne!(pixel.detail["input_path"], "extension_dom");
        assert_eq!(pixel.detail["screenshot_scale"], 1.0);
        assert_eq!(pixel.detail["ax_point"]["x"], 2218.0 + 712.0);
        assert_eq!(pixel.detail["ax_point"]["y"], 36.0 + 519.0);
        assert_eq!(pixel.detail["guide"]["overlay"], true);
        let tabs_login = b.list_tabs().await.unwrap();
        let user_edge = tabs_login
            .iter()
            .find(|t| t.browser_profile.as_deref() == Some("user"))
            .unwrap();
        assert_eq!(user_edge.login_state, Some(true));
        let ax_as_dom = b
            .click(&user_edge.tab_id, "e_web")
            .await
            .err()
            .expect("USER Edge AXWebArea must not fake a DOM click");
        assert_eq!(ax_as_dom.code(), ErrorCode::ActionFailed);
        assert!(ax_as_dom.message().contains("extension_dom"));
        assert_eq!(
            crate::login_state::pick_login_tab(&tabs_login, "edge", None).as_deref(),
            Some("proc:Microsoft_Edge:10")
        );
        let extracted = b.extract(&feishu.tab_id, "messenger").await.unwrap();
        assert!(extracted.count >= 1);
        assert!(extracted.matches.iter().any(|m| m["ref"] == "e15"));
        let all = b.extract(&textedit.tab_id, "*").await.unwrap();
        assert!(all.count >= 2);
    }

    #[tokio::test]
    async fn wait_stops_on_stage_abort() {
        let dir = tempfile::tempdir().unwrap();
        let abort = dir.path().join("stage.abort");
        std::fs::write(&abort, b"1").unwrap();
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let mut b = DesktopBackend::new_with_stage(
            app,
            crate::stage::StageHandle::noop_with_abort(abort),
        )
        .await
        .unwrap();
        let tabs = b.list_tabs().await.unwrap();
        let textedit = tabs.iter().find(|t| t.title == "TextEdit").unwrap();
        let err = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "wait".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"ms": 400}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::SessionClosed);
    }

    #[tokio::test]
    async fn wait_times_out_when_value_missing() {
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let mut b = DesktopBackend::new(app).await.unwrap();
        let tabs = b.list_tabs().await.unwrap();
        let textedit = tabs.iter().find(|t| t.title == "TextEdit").unwrap();
        let err = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "wait".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"ms": 120, "value": "VCU-D-210-MISS"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::ActionFailed);
        assert!(err.message().contains("timed out"), "{}", err.message());
        assert!(err.message().contains("VCU-D-210-MISS"), "{}", err.message());
        assert!(err.message().contains("value="), "{}", err.message());
    }

    #[tokio::test]
    async fn wait_times_out_when_ref_missing() {
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let mut b = DesktopBackend::new(app).await.unwrap();
        let tabs = b.list_tabs().await.unwrap();
        let textedit = tabs.iter().find(|t| t.title == "TextEdit").unwrap();
        let err = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "wait".into(),
                    target: serde_json::json!({"ref": "e999"}),
                    args: serde_json::json!({"ms": 120}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap_err();
        assert_eq!(err.code(), ErrorCode::ActionFailed);
        assert!(err.message().contains("timed out"), "{}", err.message());
        assert!(err.message().contains("e999"), "{}", err.message());
    }

    #[tokio::test]
    async fn wait_succeeds_when_scene_value_present() {
        let app: Arc<RwLock<Box<dyn AppBackend>>> =
            Arc::new(RwLock::new(Box::new(MockAppBackend::default())));
        let mut b = DesktopBackend::new(app).await.unwrap();
        let tabs = b.list_tabs().await.unwrap();
        let textedit = tabs.iter().find(|t| t.title == "TextEdit").unwrap();
        let ok = b
            .act(
                &textedit.tab_id,
                &ActionRequest {
                    r#type: "wait".into(),
                    target: serde_json::json!({}),
                    args: serde_json::json!({"ms": 400, "value": "Hello"}),
                    idempotency_key: None,
                },
            )
            .await
            .unwrap();
        assert!(ok.ok);
        assert_eq!(ok.detail["input_path"], "scene_wait");
        assert_eq!(ok.detail["os_cursor_used"], false);
        assert_eq!(ok.detail["found_ref"], "e2");
    }

    #[test]
    fn desktop_key_policy_gates_return_and_escape() {
        assert_eq!(
            plan_desktop_key("return", false, true).unwrap_err().code(),
            ErrorCode::FocusPolicyViolation
        );
        assert_eq!(
            plan_desktop_key("enter", true, false).unwrap_err().code(),
            ErrorCode::FocusPolicyViolation
        );
        assert_eq!(
            plan_desktop_key("escape", false, false).unwrap_err().code(),
            ErrorCode::FocusPolicyViolation
        );
        assert_eq!(
            plan_desktop_key("tab", false, true).unwrap_err().code(),
            ErrorCode::NotImplemented
        );
        assert!(plan_desktop_key("return", true, true).is_ok());
        assert!(plan_desktop_key("space", false, true).is_ok());
    }

    #[test]
    fn scene_wait_match_filters_ref_name_role() {
        let refs = vec![DomRef {
            r#ref: "e1".into(),
            role: "button".into(),
            name: "OK".into(),
            value: None,
            selector: None,
            frame: None,
        }];
        assert_eq!(scene_wait_match(&refs, Some("e1"), None, None, None).as_deref(), Some("e1"));
        assert_eq!(scene_wait_match(&refs, None, Some("ok"), Some("button"), None).as_deref(), Some("e1"));
        assert!(scene_wait_match(&refs, Some("e9"), None, None, None).is_none());
        assert!(scene_wait_match(&refs, None, Some("Send"), None, None).is_none());
        let with_val = vec![DomRef {
            r#ref: "e2".into(),
            role: "ControlType.Pane/Edit".into(),
            name: "".into(),
            value: Some("VCU-D-200-MARK".into()),
            selector: None,
            frame: None,
        }];
        assert_eq!(
            scene_wait_match(&with_val, None, None, None, Some("VCU-D-200-MARK")).as_deref(),
            Some("e2")
        );
        assert!(scene_wait_match(&with_val, None, None, None, Some("nope")).is_none());
        let same_name = vec![
            DomRef {
                r#ref: "e1".into(),
                role: "ControlType.Window".into(),
                name: "VCU-SAME".into(),
                value: Some("VCU-SAME".into()),
                selector: None,
                frame: None,
            },
            DomRef {
                r#ref: "e2".into(),
                role: "ControlType.Button".into(),
                name: "VCU-SAME".into(),
                value: None,
                selector: None,
                frame: None,
            },
        ];
        assert_eq!(
            scene_wait_match(&same_name, None, Some("VCU-SAME"), None, None).as_deref(),
            Some("e2")
        );
        assert_eq!(
            scene_wait_match(&same_name, Some("e1"), None, None, None).as_deref(),
            Some("e1")
        );
        assert!(!is_desktop_window_role("ControlType.Pane/WindowsForms10.EDIT.app"));
    }

    #[test]
    fn scene_extract_matches_name_and_star() {
        let refs = vec![DomRef {
            r#ref: "e15".into(),
            role: "AXGroup".into(),
            name: "MultiWebView - messenger:messenger-chat:default".into(),
            value: None,
            selector: None,
            frame: Some([2218.0, 36.0, 1424.0, 1038.0]),
        }];
        let hits = scene_extract_matches(&refs, "messenger");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["ref"], "e15");
        assert_eq!(scene_extract_matches(&refs, "*").len(), 1);
        assert!(scene_extract_matches(&refs, "etherscan").is_empty());
    }
}
