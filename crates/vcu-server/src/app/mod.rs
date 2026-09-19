//! Desktop App computer-use adapters.
pub mod macos;
pub mod mock_app;
pub mod windows;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use vcu_core::{ErrorCode, VcuError, VcuResult};

/// Hard denylist — evaluated before allowlist. WeChat is never operable.
/// Map screenshot pixels back to AX point frames. None unless scale is ~1/2/3.
pub fn pixel_scale(width: u32, height: u32, frame: [f64; 4]) -> Option<f64> {
    let fw = frame[2];
    let fh = frame[3];
    if fw < 1.0 || fh < 1.0 || width == 0 || height == 0 {
        return None;
    }
    let sx = width as f64 / fw;
    let sy = height as f64 / fh;
    if (sx - sy).abs() > 0.2 {
        return None;
    }
    for cand in [1.0, 2.0, 3.0] {
        if (sx - cand).abs() < 0.2 {
            return Some(cand);
        }
    }
    None
}

/// Read PNG IHDR pixel size. Retina screencapture is often 2x the AX point frame.
pub fn png_ihdr_size(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 {
        return None;
    }
    if png[0..8] != [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a] {
        return None;
    }
    if &png[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(png[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(png[20..24].try_into().ok()?);
    if w == 0 || h == 0 {
        return None;
    }
    Some((w, h))
}

/// Map a screenshot pixel (origin top-left of `frame`) back to AX points.
/// `scale` must be 1, 2, or 3 (retina).
pub fn ax_point_from_pixel(
    pixel_x: f64,
    pixel_y: f64,
    frame: [f64; 4],
    scale: f64,
) -> Option<(f64, f64)> {
    if scale != 1.0 && scale != 2.0 && scale != 3.0 {
        return None;
    }
    if !pixel_x.is_finite() || !pixel_y.is_finite() || pixel_x < 0.0 || pixel_y < 0.0 {
        return None;
    }
    let pw = frame[2] * scale;
    let ph = frame[3] * scale;
    if pixel_x > pw + 0.51 || pixel_y > ph + 0.51 {
        return None;
    }
    Some((frame[0] + pixel_x / scale, frame[1] + pixel_y / scale))
}

/// Map screenshot pixels in `window` or `webview` space to AX points.
/// Typical Edge chrome above the page WebArea (points). Used only when AX
/// did not report a WebArea frame.
pub fn edge_webview_heuristic(window: [f64; 4]) -> [f64; 4] {
    let top = 165.0;
    let side = 4.0;
    let bottom = 4.0;
    [
        window[0] + side,
        window[1] + top,
        (window[2] - side * 2.0).max(2.0),
        (window[3] - top - bottom).max(2.0),
    ]
}

pub fn map_screenshot_pixel(
    pixel_x: f64,
    pixel_y: f64,
    space: &str,
    window_frame: Option<[f64; 4]>,
    window_scale: Option<f64>,
    webview_frame: Option<[f64; 4]>,
    webview_scale: Option<f64>,
) -> Option<(f64, f64, [f64; 4], f64)> {
    let webview = space.eq_ignore_ascii_case("webview");
    let (frame, scale) = if webview {
        (webview_frame?, webview_scale?)
    } else {
        (window_frame?, window_scale?)
    };
    let (ax, ay) = ax_point_from_pixel(pixel_x, pixel_y, frame, scale)?;
    Some((ax, ay, frame, scale))
}

pub fn json_frame(v: Option<&serde_json::Value>) -> Option<[f64; 4]> {
    let arr = v?.as_array()?;
    if arr.len() != 4 {
        return None;
    }
    Some([
        arr[0].as_f64()?,
        arr[1].as_f64()?,
        arr[2].as_f64()?,
        arr[3].as_f64()?,
    ])
}

/// Smallest-area AX frame that contains the point (points, y down).
pub fn smallest_ref_at_point(refs: &[(String, [f64; 4])], x: f64, y: f64) -> Option<String> {
    let mut best: Option<(f64, String)> = None;
    for (id, f) in refs {
        if x >= f[0] && y >= f[1] && x <= f[0] + f[2] && y <= f[1] + f[3] {
            let area = (f[2] * f[3]).max(1.0);
            match &best {
                None => best = Some((area, id.clone())),
                Some((a, _)) if area < *a => best = Some((area, id.clone())),
                _ => {}
            }
        }
    }
    best.map(|(_, id)| id)
}

pub fn is_denied_app(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("wechat")
        || lower.contains("weixin")
        || name.contains("微信")
}

pub fn denied_app_error(name: &str) -> VcuError {
    VcuError::coded(
        ErrorCode::AppDenied,
        format!("app '{name}' is denied by VCU policy"),
    )
}

/// Window rect is [x, y, w, h] in points, y down.
pub fn denied_app_covering_point(windows: &[(String, [f64; 4])], x: f64, y: f64) -> Option<String> {
    let mut hit: Option<(f64, String)> = None;
    for (name, f) in windows {
        if x >= f[0] && y >= f[1] && x <= f[0] + f[2] && y <= f[1] + f[3] {
            let area = (f[2] * f[3]).max(1.0);
            if is_denied_app(name) {
                return Some(name.clone());
            }
            match &hit {
                None => hit = Some((area, name.clone())),
                Some((a, _)) if area < *a => hit = Some((area, name.clone())),
                _ => {}
            }
        }
    }
    hit.and_then(|(a, n)| {
        let _ = a;
        if is_denied_app(&n) { Some(n) } else { None }
    })
}

pub fn hit_error_code(detail: &str) -> Option<ErrorCode> {
    let d = detail.to_ascii_lowercase();
    if d.contains("denied-app") || d.contains("wechat") && d.contains("error:") {
        Some(ErrorCode::AppDenied)
    } else if d.contains("other-pid") {
        Some(ErrorCode::ActionFailed)
    } else {
        None
    }
}

/// AXPress returns zero on success. A successful script process is not enough.
pub fn ax_position_press_succeeded(detail: &str) -> bool {
    detail.starts_with("ok:ax_position_press:pid:")
        && detail.rsplit_once(":axpress:").is_some_and(|(_, code)| code.trim() == "0")
}

/// AXPress by Scene ref. Bare "ok" or System Events `click` is not success.
pub fn ax_ref_press_succeeded(detail: &str) -> bool {
    let d = detail.trim();
    (d.starts_with("ok:ax_press:") || d.starts_with("ok-webview:ax_press:"))
        && d.rsplit_once(":axpress:").is_some_and(|(_, code)| code.trim() == "0")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTarget {
    pub id: String,
    pub title: String,
    pub bundle_or_exe: String,
    #[serde(default)]
    pub pid: Option<i32>,
    #[serde(default = "default_true")]
    pub allowed: bool,
    /// user | agent when this process is Chrome/Edge; none for ordinary apps.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub browser_profile: Option<String>,
}
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppElement {
    pub r#ref: String,
    pub role: String,
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frame: Option<[f64; 4]>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppTab {
    pub name: String,
    #[serde(default)]
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub target: AppTarget,
    pub summary: String,
    pub elements: Vec<AppElement>,
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window_frame: Option<[f64; 4]>,
    /// True when Scene saw an Electron/WebView region (compose UI often not in AX).
    #[serde(default)]
    pub webview: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub webview_ref: Option<String>,
    /// Window title (login-state browser / app).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_title: Option<String>,
    /// Address-bar AXValue when present (no CDP).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page_url: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tabs: Vec<AppTab>,
    /// Chromium/Electron AXEnhancedUserInterface was requested.
    #[serde(default)]
    pub ax_enhanced: bool,
}

pub fn is_webview_like(role: &str, name: &str) -> bool {
    let blob = format!("{role} {name}").to_ascii_lowercase();
    blob.contains("webview")
        || blob.contains("messenger")
        || blob.contains("webarea")
        || blob.contains("axwebarea")
}

/// Prefer the largest WebView/messenger frame so chat pane wins over the sidebar strip.
pub fn webview_hint(elements: &[AppElement]) -> (bool, Option<String>) {
    let mut best: Option<(f64, String)> = None;
    for e in elements {
        if !is_webview_like(&e.role, &e.name) {
            continue;
        }
        let area = e
            .frame
            .map(|f| (f[2] * f[3]).max(0.0))
            .unwrap_or(0.0);
        match &best {
            None => best = Some((area, e.r#ref.clone())),
            Some((a, _)) if area > *a => best = Some((area, e.r#ref.clone())),
            _ => {}
        }
    }
    match best {
        Some((_, r)) => (true, Some(r)),
        None => (false, None),
    }
}

/// Filter Scene elements by ref/role/name/value. `*` or empty = all.
#[derive(Debug, Clone, Default)]
pub struct LoginLatestMeta {
    pub scale: Option<f64>,
    pub frame: Option<serde_json::Value>,
    pub page_title: Option<String>,
    pub page_url: Option<String>,
    pub webview_scale: Option<f64>,
    pub webview_frame: Option<serde_json::Value>,
}

pub fn publish_latest_capture(
    id: &str,
    src: &std::path::Path,
    dir: &std::path::Path,
    meta: Option<&LoginLatestMeta>,
    stem: &str,
) -> Option<std::path::PathBuf> {
    let _ = std::fs::create_dir_all(dir);
    let dest = dir.join(format!("{stem}.png"));
    std::fs::copy(src, &dest).ok()?;
    let sidecar = dir.join(format!("{stem}.json"));
    let body = serde_json::json!({
        "app_id": id,
        "png": dest.display().to_string(),
        "screenshot_scale": meta.and_then(|m| m.scale),
        "screenshot_frame": meta.and_then(|m| m.frame.clone()),
        "page_title": meta.and_then(|m| m.page_title.clone()),
        "page_url": meta.and_then(|m| m.page_url.clone()),
        "webview_screenshot_scale": meta.and_then(|m| m.webview_scale),
        "webview_screenshot_frame": meta.and_then(|m| m.webview_frame.clone()),
        "coordinate_help": "ax = frame_origin + pixel / screenshot_scale"
    });
    let _ = std::fs::write(&sidecar, serde_json::to_vec_pretty(&body).unwrap_or_default());
    Some(dest)
}


/// Merge address-bar URL into an existing login-latest.json without rewriting the PNG.
pub fn merge_login_latest_page_url(dir: &std::path::Path, url: &str) -> bool {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return false;
    }
    let sidecar = dir.join("login-latest.json");
    let Ok(raw) = std::fs::read(&sidecar) else {
        return false;
    };
    let Ok(mut v) = serde_json::from_slice::<serde_json::Value>(&raw) else {
        return false;
    };
    v["page_url"] = serde_json::json!(url);
    std::fs::write(&sidecar, serde_json::to_vec_pretty(&v).unwrap_or_default()).is_ok()
}

/// Stable path for host-vision: login-latest.png + login-latest.json (scale/frame).
pub fn publish_login_latest(
    id: &str,
    src: &std::path::Path,
    dir: &std::path::Path,
    meta: Option<&LoginLatestMeta>,
) -> Option<std::path::PathBuf> {
    let id_l = id.to_ascii_lowercase();
    if !(id_l.contains("edge") || id_l.contains("chrome")) {
        return None;
    }
    publish_latest_capture(id, src, dir, meta, "login-latest")
}

pub fn publish_feishu_latest(
    id: &str,
    src: &std::path::Path,
    dir: &std::path::Path,
    meta: Option<&LoginLatestMeta>,
) -> Option<std::path::PathBuf> {
    let id_l = id.to_ascii_lowercase();
    if !(id_l.contains("feishu") || id_l.contains("lark") || id.contains("飞书")) {
        return None;
    }
    publish_latest_capture(id, src, dir, meta, "feishu-latest")
}

pub fn element_wait_match(
    elements: &[AppElement],
    want_ref: Option<&str>,
    want_name: Option<&str>,
    want_role: Option<&str>,
) -> Option<String> {
    elements.iter().find_map(|e| {
        if let Some(w) = want_ref {
            if e.r#ref != w {
                return None;
            }
        }
        if let Some(w) = want_name {
            if !e.name.to_lowercase().contains(&w.to_lowercase()) {
                return None;
            }
        }
        if let Some(w) = want_role {
            if !e.role.to_ascii_lowercase().contains(&w.to_ascii_lowercase()) {
                return None;
            }
        }
        Some(e.r#ref.clone())
    })
}

pub fn extract_elements(elements: &[AppElement], selector: &str) -> Vec<serde_json::Value> {
    let sel = selector.trim();
    let star = sel.is_empty() || sel == "*";
    let needle = sel.to_ascii_lowercase();
    elements
        .iter()
        .filter(|e| {
            if star {
                return true;
            }
            e.r#ref.eq_ignore_ascii_case(sel)
                || e.role.to_ascii_lowercase().contains(&needle)
                || e.name.to_ascii_lowercase().contains(&needle)
                || e.value
                    .as_deref()
                    .unwrap_or("")
                    .to_ascii_lowercase()
                    .contains(&needle)
        })
        .map(|e| {
            serde_json::json!({
                "ref": e.r#ref,
                "role": e.role,
                "name": e.name,
                "value": e.value,
                "frame": e.frame,
                "source": "app.scene"
            })
        })
        .collect()
}

pub fn webview_crop_frame(elements: &[AppElement], webview_ref: Option<&str>) -> Option<[f64; 4]> {
    let want = webview_ref?;
    let el = elements.iter().find(|e| e.r#ref == want)?;
    let frame = el.frame?;
    if frame[2] < 2.0 || frame[3] < 2.0 {
        None
    } else {
        Some(frame)
    }
}

#[derive(Debug, Clone)]
pub struct AppCapture {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub frame: [f64; 4],
}

#[async_trait]
pub trait AppBackend: Send + Sync {
    fn platform(&self) -> &str;
    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>>;
    /// Focus is explicit and opt-in; default backends may refuse.
    async fn focus_window(&mut self, id: &str, allow_focus_steal: bool) -> VcuResult<()>;
    async fn snapshot(&self, id: &str, budget: u64) -> VcuResult<AppSnapshot>;
    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value>;
    async fn set_value(&mut self, id: &str, element_ref: &str, value: &str) -> VcuResult<serde_json::Value>;
    async fn capture_window(&self, id: &str) -> VcuResult<Option<AppCapture>>;
    /// Capture a screen rect (AX frame). Default none. Never prompt for TCC.
    async fn capture_rect(&self, _frame: [f64; 4]) -> VcuResult<Option<AppCapture>> {
        Ok(None)
    }
    async fn scroll(
        &mut self,
        _id: &str,
        _element_ref: Option<&str>,
        _dy: i32,
    ) -> VcuResult<serde_json::Value> {
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "app scroll not implemented",
        ))
    }
    /// AX hit-test at a point (points, y down) then AXPress. Never warps the OS cursor.
    async fn press_at_point(
        &mut self,
        _id: &str,
        _x: f64,
        _y: f64,
    ) -> VcuResult<serde_json::Value> {
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "press_at_point not implemented",
        ))
    }
}

/// Placeholder used when platform adapter is unavailable.
pub struct UnsupportedAppBackend {
    pub platform: String,
}

#[async_trait]
impl AppBackend for UnsupportedAppBackend {
    fn platform(&self) -> &str {
        &self.platform
    }

    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>> {
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            format!("app computer-use adapter not enabled on {}", self.platform),
        ))
    }

    async fn focus_window(&mut self, _id: &str, _allow_focus_steal: bool) -> VcuResult<()> {
        Err(VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "app focus steal denied by default policy",
        ))
    }

    async fn snapshot(&self, _id: &str, _budget: u64) -> VcuResult<AppSnapshot> {
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "app a11y snapshot not implemented",
        ))
    }

    async fn invoke(&mut self, _id: &str, _element_ref: &str) -> VcuResult<serde_json::Value> {
        Err(VcuError::coded(
            ErrorCode::OsCursorDenied,
            "app invoke via OS input is denied unless a safe AX press path is available",
        ))
    }

    async fn set_value(&mut self, _id: &str, _element_ref: &str, _value: &str) -> VcuResult<serde_json::Value> {
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "app set_value not implemented",
        ))
    }

    async fn capture_window(&self, _id: &str) -> VcuResult<Option<AppCapture>> {
        Ok(None)
    }
}

pub fn detect_app_backend() -> Box<dyn AppBackend> {
    detect_app_backend_with_allowlist(None)
}

pub fn detect_app_backend_with_allowlist(allowlist: Option<Vec<String>>) -> Box<dyn AppBackend> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosAppBackend::with_allowlist(allowlist))
    }
    #[cfg(target_os = "windows")]
    {
        let _ = allowlist; Box::new(windows::WindowsAppBackend::new())
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let _ = allowlist;
        Box::new(UnsupportedAppBackend {
            platform: "other".into(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vcu_core::ErrorCode;

    #[test]
    fn ax_press_errors_are_not_successful_clicks() {
        assert!(ax_position_press_succeeded("ok:ax_position_press:pid:123:axpress:0"));
        for detail in ["ok:ax_position_press:pid:123:axpress:-25206", "ok:ax_position_press:pid:123:axpress:-25204", "", "ok", "error:other-pid:999:Finder"] {
            assert!(!ax_position_press_succeeded(detail), "{detail}");
        }
        assert!(ax_ref_press_succeeded("ok:ax_press:axpress:0"));
        assert!(ax_ref_press_succeeded("ok-webview:ax_press:axpress:0"));
        for detail in ["ok", "ok-click", "ok-webview", "ok-webview-click", "ok:ax_press:axpress:-25204", "not-found", "error:ax_press:fail"] {
            assert!(!ax_ref_press_succeeded(detail), "{detail}");
        }
    }

    #[tokio::test]
    async fn detect_returns_platform_backend() {
        let b = detect_app_backend();
        assert!(!b.platform().is_empty());
    }

    #[test]
    fn wechat_is_hard_denied() {
        assert!(is_denied_app("WeChat"));
        assert!(is_denied_app("微信"));
        assert!(is_denied_app("Weixin"));
        assert!(!is_denied_app("Microsoft Edge"));
        assert!(!is_denied_app("Feishu"));
        let wins = vec![
            ("Microsoft Edge".into(), [1728.0, 30.0, 1920.0, 1050.0]),
            ("WeChat".into(), [1900.0, 80.0, 1200.0, 800.0]),
        ];
        assert_eq!(denied_app_covering_point(&wins, 2000.0, 200.0).as_deref(), Some("WeChat"));
        assert_eq!(denied_app_covering_point(&wins, 1730.0, 40.0).as_deref(), None);
        assert_eq!(hit_error_code("error:denied-app:WeChat"), Some(ErrorCode::AppDenied));
        assert_eq!(hit_error_code("error:other-pid:12:Finder"), Some(ErrorCode::ActionFailed));
        assert_eq!(hit_error_code("ok:ax_position_press"), None);
    }

    #[test]
    fn webview_hint_picks_largest_messenger_pane() {
        let els = vec![
            AppElement {
                r#ref: "e1".into(),
                role: "AXButton".into(),
                name: "搜索（⌘＋K）".into(),
                value: None,
                frame: Some([0.0, 0.0, 163.0, 32.0]),
            },
            AppElement {
                r#ref: "e14".into(),
                role: "AXGroup".into(),
                name: "MultiWebView - messenger".into(),
                value: None,
                frame: Some([1907.0, 36.0, 305.0, 1038.0]),
            },
            AppElement {
                r#ref: "e15".into(),
                role: "AXGroup".into(),
                name: "MultiWebView - messenger:messenger-chat:default".into(),
                value: None,
                frame: Some([2218.0, 36.0, 1424.0, 1038.0]),
            },
        ];
        let (flag, r) = webview_hint(&els);
        assert!(flag);
        assert_eq!(r.as_deref(), Some("e15"));
        assert!(!is_webview_like("AXButton", "关闭按钮"));
        assert!(is_webview_like("AXWebArea", "bilibili"));
        assert_eq!(
            element_wait_match(&els, None, Some("messenger-chat"), None).as_deref(),
            Some("e15")
        );
        assert!(element_wait_match(&els, Some("e9"), None, None).is_none());
        assert_eq!(
            webview_crop_frame(&els, Some("e15")),
            Some([2218.0, 36.0, 1424.0, 1038.0])
        );
        assert!(webview_crop_frame(&els, Some("missing")).is_none());
        assert!(webview_crop_frame(&els, None).is_none());
        let hits = extract_elements(&els, "messenger-chat");
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["ref"], "e15");
        assert_eq!(extract_elements(&els, "*").len(), 3);
    }

    #[test]
    fn publish_login_latest_only_for_browsers() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src.png");
        std::fs::write(&src, b"png").unwrap();
        let out = dir.path().join("cap");
        let meta = LoginLatestMeta { scale: Some(2.0), frame: Some(serde_json::json!([0,0,100,100])), page_title: Some("t".into()), page_url: Some("https://example.com".into()), webview_scale: Some(2.0), webview_frame: Some(serde_json::json!([10,20,50,40])) };
        assert!(publish_login_latest("proc:Finder:1", &src, &out, Some(&meta)).is_none());
        let p = publish_login_latest("proc:Microsoft_Edge:10", &src, &out, Some(&meta)).unwrap();
        assert_eq!(p.file_name().unwrap(), "login-latest.png");
        assert_eq!(std::fs::read(&p).unwrap(), b"png");
        let side = serde_json::from_slice::<serde_json::Value>(&std::fs::read(out.join("login-latest.json")).unwrap()).unwrap();
        assert_eq!(side["screenshot_scale"], 2.0);
        assert_eq!(side["page_url"], "https://example.com");
        assert!(merge_login_latest_page_url(&out, "https://github.com/originoneai/agent-work-runtime/pulls"));
        let side2 = serde_json::from_slice::<serde_json::Value>(&std::fs::read(out.join("login-latest.json")).unwrap()).unwrap();
        assert_eq!(side2["page_url"], "https://github.com/originoneai/agent-work-runtime/pulls");
        assert!(!merge_login_latest_page_url(&out, "not-a-url"));
        let f = publish_feishu_latest("proc:Feishu:3", &src, &out, Some(&meta)).unwrap();
        assert_eq!(f.file_name().unwrap(), "feishu-latest.png");
    }

    #[test]
    fn png_ihdr_reads_minimal_png() {
        let png: &[u8] = &[
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44,
            0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f,
            0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60,
            0x00, 0x02, 0x00, 0x00, 0x05, 0x00, 0x01, 0xd5, 0xc8, 0x4d, 0x6e, 0x00, 0x00, 0x00, 0x00,
            0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        assert_eq!(png_ihdr_size(png), Some((1, 1)));
        assert!(png_ihdr_size(&png[..12]).is_none());
        assert!(png_ihdr_size(b"not a png").is_none());
    }

    #[test]
    fn pixel_scale_snaps_retina_and_rejects_junk() {
        assert_eq!(pixel_scale(2848, 2076, [2218.0, 36.0, 1424.0, 1038.0]), Some(2.0));
        assert_eq!(pixel_scale(1424, 1038, [2218.0, 36.0, 1424.0, 1038.0]), Some(1.0));
        assert!(pixel_scale(1, 1, [0.0, 0.0, 800.0, 600.0]).is_none());
        assert!(pixel_scale(200, 50, [0.0, 0.0, 100.0, 100.0]).is_none());
    }

    #[test]
    fn ax_point_from_pixel_retina_feishu_webview() {
        let frame = [2218.0, 36.0, 1424.0, 1038.0];
        assert_eq!(
            ax_point_from_pixel(1424.0, 1038.0, frame, 2.0),
            Some((2218.0 + 712.0, 36.0 + 519.0))
        );
        assert_eq!(ax_point_from_pixel(0.0, 0.0, frame, 2.0), Some((2218.0, 36.0)));
        assert!(ax_point_from_pixel(-1.0, 0.0, frame, 2.0).is_none());
        assert!(ax_point_from_pixel(10.0, 10.0, frame, 1.5).is_none());
        let refs = vec![
            ("e14".into(), [1907.0, 36.0, 305.0, 1038.0]),
            ("e15".into(), [2218.0, 36.0, 1424.0, 1038.0]),
        ];
        assert_eq!(
            smallest_ref_at_point(&refs, 2218.0 + 712.0, 36.0 + 519.0).as_deref(),
            Some("e15")
        );
        assert_eq!(
            smallest_ref_at_point(&refs, 1907.0 + 10.0, 36.0 + 10.0).as_deref(),
            Some("e14")
        );
        let mapped = map_screenshot_pixel(
            0.0,
            0.0,
            "webview",
            Some([1728.0, 30.0, 1920.0, 1050.0]),
            Some(2.0),
            Some([1732.0, 195.0, 1912.0, 881.0]),
            Some(2.0),
        )
        .unwrap();
        let h = edge_webview_heuristic([1728.0, 30.0, 1920.0, 1050.0]);
        assert!((h[1] - 195.0).abs() < 1.0);
        assert_eq!(mapped.0, 1732.0);
        assert_eq!(mapped.1, 195.0);
        assert_eq!(mapped.3, 2.0);
        let mid = map_screenshot_pixel(
            1912.0,
            881.0,
            "webview",
            Some([1728.0, 30.0, 1920.0, 1050.0]),
            Some(2.0),
            Some([1732.0, 195.0, 1912.0, 881.0]),
            Some(2.0),
        )
        .unwrap();
        assert_eq!(mid.0, 1732.0 + 956.0);
        assert_eq!(mid.1, 195.0 + 440.5);
        assert!(map_screenshot_pixel(0.0, 0.0, "webview", Some([0.0, 0.0, 10.0, 10.0]), Some(2.0), None, None).is_none());
    }

    #[test]
    fn map_window_and_oob() {
        let win = [0.0, 33.0, 1728.0, 960.0];
        let wv = [179.0, 39.0, 1549.0, 948.0];
        let w0 = map_screenshot_pixel(0.0, 0.0, "window", Some(win), Some(2.0), Some(wv), Some(2.0)).unwrap();
        let v0 = map_screenshot_pixel(0.0, 0.0, "webview", Some(win), Some(2.0), Some(wv), Some(2.0)).unwrap();
        assert_eq!(w0.0, 0.0);
        assert_eq!(w0.1, 33.0);
        assert_eq!(v0.0, 179.0);
        assert_eq!(v0.1, 39.0);
        assert_ne!((w0.0, w0.1), (v0.0, v0.1));
        let cx = 1728.0 * 2.0 / 2.0;
        let cy = 960.0 * 2.0 / 2.0;
        let mid = map_screenshot_pixel(cx, cy, "window", Some(win), Some(2.0), Some(wv), Some(2.0)).unwrap();
        assert!((mid.0 - (0.0 + 1728.0 / 2.0)).abs() < 0.6);
        assert!((mid.1 - (33.0 + 960.0 / 2.0)).abs() < 0.6);
        assert!(map_screenshot_pixel(10_000.0, 0.0, "window", Some(win), Some(2.0), Some(wv), Some(2.0)).is_none());
        assert!(ax_point_from_pixel(0.0, 0.0, win, 3.0).is_some());
        assert!(ax_point_from_pixel(0.0, 0.0, win, 4.0).is_none());
    }
}
