use std::fs;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::{ConnectInfo, DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, HeaderName, Method, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use base64::Engine;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower_http::cors::{AllowOrigin, CorsLayer};
use vcu_core::{
    new_id, ActionRequest, ActionResult, AdapterKind, BackendKind, Blackboard, BrowserKind,
    Envelope, ErrorCode, Observation, Session, SessionPolicy, SnapshotMode, SurfaceKind, TabInfo,
    VcuError, VisionInfo, VisionPolicy, VcuPaths,
};

use crate::app::{element_wait_match, extract_elements, json_frame, map_screenshot_pixel, merge_login_latest_page_url, pixel_scale, publish_feishu_latest, publish_login_latest, smallest_ref_at_point, webview_crop_frame, AppCapture, LoginLatestMeta};
use crate::doctor;
use crate::state::{create_backend, AppState, SessionSlot};
use crate::vision::VisionService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/extension/bootstrap", post(extension_bootstrap))
        .route("/v1/extension/hello", post(extension_hello))
        .route("/v1/extension/poll", get(extension_poll).post(extension_poll_post))
        .route("/v1/extension/result", post(extension_result).layer(DefaultBodyLimit::max(16 * 1024 * 1024)))
        .route("/v1/app/windows", get(app_windows))
        .route("/v1/app/snapshot", post(app_snapshot))
        .route("/v1/app/invoke", post(app_invoke))
        .route("/v1/app/focus", post(app_focus))
        .route("/v1/doctor", get(doctor_handler))
        .route("/v1/browser/login-state", get(browser_login_state))
        .route("/v1/browser/observe", post(browser_observe))
        .route("/v1/browser/install-lens", post(browser_install_lens))
        .route("/v1/browser/click", post(browser_click))
        .route("/v1/browser/hover", post(browser_hover))
        .route("/v1/browser/type", post(browser_type))
        .route("/v1/browser/scroll", post(browser_scroll))
        .route("/v1/browser/wait", post(browser_wait))
        .route("/v1/browser/key", post(browser_key))
        .route("/v1/browser/extract", post(browser_extract))
        .route("/v1/browser/ping", post(browser_ping))
        .route("/v1/browser/open", post(browser_open))
        .route("/v1/browser/tabs", get(browser_tabs))
        .route("/v1/browser/screenshot", post(browser_screenshot))
        .route("/v1/browser/select", post(browser_select))
        .route("/v1/browser/close", post(browser_close))
        .route("/v1/browser/group", post(browser_group))
        .route("/v1/browser/group/update", post(browser_group_update))
        .route("/v1/browser/ungroup", post(browser_ungroup))
        .route("/v1/session/start", post(session_start))
        .route("/v1/session/list", get(session_list))
        .route("/v1/session/{id}", get(session_show))
        .route("/v1/session/{id}/stop", post(session_stop))
        .route("/v1/session/{id}/abort", post(session_abort))
        .route("/v1/session/{id}/checkpoint", post(session_checkpoint))
        .route("/v1/session/{id}/request-help", post(session_request_help))
        .route("/v1/session/{id}/tabs", get(tabs_list))
        .route("/v1/session/{id}/tabs/borrow", post(tabs_borrow))
        .route("/v1/session/{id}/tabs/return", post(tabs_return))
        .route("/v1/session/{id}/navigate", post(navigate))
        .route("/v1/session/{id}/snapshot", post(snapshot))
        .route("/v1/session/{id}/click", post(click))
        .route("/v1/session/{id}/type", post(type_text))
        .route("/v1/session/{id}/extract", post(extract))
        .route("/v1/session/{id}/screenshot", post(screenshot))
        .route("/v1/session/{id}/act", post(act))
        .route("/v1/session/{id}/blackboard", get(blackboard_get))
        .route("/v1/model/test", post(model_test))
        .with_state(Arc::new(state))
        .layer(cors_layer())
}

fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([
            header::CONTENT_TYPE,
            HeaderName::from_static("x-vcu-token"),
        ])
        .allow_origin(AllowOrigin::predicate(|origin, _req| {
            let b = origin.as_bytes();
            b.starts_with(b"chrome-extension://")
                || b.starts_with(b"moz-extension://")
                || b.starts_with(b"safari-web-extension://")
        }))
}

async fn require_auth(headers: &HeaderMap, state: &AppState) -> Result<(), VcuError> {
    let expected = state.config.read().await.pairing_token.clone();
    let got = headers
        .get("x-vcu-token")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if got == expected {
        Ok(())
    } else {
        Err(VcuError::coded(
            ErrorCode::DaemonAuthFailed,
            "missing or invalid X-Vcu-Token",
        ))
    }
}

fn err_response(err: VcuError) -> axum::response::Response {
    let status = match err.code() {
        ErrorCode::DaemonAuthFailed => StatusCode::UNAUTHORIZED,
        ErrorCode::SessionNotFound | ErrorCode::TabNotFound | ErrorCode::ModelNotFound => {
            StatusCode::NOT_FOUND
        }
        ErrorCode::BorrowRequired
        | ErrorCode::OsCursorDenied
        | ErrorCode::FocusPolicyViolation
        | ErrorCode::VisionProviderRequired
        | ErrorCode::DaemonAlreadyRunning
        | ErrorCode::AccessibilityDenied
        | ErrorCode::AppDenied => StatusCode::CONFLICT,
        ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        _ => StatusCode::BAD_REQUEST,
    };
    let body = Envelope::<Value>::from_error(&err);
    (status, Json(body)).into_response()
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let mut login = crate::login_state::inspect_login_browsers();
    if state.extension_bridge.likely_user_profile().await {
        login.extension_profile = "user";
    }
    login.next_action = crate::login_state::login_next_action(
        login.user_browsers.is_empty() && login.extension_profile != "user",
        login.lens_copied,
        login.extension_profile,
        false,
        &login.lens_dir,
        false,
    );
    Json(Envelope::ok(json!({
        "version": state.version,
        "started_at": state.started_at,
        "sessions": state.sessions.read().await.len(),
        "pid": std::process::id(),
        "extension_connected": state.extension_bridge.is_connected().await,
        "extension_polling": state.extension_bridge.is_polling().await,
        "extension_browsers": state.extension_bridge.active_browsers().await,
        "extension_browser_count": state.extension_bridge.active_client_ids().await.len(),
        "extension_clients": state.extension_bridge.client_snapshots().await,
        "last_poll_age_ms": state.extension_bridge.last_poll_age_ms().await,
        "extension_pending": state.extension_bridge.pending_len().await,
        "extension_waiters": state.extension_bridge.waiter_len().await,
        "extension_profile": login.extension_profile,
        "login_next_action": login.next_action,
        "host_vision": login.host_vision,
        "lens_copied": login.lens_copied,
        "lens_dir": login.lens_dir,
        "observe": "vcu browser observe --json",
        "click": "vcu browser click --selector body --dry-run --json  # after observe; viewport pixels need screenshot capture_id",
        "install_lens": "vcu browser install-lens",
    })))
}

async fn doctor_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let report = doctor::build_report(&state.paths, Some(&state)).await;
    Json(Envelope::ok(report)).into_response()
}


#[derive(Deserialize)]
struct BrowserClickReq {
    #[serde(default)]
    pixel_x: Option<f64>,
    #[serde(default)]
    pixel_y: Option<f64>,
    #[serde(default)]
    space: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    app_id: Option<String>,
    /// Flash Guide at the mapped AX point (no HUD capsule).
    #[serde(default)]
    guide: bool,
    /// CSS selector: DOM click via USER extension (no pixel map).
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
    #[serde(default)]
    capture_id: Option<String>,
}

async fn browser_click(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserClickReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if req.space.as_deref() == Some("viewport") {
        return browser_viewport_click(state, req).await;
    }
    if req.capture_id.is_some() {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "capture_id requires space=viewport"));
    }
    if req.selector.is_some() && (req.pixel_x.is_some() || req.pixel_y.is_some()) {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "choose selector or screenshot pixels, not both"));
    }
    if req.selector.is_none() && req.tab_id.is_some() {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "tab_id requires selector; pixel clicks target the observed browser window"));
    }
    let login = crate::login_state::inspect_login_browsers();
    if !req.dry_run && login.allow_dialog_visible {
        return err_response(VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "Allow debugging dialog is visible; agent will not click. Use --dry-run or observe.",
        ));
    }
    if let Some(selector) = req.selector.as_deref().filter(|s| !s.is_empty()) {
        if !state.extension_bridge.is_polling().await {
            return err_response(VcuError::coded(
                ErrorCode::ExtensionDisconnected,
                "USER Edge extension is not polling; cannot click via DOM selector",
            ));
        }
        if !state.extension_bridge.likely_user_profile().await {
            return err_response(VcuError::coded(
                ErrorCode::ActionFailed,
                "extension_profile is not user; refusing Agent Edge click",
            ));
        }
        let mut params = json!({
            "selector": selector,
            "dry_run": req.dry_run,
        });
        let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
            Ok(v) => v,
            Err(e) => return err_response(e),
        };
        if let Some(id) = tab.as_deref() {
            params["tab_id"] = json_tab_param(id);
        }
        match extension_dom_call_for(&state, "click", params, 8, kind).await {
            Ok(v) => {
                return Json(Envelope::ok(json!({
                    "login_state": true,
                    "hud": false,
                    "pressed": v.get("pressed").cloned().unwrap_or(json!(false)),
                    "dry_run": req.dry_run,
                    "selector": selector,
                    "source": "extension_dom",
                    "tab_id": v.get("tab_id").cloned().unwrap_or(json!(tab)),
                    "tab_id_source": tab_src,
                    "browser": kind,
                    "page_url": v.get("page_url").cloned().unwrap_or(json!(null)),
                    "focused": v.get("focused").cloned().unwrap_or(json!(null)),
                    "os_cursor_used": false,
                    "never_click_allow": true,
                    "extension": v,
                }))).into_response();
            }
            Err(e) => return err_response(e),
        }
    }
    let Some(px) = req.pixel_x else {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "pixel_x required"));
    };
    let Some(py) = req.pixel_y else {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "pixel_y required"));
    };
    let space = req.space.as_deref().unwrap_or("window");
    let last = state.last_observe.read().await.clone();
    let id = match login_user_app_id(&login, req.app_id.clone(), last.as_ref()) {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    let sidecar = state.paths.captures_dir().join("login-latest.json");
    let side = fs::read(&sidecar)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    if side.as_ref().and_then(|v| v.get("app_id")).and_then(Value::as_str) != Some(id.as_str()) {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "observe this browser first; screenshot belongs to a different or missing window"));
    }
    let mut window_frame = side.as_ref().and_then(|v| json_frame(v.get("screenshot_frame")));
    let window_scale = side.as_ref().and_then(|v| v.get("screenshot_scale").and_then(|x| x.as_f64()));
    let mut webview_frame = side.as_ref().and_then(|v| json_frame(v.get("webview_screenshot_frame")));
    let mut webview_scale = side.as_ref().and_then(|v| v.get("webview_screenshot_scale").and_then(|x| x.as_f64()));
    let mut refs: Vec<(String, [f64; 4])> = Vec::new();
    let sidecar_ready = match req.space.as_deref().unwrap_or("window") {
        "webview" => webview_frame.is_some() && webview_scale.is_some(),
        _ => window_frame.is_some() && window_scale.is_some(),
    };
    if !(req.dry_run && sidecar_ready) {
        let backend = state.app_backend.read().await;
        match backend.snapshot(&id, 2000).await {
            Ok(snap) => {
                if !req.dry_run && window_frame != snap.window_frame {
                    return err_response(VcuError::coded(ErrorCode::InvalidInput, "browser window moved or resized since screenshot; observe again before clicking"));
                }
                refs = snap
                    .elements
                    .iter()
                    .filter_map(|e| e.frame.map(|f| (e.r#ref.clone(), f)))
                    .collect();
                if window_frame.is_none() {
                    window_frame = snap.window_frame;
                }
                if webview_frame.is_none() {
                    webview_frame = webview_crop_frame(snap.elements.as_slice(), snap.webview_ref.as_deref());
                }
            }
            Err(e) if !req.dry_run || (window_frame.is_none() && webview_frame.is_none()) => return err_response(e),
            Err(_) => {}
        }
    }
    if window_scale.is_none() {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "screenshot scale missing; observe again before clicking"));
    }
    if webview_scale.is_none() && webview_frame.is_some() {
        webview_scale = window_scale;
    }
    let mut space = space.to_string();
    if space == "webview" && webview_frame.is_none() {
        if let Some(w) = window_frame {
            webview_frame = Some(crate::app::edge_webview_heuristic(w));
            webview_scale = window_scale;
            space = "webview".into();
        } else {
            space = "window".into();
        }
    }
    let Some((ax, ay, frame, scale)) = map_screenshot_pixel(
        px,
        py,
        space.as_str(),
        window_frame,
        window_scale,
        webview_frame,
        webview_scale,
    ) else {
        return err_response(VcuError::coded(
            ErrorCode::InvalidInput,
            format!("cannot map pixel ({px},{py}) space={space}"),
        ));
    };
    let hit = smallest_ref_at_point(&refs, ax, ay);
    let mut body = json!({
        "app_id": id,
        "login_state": true,
        "hud": false,
        "dry_run": req.dry_run,
        "pressed": false,
        "os_cursor_used": false,
        "never_click_allow": true,
        "pixel": {"x": px, "y": py, "space": space},
        "ax_point": {"x": ax, "y": ay},
        "screenshot_scale": scale,
        "screenshot_frame": frame,
        "hit_ref": hit,
        "coordinate_help": "ax = frame_origin + pixel / screenshot_scale",
        "guide": { "overlay": false, "os_cursor_used": false },
    });
    if req.guide {
        match crate::stage::StageHandle::flash_guide(ax, ay, 180) {
            Ok(g) => {
                body["guide"] = json!({
                    "x": g.x,
                    "y": g.y,
                    "overlay": true,
                    "hud": false,
                    "os_cursor_used": false
                });
            }
            Err(e) => {
                body["guide"] = json!({
                    "overlay": false,
                    "error": e.message(),
                    "os_cursor_used": false
                });
            }
        }
    }
    if req.dry_run {
        return Json(Envelope::ok(body)).into_response();
    }
    let pressed = {
        let mut backend = state.app_backend.write().await;
        backend.press_at_point(&id, ax, ay).await
    };
    match pressed {
        Ok(detail) => {
            body["pressed"] = json!(true);
            body["press"] = detail;
            Json(Envelope::ok(body)).into_response()
        }
        Err(e) => err_response(e),
    }
}


fn login_user_app_id(
    login: &crate::login_state::LoginBrowserReport,
    app_id: Option<String>,
    last: Option<&crate::login_state::LastObserve>,
) -> Result<String, VcuError> {
    if let Some(id) = crate::login_state::bind_app_id(app_id.as_deref(), last, std::time::Instant::now()) {
        return Ok(id);
    }
    let Some(user) = login.user_browsers.first() else {
        return Err(VcuError::coded(
            ErrorCode::ActionFailed,
            "no user Chrome/Edge; login-state needs the user browser window",
        ));
    };
    Ok(format!("proc:{}:{}", user.name.replace(' ', "_"), user.pid))
}

fn json_tab_param(id: &str) -> Value {
    json!(id)
}

async fn remember_observe(state: &AppState, app_id: &str, snap: &Value) {
    let tab_id = snap
        .get("tab_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    *state.last_observe.write().await = Some(crate::login_state::LastObserve {
        app_id: app_id.to_string(),
        tab_id,
        at: std::time::Instant::now(),
    });
}

async fn bound_tab(state: &AppState, explicit: Option<&str>) -> (Option<String>, &'static str) {
    let last = state.last_observe.read().await.clone();
    crate::login_state::bind_tab_id(explicit, last.as_ref(), std::time::Instant::now())
}

async fn bound_dom_target(
    state: &AppState,
    explicit_tab: Option<&str>,
    browser: Option<&str>,
) -> Result<(Option<String>, &'static str, Option<&'static str>), VcuError> {
    let want = parse_observe_browser(browser)?;
    let explicit = explicit_tab
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .is_some();
    let (tab, src) = bound_tab(state, explicit_tab).await;
    let last = state.last_observe.read().await.clone();
    let last_kind = last
        .as_ref()
        .and_then(|l| crate::login_state::browser_kind_from_app_id(&l.app_id));
    if let Some(id) = tab.as_deref() {
        let tabs = state.extension_bridge.list_tabs_merged().await?;
        let filter = if explicit { want } else { want.or(last_kind) };
        let found = find_extension_tab(&tabs, id, filter)?;
        let kind = kind_from_tab(&found).or(filter);
        return Ok((
            json_tab_id_value(&found).or_else(|| Some(id.to_string())),
            src,
            kind,
        ));
    }
    Ok((None, src, want.or(last_kind)))
}

async fn extension_dom_call_for(
    state: &AppState,
    method: &str,
    params: Value,
    timeout: u64,
    kind: Option<&str>,
) -> Result<Value, VcuError> {
    state
        .extension_bridge
        .call_timeout_hinted(method, params, timeout, kind)
        .await
}

async fn extension_dom_call(
    state: &AppState,
    method: &str,
    params: Value,
    timeout: u64,
) -> Result<Value, VcuError> {
    let last = state.last_observe.read().await.clone();
    let kind = last
        .as_ref()
        .and_then(|l| crate::login_state::browser_kind_from_app_id(&l.app_id));
    state
        .extension_bridge
        .call_timeout_hinted(method, params, timeout, kind)
        .await
}

#[derive(Deserialize)]
struct BrowserTypeReq {
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    r#ref: Option<String>,
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
}

#[derive(Deserialize)]
struct BrowserHoverReq {
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
    #[serde(default)]
    dry_run: bool,
}

async fn browser_hover(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserHoverReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let Some(selector) = req.selector.as_deref().filter(|s| !s.is_empty()) else {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "hover requires selector"));
    };
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(
            ErrorCode::ExtensionDisconnected,
            "USER browser extension is not polling; cannot hover via DOM selector",
        ));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return err_response(VcuError::coded(
            ErrorCode::ActionFailed,
            "extension_profile is not user; refusing Agent Edge hover",
        ));
    }
    let mut params = json!({
        "selector": selector,
        "dry_run": req.dry_run,
    });
    let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    if let Some(id) = tab.as_deref() {
        params["tab_id"] = json_tab_param(id);
    }
    match extension_dom_call_for(&state, "hover", params, 8, kind).await {
        Ok(v) => Json(Envelope::ok(json!({
            "login_state": true,
            "hud": false,
            "hovered": v.get("hovered").cloned().unwrap_or(json!(false)),
            "dry_run": req.dry_run,
            "selector": selector,
            "source": "extension_dom",
            "tab_id": v.get("tab_id").cloned().unwrap_or(json!(tab)),
            "tab_id_source": tab_src,
            "browser": kind,
            "page_url": v.get("page_url").cloned().unwrap_or(json!(null)),
            "os_cursor_used": false,
            "never_click_allow": true,
            "extension": v,
        }))).into_response(),
        Err(e) => err_response(e),
    }
}

async fn browser_type(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserTypeReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let login = crate::login_state::inspect_login_browsers();
    if !req.dry_run && login.allow_dialog_visible {
        return err_response(VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "Allow debugging dialog is visible; agent will not type.",
        ));
    }
    if let Some(selector) = req.selector.as_deref().filter(|s| !s.is_empty()) {
        if !state.extension_bridge.is_polling().await {
            return err_response(VcuError::coded(
                ErrorCode::ExtensionDisconnected,
                "USER Edge extension is not polling; cannot type via DOM selector",
            ));
        }
        if !state.extension_bridge.likely_user_profile().await {
            return err_response(VcuError::coded(
                ErrorCode::ActionFailed,
                "extension_profile is not user; refusing Agent Edge type",
            ));
        }
        let mut params = json!({
            "selector": selector,
            "text": req.text.clone().unwrap_or_default(),
            "dry_run": req.dry_run,
        });
        let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
            Ok(v) => v,
            Err(e) => return err_response(e),
        };
        if let Some(id) = tab.as_deref() { params["tab_id"] = json_tab_param(id); }
        match extension_dom_call_for(&state, "type", params, 8, kind).await {
            Ok(v) => {
                return Json(Envelope::ok(json!({
                    "login_state": true,
                    "hud": false,
                    "typed": v.get("typed").cloned().unwrap_or(json!(false)),
                    "dry_run": req.dry_run,
                    "selector": selector,
                    "source": "extension_dom",
                    "tab_id": v.get("tab_id").cloned().unwrap_or(json!(tab)),
                    "tab_id_source": tab_src,
                    "browser": kind,
                    "page_url": v.get("page_url"),
                    "focused": v.get("focused"),
                    "os_cursor_used": false,
                    "never_click_allow": true,
                    "extension": v,
                }))).into_response();
            }
            Err(e) => return err_response(e),
        }
    }
    if req.tab_id.is_some() {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "tab_id requires a DOM selector for type"));
    }
    let last = state.last_observe.read().await.clone();
    let id = match login_user_app_id(&login, req.app_id.clone(), last.as_ref()) {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    #[cfg(target_os = "macos")]
    let found = crate::app::macos::login_find_address_field(&id);
    #[cfg(not(target_os = "macos"))]
    let found: Option<(String, String)> = None;
    let Some((field_name, field_value)) = found else {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "no address/search field for login-state type"));
    };
    let eref = req.r#ref.clone().unwrap_or_else(|| "address-bar".to_string());
    let mut body = json!({
        "app_id": id,
        "login_state": true,
        "hud": false,
        "dry_run": req.dry_run,
        "typed": false,
        "ref": eref,
        "field_name": field_name,
        "field_value": field_value,
        "os_cursor_used": false,
        "never_click_allow": true,
    });
    let merged = merge_login_latest_page_url(&state.paths.captures_dir(), &field_value);
    body["login_latest_url_merged"] = json!(merged);
    if req.dry_run {
        return Json(Envelope::ok(body)).into_response();
    }
    let text = req.text.unwrap_or_default();
    #[cfg(target_os = "macos")]
    let set_res = crate::app::macos::login_set_address_field(&id, &text);
    #[cfg(not(target_os = "macos"))]
    let set_res: Result<String, VcuError> = Err(VcuError::coded(ErrorCode::NotImplemented, "login-state type is macOS AX"));
    match set_res {
        Ok(detail) => {
            body["typed"] = json!(true);
            body["press"] = json!(detail);
            Json(Envelope::ok(body)).into_response()
        }
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct BrowserScrollReq {
    #[serde(default = "default_scroll_dy")]
    dy: i32,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
}
fn default_scroll_dy() -> i32 { 600 }

async fn browser_scroll(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserScrollReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let login = crate::login_state::inspect_login_browsers();
    if !req.dry_run && login.allow_dialog_visible {
        return err_response(VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "Allow debugging dialog is visible; agent will not scroll.",
        ));
    }
    let mut body = json!({
        "login_state": true,
        "hud": false,
        "dry_run": req.dry_run,
        "scrolled": false,
        "dy": req.dy,
        "os_cursor_used": false,
        "never_click_allow": true,
    });
    if state.extension_bridge.is_polling().await && state.extension_bridge.likely_user_profile().await {
        let mut params = json!({"dy": req.dy, "dry_run": req.dry_run});
        let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
            Ok(v) => v,
            Err(e) => return err_response(e),
        };
        if let Some(id) = tab.as_deref() { params["tab_id"] = json_tab_param(id); }
        body["tab_id_source"] = json!(tab_src);
        body["browser"] = json!(kind);
        match extension_dom_call_for(&state, "scroll", params, 8, kind).await {
            Ok(v) => {
                body["scrolled"] = v.get("scrolled").cloned().unwrap_or(json!(false));
                body["source"] = json!("extension_dom");
                for key in ["tab_id", "page_url", "focused"] {
                    if let Some(value) = v.get(key) { body[key] = value.clone(); }
                }
                body["extension"] = v;
                return Json(Envelope::ok(body)).into_response();
            }
            Err(e) => return err_response(e),
        }
    }
    if req.tab_id.is_some() {
        return err_response(VcuError::coded(ErrorCode::ExtensionDisconnected, "explicit tab scroll requires the USER extension"));
    }
    if req.dry_run {
        return Json(Envelope::ok(body)).into_response();
    }
    let last = state.last_observe.read().await.clone();
    let id = match login_user_app_id(&login, req.app_id.clone(), last.as_ref()) {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    body["app_id"] = json!(id);
    let mut backend = state.app_backend.write().await;
    match backend.scroll(&id, None, req.dy).await {
        Ok(detail) => {
            body["scrolled"] = json!(true);
            body["press"] = detail;
            Json(Envelope::ok(body)).into_response()
        }
        Err(e) => err_response(e),
    }
}


#[derive(Deserialize)]
struct BrowserWaitReq {
    #[serde(default = "default_wait_ms")]
    ms: u64,
    #[serde(default)]
    r#ref: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    app_id: Option<String>,
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
}
fn default_wait_ms() -> u64 { 200 }

fn extract_match_text(m: &Value) -> String {
    let text = m.get("text").and_then(Value::as_str).unwrap_or("");
    let value = m.get("value").and_then(Value::as_str).unwrap_or("");
    format!("{text}{value}")
}

fn extract_has_text(matches: &Value, needle: Option<&str>) -> bool {
    let Some(arr) = matches.as_array() else {
        return false;
    };
    if arr.is_empty() {
        return false;
    }
    let Some(needle) = needle.map(str::trim).filter(|s| !s.is_empty()) else {
        return true;
    };
    arr.iter().any(|m| extract_match_text(m).contains(needle))
}

async fn browser_wait(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserWaitReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let ms = req.ms.min(8000);
    if let Some(selector) = req.selector.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        if let Err(e) = user_extension_ready(&state).await {
            return err_response(e);
        }
        let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
            Ok(v) => v,
            Err(e) => return err_response(e),
        };
        let started = std::time::Instant::now();
        let budget = std::time::Duration::from_millis(ms.max(1));
        let needle = req.text.as_deref().map(str::trim).filter(|s| !s.is_empty());
        loop {
            let mut params = json!({"selector": selector});
            if let Some(id) = tab.as_deref() {
                params["tab_id"] = json_tab_param(id);
            }
            match extension_dom_call_for(&state, "extract", params, 8, kind).await {
                Ok(v) => {
                    let matches = v.get("matches").cloned().unwrap_or(json!([]));
                    let count = v.get("count").and_then(Value::as_u64).unwrap_or_else(|| {
                        matches.as_array().map(|a| a.len() as u64).unwrap_or(0)
                    });
                    if count > 0 && extract_has_text(&matches, needle) {
                        return Json(Envelope::ok(json!({
                            "login_state": true,
                            "hud": false,
                            "waited_ms": started.elapsed().as_millis() as u64,
                            "found": true,
                            "count": count,
                            "selector": selector,
                            "text": req.text,
                            "source": "extension_dom",
                            "tab_id": v.get("tab_id").cloned().unwrap_or(json!(tab.clone())),
                            "tab_id_source": tab_src,
                            "browser": kind,
                            "os_cursor_used": false,
                        }))).into_response();
                    }
                }
                Err(e) => {
                    if started.elapsed() >= budget {
                        return err_response(e);
                    }
                }
            }
            if started.elapsed() >= budget {
                return err_response(VcuError::coded(
                    ErrorCode::ActionFailed,
                    format!(
                        "wait timed out after {ms}ms for selector={selector:?} text={:?}",
                        req.text
                    ),
                ));
            }
            tokio::time::sleep(std::time::Duration::from_millis(80)).await;
        }
    }
    let login = crate::login_state::inspect_login_browsers();
    let last = state.last_observe.read().await.clone();
    let id = match login_user_app_id(&login, req.app_id.clone(), last.as_ref()) {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    let conditioned = req.r#ref.is_some() || req.name.is_some() || req.role.is_some();
    let started = std::time::Instant::now();
    let role_l = req.role.as_deref().unwrap_or("").to_ascii_lowercase();
    if role_l.contains("webarea") || role_l.contains("webview") {
        return Json(Envelope::ok(json!({
            "app_id": id,
            "login_state": true,
            "hud": false,
            "waited_ms": 0,
            "found_ref": "webview",
            "os_cursor_used": false,
            "fast": true,
        }))).into_response();
    }
    if !conditioned {
        tokio::time::sleep(std::time::Duration::from_millis(ms)).await;
        return Json(Envelope::ok(json!({
            "app_id": id,
            "login_state": true,
            "hud": false,
            "waited_ms": started.elapsed().as_millis() as u64,
            "os_cursor_used": false,
        }))).into_response();
    }
    let budget = std::time::Duration::from_millis(ms.max(1));
    loop {
        let backend = state.app_backend.read().await;
        let snap = match backend.snapshot(&id, 2000).await {
            Ok(s) => s,
            Err(e) => return err_response(e),
        };
        drop(backend);
        if let Some(found) = element_wait_match(
            &snap.elements,
            req.r#ref.as_deref(),
            req.name.as_deref(),
            req.role.as_deref(),
        ) {
            return Json(Envelope::ok(json!({
                "app_id": id,
                "login_state": true,
                "hud": false,
                "waited_ms": started.elapsed().as_millis() as u64,
                "found_ref": found,
                "os_cursor_used": false,
            }))).into_response();
        }
        if started.elapsed() >= budget {
            return err_response(VcuError::coded(
                ErrorCode::ActionFailed,
                format!(
                    "wait timed out after {ms}ms for ref={:?} name={:?} role={:?}",
                    req.r#ref, req.name, req.role
                ),
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }
}


#[derive(Deserialize)]
struct BrowserKeyReq {
    key: String,
    #[serde(default)]
    dry_run: bool,
    #[serde(default)]
    confirm_send: bool,
    #[serde(default)]
    r#ref: Option<String>,
}

async fn browser_key(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserKeyReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let _ = state;
    let has_ref = req.r#ref.as_deref().map(|s| !s.is_empty()).unwrap_or(false);
    match crate::login_state::plan_login_key(&req.key, req.confirm_send, has_ref) {
        Ok(plan) => Json(Envelope::ok(json!({
            "login_state": true,
            "hud": false,
            "dry_run": req.dry_run,
            "pressed": false,
            "key": req.key,
            "plan": plan,
            "os_cursor_used": false,
            "hid_injected": false,
            "never_click_allow": true,
            "confirm_send": req.confirm_send,
        }))).into_response(),
        Err(e) => {
            if req.dry_run {
                Json(Envelope::ok(json!({
                    "login_state": true,
                    "hud": false,
                    "dry_run": true,
                    "pressed": false,
                    "key": req.key,
                    "blocked": true,
                    "code": format!("{:?}", e.code()),
                    "message": e.message(),
                    "os_cursor_used": false,
                    "hid_injected": false,
                }))).into_response()
            } else {
                err_response(e)
            }
        }
    }
}

#[derive(Deserialize)]
struct BrowserExtractReq {
    #[serde(default = "default_extract_selector")]
    selector: String,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
}
fn default_extract_selector() -> String {
    "a".into()
}

async fn browser_extract(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserExtractReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(
            ErrorCode::ExtensionDisconnected,
            "USER Edge extension is not polling; load unpacked lens, do not use Agent Edge",
        ));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return err_response(VcuError::coded(
            ErrorCode::ActionFailed,
            "extension_profile is not user; refusing Agent Edge extract",
        ));
    }
    let mut params = json!({"selector": req.selector});
    // Prefer last observe tab; never substitute the first page in the other browser.
    let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    if let Some(id) = tab.as_deref() {
        params["tab_id"] = json_tab_param(id);
    }
    match extension_dom_call_for(&state, "extract", params, 8, kind).await {
        Ok(v) => {
            let matches = v.get("matches").cloned().unwrap_or(json!([]));
            let count = v.get("count").and_then(|x| x.as_u64()).unwrap_or_else(|| {
                matches.as_array().map(|a| a.len() as u64).unwrap_or(0)
            });
            Json(Envelope::ok(json!({
                "login_state": true,
                "hud": false,
                "extension_profile": "user",
                "source": "extension_dom",
                "tab_id": v.get("tab_id").cloned().unwrap_or(json!(tab.clone())),
                "tab_id_source": tab_src,
                "browser": kind,
                "selector": req.selector,
                "page_url": v.get("page_url"),
                "focused": v.get("focused"),
                "truncated": v.get("truncated").cloned().unwrap_or(json!(false)),
                "count": count,
                "matches": matches,
                "os_cursor_used": false,
                "never_click_allow": true,
            }))).into_response()
        }
        Err(e) => {
            let stale = crate::login_state::extension_error_is_stale(&e.message(), e.detail().as_deref());
            let msg = if stale {
                "extension service worker is stale (unknown method). Open edge://extensions and Reload VCU Browser Bridge. AX chrome is not HTML DOM. Never click Allow."
            } else {
                "extension DOM extract timed out or failed. Reload VCU Browser Bridge if `vcu browser ping` is not pong. AX chrome is not HTML DOM. Never click Allow."
            };
            err_response(VcuError::with_detail(
                ErrorCode::ActionFailed,
                msg,
                e.detail().unwrap_or_else(|| e.message()),
            ))
        }
    }
}

#[derive(Default, Deserialize)]
struct BrowserPingReq {
    #[serde(default)]
    reload: bool,
}

async fn browser_ping(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserPingReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(
            ErrorCode::ExtensionDisconnected,
            "USER Edge extension is not polling",
        ));
    }
    if req.reload {
        match state.extension_bridge.reload_all_clients().await {
            Ok(v) => {
                return Json(Envelope::ok(json!({
                    "login_state": true,
                    "hud": false,
                    "reloading": true,
                    "reloaded": v.get("reloaded").cloned().unwrap_or(json!(0)),
                    "extension": v,
                    "os_cursor_used": false,
                    "never_click_allow": true,
                }))).into_response();
            }
            Err(e) => return err_response(e),
        }
    }
    match state.extension_bridge.call_timeout("ping", json!({}), 3).await {
        Ok(v) => Json(Envelope::ok(json!({
            "login_state": true,
            "hud": false,
            "pong": v.get("pong").cloned().unwrap_or(json!(true)),
            "extension": v,
            "os_cursor_used": false,
        }))).into_response(),
        Err(e) => {
            if crate::login_state::extension_error_is_stale(&e.message(), e.detail().as_deref()) {
                return err_response(VcuError::with_detail(
                    ErrorCode::ActionFailed,
                    "extension service worker is stale (unknown method ping). Open edge://extensions and click Reload on VCU Browser Bridge. Never click Allow.",
                    e.detail().unwrap_or_else(|| e.message()),
                ));
            }
            err_response(e)
        }
    }
}

#[derive(Deserialize)]
struct BrowserScreenshotReq {
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
}

async fn user_extension_ready(state: &AppState) -> Result<(), VcuError> {
    if !state.extension_bridge.is_polling().await {
        return Err(VcuError::coded(ErrorCode::ExtensionDisconnected, "USER browser extension is not polling"));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return Err(VcuError::coded(ErrorCode::ActionFailed, "USER browser extension required"));
    }
    Ok(())
}

async fn browser_screenshot(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(req): Json<BrowserScreenshotReq>) -> axum::response::Response {
    if let Err(e) = require_auth(&headers, &state).await { return err_response(e); }
    if let Err(e) = user_extension_ready(&state).await { return err_response(e); }
    let mut params = json!({});
    let (tab, tab_src, kind) = match bound_dom_target(&state, req.tab_id.as_deref(), req.browser.as_deref()).await {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    if let Some(id) = tab.as_deref() {
        params["tab_id"] = json_tab_param(id);
    }
    let mut result = match extension_dom_call_for(&state, "capture_tab", params, 8, kind).await {
        Ok(v) => v, Err(e) => return err_response(e),
    };
    let invalid = || VcuError::coded(ErrorCode::ActionFailed, "extension screenshot must contain a PNG and bound viewport metadata");
    let Some(encoded) = result.get("png_base64").and_then(Value::as_str) else { return err_response(invalid()); };
    if encoded.len() > 24_000_000 { return err_response(invalid()); }
    let png = match base64::engine::general_purpose::STANDARD.decode(encoded) {
        Ok(v) => v, Err(_) => return err_response(invalid()),
    };
    let Some((width, height)) = crate::app::png_ihdr_size(&png) else { return err_response(invalid()); };
    let viewport = result.get("viewport").cloned().unwrap_or(Value::Null);
    if viewport.get("document_id").and_then(Value::as_str).is_none()
        || result.get("tab_id").and_then(Value::as_str).is_none()
        || viewport.get("width").and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0).is_none()
        || viewport.get("height").and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0).is_none() {
        return err_response(invalid());
    }
    let capture_id = new_id();
    let dir = state.paths.captures_dir();
    let path = dir.join(format!("viewport-{capture_id}.png"));
    let metadata = json!({"capture_id": capture_id, "tab_id": result["tab_id"], "viewport": viewport, "width": width, "height": height, "created_at_ms": Utc::now().timestamp_millis()});
    let write = fs::create_dir_all(&dir).and_then(|_| fs::write(&path, &png))
        .and_then(|_| fs::write(dir.join(format!("viewport-{capture_id}.json")), serde_json::to_vec(&metadata).unwrap()));
    if let Err(e) = write { return err_response(VcuError::with_detail(ErrorCode::Internal, "save viewport screenshot", e.to_string())); }
    result.as_object_mut().unwrap().remove("png_base64");
    // Keep the complete geometry binding in the sidecar, not in every model
    // tool response. A dense page can otherwise emit thousands of JSON fields.
    summarize_viewport_result(&mut result);
    result["capture_id"] = json!(capture_id);
    result["screenshot_path"] = json!(path.display().to_string());
    result["screenshot_width"] = json!(width);
    result["screenshot_height"] = json!(height);
    result["coordinate_space"] = json!("viewport");
    result["source"] = json!("extension_viewport");
    result["tab_id_source"] = json!(tab_src);
    result["browser"] = json!(kind);
    result["vision_handoff"] = json!({"must_view": [path], "serial":true, "rule":"View this image before clicking. Use its capture_id and space=viewport; captures expire after 60s and a real click consumes the capture."});
    Json(Envelope::ok(result)).into_response()
}

async fn browser_viewport_click(state: Arc<AppState>, req: BrowserClickReq) -> axum::response::Response {
    if req.selector.is_some() || req.app_id.is_some() {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "viewport click takes capture_id and screenshot pixels, without selector/app_id"));
    }
    let Some(id) = req.capture_id.as_deref().filter(|s| s.len() == 26 && s.bytes().all(|c| c.is_ascii_alphanumeric())) else {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "capture_id from browser screenshot is required"));
    };
    let dir = state.paths.captures_dir();
    let metadata = fs::read(dir.join(format!("viewport-{id}.json"))).ok().and_then(|b| serde_json::from_slice::<Value>(&b).ok());
    let Some(meta) = metadata else { return err_response(VcuError::coded(ErrorCode::InvalidInput, "screenshot capture not found; capture again")); };
    let age = Utc::now().timestamp_millis() - meta["created_at_ms"].as_i64().unwrap_or(0);
    if !(0..=60_000).contains(&age) { return err_response(VcuError::coded(ErrorCode::InvalidInput, "screenshot expired; capture again")); }
    let Some(tab) = meta["tab_id"].as_str() else { return err_response(VcuError::coded(ErrorCode::InvalidInput, "capture has no tab")); };
    if req.tab_id.as_deref().is_some_and(|t| t != tab) { return err_response(VcuError::coded(ErrorCode::InvalidInput, "tab differs from screenshot target")); }
    let Some((x, y)) = viewport_pixel_point(req.pixel_x, req.pixel_y, &meta) else {
        return err_response(VcuError::coded(ErrorCode::InvalidInput, "viewport pixels must be finite and inside the screenshot"));
    };
    if let Err(e) = user_extension_ready(&state).await { return err_response(e); }
    let used = dir.join(format!("viewport-{id}.used"));
    if used.exists() { return err_response(VcuError::coded(ErrorCode::InvalidInput, "screenshot already used for an action; capture again")); }
    if !req.dry_run {
        // Atomic consumption prevents concurrent or user-retried actions from
        // replaying against a screenshot after an ambiguous/lost reply.
        if fs::OpenOptions::new().write(true).create_new(true).open(&used).is_err() {
            return err_response(VcuError::coded(ErrorCode::InvalidInput, "cannot consume screenshot; capture again"));
        }
    }
    let params = json!({"tab_id": tab, "x":x, "y":y, "expected_viewport":meta["viewport"], "dry_run":req.dry_run});
    match extension_dom_call(&state, "click_point", params, 8).await {
        Ok(mut v) => {
            v["capture_id"] = json!(id);
            v["coordinate_space"] = json!("viewport");
            v["source"] = json!("extension_dom");
            v["os_cursor_used"] = json!(false);
            Json(Envelope::ok(v)).into_response()
        }
        Err(e) => {
            if let Some(mut detail) = e.detail().and_then(|s| serde_json::from_str::<Value>(&s).ok()) {
                summarize_viewport_result(&mut detail);
                return err_response(VcuError::with_detail(e.code(), e.message(), detail.to_string()));
            }
            err_response(e)
        }
    }
}

fn summarize_viewport_result(result: &mut Value) {
    if let Some(signature) = result.pointer_mut("/viewport/layout_signature").and_then(Value::as_object_mut) {
        signature.remove("nodes");
    }
}

fn viewport_pixel_point(px: Option<f64>, py: Option<f64>, meta: &Value) -> Option<(f64, f64)> {
    let (px, py) = (px?, py?);
    let width = meta["width"].as_f64()?;
    let height = meta["height"].as_f64()?;
    let css_width = meta["viewport"]["width"].as_f64()?;
    let css_height = meta["viewport"]["height"].as_f64()?;
    if ![px, py, width, height, css_width, css_height].iter().all(|v| v.is_finite())
        || px < 0.0 || py < 0.0 || px >= width || py >= height || css_width <= 0.0 || css_height <= 0.0 { return None; }
    Some((px * css_width / width, py * css_height / height))
}

// Browser chrome operations are routed through the USER extension, not AX/CDP.
fn validate_tab_management(method: &str, params: &Value) -> Result<(), VcuError> {
    let invalid = |message: &str| VcuError::coded(ErrorCode::InvalidInput, message);
    let string = |key: &str| params.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty());
    let valid_id = |key: &str| string(key).and_then(|s| s.parse::<u32>().ok()).is_some();
    if !params.is_object() { return Err(invalid("expected an object")); }
    if matches!(method, "select_tab" | "close_tab") && !valid_id("tab_id") {
        return Err(invalid("tab_id must be a nonnegative integer string"));
    }
    if method == "update_group" && !valid_id("group_id") {
        return Err(invalid("group_id must be a nonnegative integer string"));
    }
    if matches!(method, "group_tabs" | "ungroup_tabs") {
        let ids = params.get("tab_ids").and_then(Value::as_array)
            .ok_or_else(|| invalid("tab_ids must be a nonempty array of ID strings"))?;
        if ids.is_empty() || ids.iter().any(|v| v.as_str().and_then(|s| s.parse::<u32>().ok()).is_none()) {
            return Err(invalid("tab_ids must be a nonempty array of ID strings"));
        }
    }
    if method == "group_tabs" && string("title").is_none() {
        return Err(invalid("title must be nonempty"));
    }
    for key in ["title", "session_name"] {
        if params.get(key).is_some() && string(key).is_none() { return Err(invalid("group title must be nonempty")); }
    }
    if let Some(color) = params.get("color") {
        if !["grey", "blue", "red", "yellow", "green", "pink", "purple", "cyan", "orange"].iter().any(|c| color.as_str() == Some(c)) {
            return Err(invalid("unsupported tab group color"));
        }
    }
    for key in ["collapsed", "active", "new_window"] {
        if params.get(key).is_some_and(|v| !v.is_boolean()) { return Err(invalid("collapsed and active must be booleans")); }
    }
    if method == "open_tab" {
        let url = string("url").ok_or_else(|| invalid("http(s) url required"))?;
        if !(url.starts_with("http://") || url.starts_with("https://")) { return Err(invalid("http(s) url required")); }
        if params.get("session_name").is_some() && params.get("group_id").is_some() { return Err(invalid("session_name and group_id are mutually exclusive")); }
        if params.get("new_window").and_then(Value::as_bool) == Some(true) && params.get("group_id").is_some() { return Err(invalid("new_window and group_id are mutually exclusive")); }
        if params.get("group_id").is_some() && !valid_id("group_id") { return Err(invalid("invalid group_id")); }
    }
    Ok(())
}

fn json_param_tab_id(params: &Value) -> Option<String> {
    params
        .get("tab_id")
        .and_then(|id| {
            id.as_str()
                .map(|s| s.to_string())
                .or_else(|| id.as_i64().map(|n| n.to_string()))
                .or_else(|| id.as_u64().map(|n| n.to_string()))
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

async fn resolve_tab_command_kind(
    state: &AppState,
    params: &Value,
) -> Result<Option<&'static str>, VcuError> {
    let want = parse_observe_browser(params.get("browser").and_then(Value::as_str))?;
    let Some(tab_id) = json_param_tab_id(params) else {
        return Ok(want);
    };
    let tabs = state.extension_bridge.list_tabs_merged().await?;
    let tab = find_extension_tab(&tabs, &tab_id, want)?;
    Ok(kind_from_tab(&tab).or(want))
}

fn strip_browser_param(mut params: Value) -> Value {
    if let Some(obj) = params.as_object_mut() {
        obj.remove("browser");
    }
    params
}

async fn browser_tab_command_for_tab(
    state: Arc<AppState>,
    headers: HeaderMap,
    method: &str,
    params: Value,
) -> axum::response::Response {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if let Err(e) = validate_tab_management(method, &params) {
        return err_response(e);
    }
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(
            ErrorCode::ExtensionDisconnected,
            "USER browser extension is not polling",
        ));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return err_response(VcuError::coded(
            ErrorCode::ActionFailed,
            "extension_profile is not user; refusing Agent browser operation",
        ));
    }
    let kind = match resolve_tab_command_kind(&state, &params).await {
        Ok(k) => k,
        Err(e) => return err_response(e),
    };
    let params = strip_browser_param(params);
    match state
        .extension_bridge
        .call_timeout_hinted(method, params, 8, kind)
        .await
    {
        Ok(mut value) => {
            if !value.is_object() {
                return err_response(VcuError::coded(ErrorCode::ActionFailed, "invalid extension response"));
            }
            value["source"] = json!("extension_tabs");
            value["login_state"] = json!(true);
            value["hud"] = json!(false);
            value["os_cursor_used"] = json!(false);
            if let Some(k) = kind {
                value["browser"] = json!(k);
            }
            Json(Envelope::ok(value)).into_response()
        }
        Err(e) => err_response(e),
    }
}

async fn browser_tab_command(state: Arc<AppState>, headers: HeaderMap, method: &str, params: Value) -> axum::response::Response {
    if let Err(e) = require_auth(&headers, &state).await { return err_response(e); }
    if let Err(e) = validate_tab_management(method, &params) { return err_response(e); }
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(ErrorCode::ExtensionDisconnected, "USER browser extension is not polling"));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return err_response(VcuError::coded(ErrorCode::ActionFailed, "extension_profile is not user; refusing Agent browser operation"));
    }
    let call = extension_dom_call(&state, method, params, 8).await;
    match call {
        Ok(mut value) => {
            if !value.is_object() { return err_response(VcuError::coded(ErrorCode::ActionFailed, "invalid extension response")); }
            value["source"] = json!("extension_tabs");
            value["login_state"] = json!(true);
            value["hud"] = json!(false);
            value["os_cursor_used"] = json!(false);
            Json(Envelope::ok(value)).into_response()
        }
        Err(e) => err_response(e),
    }
}

async fn browser_tabs(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(ErrorCode::ExtensionDisconnected, "USER browser extension is not polling"));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return err_response(VcuError::coded(ErrorCode::ActionFailed, "extension_profile is not user; refusing Agent browser operation"));
    }
    match state.extension_bridge.list_tabs_merged().await {
        Ok(mut value) => {
            if !value.is_object() {
                return err_response(VcuError::coded(ErrorCode::ActionFailed, "invalid extension response"));
            }
            value["source"] = json!("extension_tabs");
            value["login_state"] = json!(true);
            value["hud"] = json!(false);
            value["os_cursor_used"] = json!(false);
            Json(Envelope::ok(value)).into_response()
        }
        Err(e) => err_response(e),
    }
}
async fn browser_select(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(params): Json<Value>) -> impl IntoResponse {
    browser_tab_command_for_tab(state, headers, "select_tab", params).await
}
async fn browser_close(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(params): Json<Value>) -> impl IntoResponse {
    browser_tab_command_for_tab(state, headers, "close_tab", params).await
}
fn json_param_tab_ids(params: &Value) -> Vec<String> {
    params
        .get("tab_ids")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|id| {
                    id.as_str()
                        .map(|s| s.to_string())
                        .or_else(|| id.as_i64().map(|n| n.to_string()))
                        .or_else(|| id.as_u64().map(|n| n.to_string()))
                })
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

async fn resolve_open_kind(
    state: &AppState,
    params: &Value,
) -> Result<Option<&'static str>, VcuError> {
    let want = parse_observe_browser(params.get("browser").and_then(Value::as_str))?;
    if want.is_some() {
        return Ok(want);
    }
    let last = state.last_observe.read().await.clone();
    Ok(last
        .as_ref()
        .and_then(|l| crate::login_state::browser_kind_from_app_id(&l.app_id)))
}

async fn resolve_tab_ids_kind(
    state: &AppState,
    params: &Value,
) -> Result<Option<&'static str>, VcuError> {
    let want = parse_observe_browser(params.get("browser").and_then(Value::as_str))?;
    let ids = json_param_tab_ids(params);
    if ids.is_empty() {
        return Ok(want);
    }
    let tabs = state.extension_bridge.list_tabs_merged().await?;
    let mut kinds: Vec<&'static str> = Vec::new();
    for id in &ids {
        match find_extension_tab(&tabs, id, want) {
            Ok(found) => {
                if let Some(k) = kind_from_tab(&found).or(want) {
                    kinds.push(k);
                }
            }
            Err(e) => {
                let msg = e.message();
                if msg.contains("ambiguous") || msg.contains("browser must be") {
                    return Err(e);
                }
            }
        }
    }
    if kinds.is_empty() {
        return Ok(want);
    }
    let first = kinds[0];
    if kinds.iter().any(|k| *k != first) {
        return Err(VcuError::coded(
            ErrorCode::InvalidInput,
            "tab_ids span Chrome and Edge; pass one --browser",
        ));
    }
    Ok(Some(first))
}

async fn browser_tab_command_hinted(
    state: Arc<AppState>,
    headers: HeaderMap,
    method: &str,
    params: Value,
    kind: Option<&'static str>,
) -> axum::response::Response {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if let Err(e) = validate_tab_management(method, &params) {
        return err_response(e);
    }
    if !state.extension_bridge.is_polling().await {
        return err_response(VcuError::coded(
            ErrorCode::ExtensionDisconnected,
            "USER browser extension is not polling",
        ));
    }
    if !state.extension_bridge.likely_user_profile().await {
        return err_response(VcuError::coded(
            ErrorCode::ActionFailed,
            "extension_profile is not user; refusing Agent browser operation",
        ));
    }
    let params = strip_browser_param(params);
    match state
        .extension_bridge
        .call_timeout_hinted(method, params, 8, kind)
        .await
    {
        Ok(mut value) => {
            if !value.is_object() {
                return err_response(VcuError::coded(ErrorCode::ActionFailed, "invalid extension response"));
            }
            value["source"] = json!("extension_tabs");
            value["login_state"] = json!(true);
            value["hud"] = json!(false);
            value["os_cursor_used"] = json!(false);
            if let Some(k) = kind {
                value["browser"] = json!(k);
            }
            Json(Envelope::ok(value)).into_response()
        }
        Err(e) => err_response(e),
    }
}

async fn browser_group(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(params): Json<Value>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if let Err(e) = validate_tab_management("group_tabs", &params) {
        return err_response(e);
    }
    if let Err(e) = user_extension_ready(&state).await {
        return err_response(e);
    }
    let kind = match resolve_tab_ids_kind(&state, &params).await {
        Ok(k) => k,
        Err(e) => return err_response(e),
    };
    browser_tab_command_hinted(state, headers, "group_tabs", params, kind).await
}
fn json_group_id(group: &Value) -> Option<String> {
    group
        .get("group_id")
        .and_then(|id| {
            id.as_str()
                .map(|s| s.to_string())
                .or_else(|| id.as_i64().map(|n| n.to_string()))
                .or_else(|| id.as_u64().map(|n| n.to_string()))
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn find_extension_group(
    listed: &Value,
    want: &str,
    browser: Option<&str>,
) -> Result<Value, VcuError> {
    let arr = listed
        .get("groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let hits: Vec<Value> = arr
        .iter()
        .filter(|g| json_group_id(g).as_deref() == Some(want))
        .filter(|g| browser.map(|b| kind_from_tab(g) == Some(b)).unwrap_or(true))
        .cloned()
        .collect();
    if hits.len() > 1 {
        return Err(VcuError::coded(
            ErrorCode::InvalidInput,
            "group_id is ambiguous across Chrome/Edge; pass --browser",
        ));
    }
    hits.into_iter()
        .next()
        .ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "group not found"))
}

async fn resolve_group_kind(
    state: &AppState,
    params: &Value,
) -> Result<Option<&'static str>, VcuError> {
    let want = parse_observe_browser(params.get("browser").and_then(Value::as_str))?;
    let Some(gid) = params
        .get("group_id")
        .and_then(|id| {
            id.as_str()
                .map(|s| s.to_string())
                .or_else(|| id.as_i64().map(|n| n.to_string()))
                .or_else(|| id.as_u64().map(|n| n.to_string()))
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    else {
        return Ok(want);
    };
    let listed = state.extension_bridge.list_tabs_merged().await?;
    match find_extension_group(&listed, &gid, want) {
        Ok(found) => Ok(kind_from_tab(&found).or(want)),
        Err(e) => {
            let msg = e.message();
            if msg.contains("ambiguous") || msg.contains("browser must be") {
                return Err(e);
            }
            if want.is_some() {
                return Ok(want);
            }
            let last = state.last_observe.read().await.clone();
            Ok(last
                .as_ref()
                .and_then(|l| crate::login_state::browser_kind_from_app_id(&l.app_id)))
        }
    }
}

async fn browser_group_update(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(params): Json<Value>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if let Err(e) = validate_tab_management("update_group", &params) {
        return err_response(e);
    }
    if let Err(e) = user_extension_ready(&state).await {
        return err_response(e);
    }
    let kind = match resolve_group_kind(&state, &params).await {
        Ok(k) => k,
        Err(e) => return err_response(e),
    };
    browser_tab_command_hinted(state, headers, "update_group", params, kind).await
}
async fn browser_ungroup(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(params): Json<Value>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if let Err(e) = validate_tab_management("ungroup_tabs", &params) {
        return err_response(e);
    }
    if let Err(e) = user_extension_ready(&state).await {
        return err_response(e);
    }
    let kind = match resolve_tab_ids_kind(&state, &params).await {
        Ok(k) => k,
        Err(e) => return err_response(e),
    };
    browser_tab_command_hinted(state, headers, "ungroup_tabs", params, kind).await
}
async fn browser_open(State(state): State<Arc<AppState>>, headers: HeaderMap, Json(params): Json<Value>) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    if let Err(e) = validate_tab_management("open_tab", &params) {
        return err_response(e);
    }
    if let Err(e) = user_extension_ready(&state).await {
        return err_response(e);
    }
    let kind = match resolve_open_kind(&state, &params).await {
        Ok(k) => k,
        Err(e) => return err_response(e),
    };
    browser_tab_command_hinted(state, headers, "open_tab", params, kind).await
}

async fn browser_install_lens(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    match crate::lens::install_user_lens(&state.paths) {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => err_response(e),
    }
}

async fn browser_login_state(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut report = crate::login_state::inspect_login_browsers();
    if let Some(u) = report.user_browsers.first() {
        let id = format!("proc:{}:{}", u.name.replace(' ', "_"), u.pid);
        #[cfg(target_os = "macos")]
        let blob = crate::app::macos::login_debug_ui_blob(&id);
        #[cfg(not(target_os = "macos"))]
        let blob = String::new();
        let (auto, allow) = crate::login_state::classify_debug_ui(&blob);
        report.automation_infobar = auto;
        report.allow_dialog_visible = allow;
        if allow {
            report.cdp_handshake = "allow_dialog";
        }
    }
    if state.extension_bridge.is_polling().await
        || state.extension_bridge.likely_user_profile().await
    {
        report.extension_profile = "user";
    }
    // CDP abandoned: never pass allow_dialog into next_action.
    let mut sw_stale = false;
    if state.extension_bridge.is_polling().await {
        match state.extension_bridge.call_timeout("ping", json!({}), 2).await {
            Ok(_) => {}
            Err(e) => {
                sw_stale = crate::login_state::extension_error_is_stale(
                    &e.message(),
                    e.detail().as_deref(),
                );
            }
        }
    }
    report.next_action = crate::login_state::login_next_action(
        report.user_browsers.is_empty() && report.extension_profile != "user",
        report.lens_copied,
        report.extension_profile,
        false,
        &report.lens_dir,
        sw_stale,
    );
    Json(Envelope::ok(json!({
        "preferred_path": report.preferred_path,
        "user_browsers": report.user_browsers,
        "agent_browsers": report.agent_browsers,
        "cdp_listening": report.cdp_listening,
        "cdp_note": report.cdp_note,
        "extension_profile": report.extension_profile,
        "automation_infobar": report.automation_infobar,
        "allow_dialog_visible": report.allow_dialog_visible,
        "cdp_handshake": report.cdp_handshake,
        "host_vision": report.host_vision,
        "never": report.never,
        "lens_copied": report.lens_copied,
        "lens_dir": report.lens_dir,
        "next_action": report.next_action,
        "never_click_allow": report.never_click_allow,
        "never_os_cursor": report.never_os_cursor,
        "never_wechat": report.never_wechat,
        "extension_sw_stale": sw_stale,
    }))).into_response()
}

#[derive(Debug, Deserialize)]
pub struct StartSessionReq {
    #[serde(default = "default_browser")]
    pub browser: String,
    #[serde(default = "default_backend")]
    pub backend: String,
    pub vision_policy: Option<String>,
    pub surface: Option<String>,
    /// Desktop window id (proc:Name:pid). When set, that tab is activated.
    pub app_id: Option<String>,
}

fn default_browser() -> String {
    "auto".into()
}
fn default_backend() -> String {
    "mock".into()
}

async fn session_start(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<StartSessionReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    match session_start_inner(state.clone(), req).await {
        Ok(v) => Json(Envelope::ok_rev(v.0, v.1)).into_response(),
        Err(e) => err_response(e),
    }
}

fn parse_surface(req: &StartSessionReq) -> Result<SurfaceKind, VcuError> {
    match req.surface.as_deref() {
        Some("desktop") => Ok(SurfaceKind::Desktop),
        Some("browser_agent") => Ok(SurfaceKind::BrowserAgent),
        Some(other) => Err(VcuError::coded(
            ErrorCode::InvalidInput,
            format!("unknown surface: {other} (desktop|browser_agent)"),
        )),
        None => {
            if req.backend == "desktop" {
                Ok(SurfaceKind::Desktop)
            } else {
                Ok(SurfaceKind::BrowserAgent)
            }
        }
    }
}

async fn session_start_inner(
    state: Arc<AppState>,
    req: StartSessionReq,
) -> Result<(Session, u64), VcuError> {
    let cfg = state.config.read().await.clone();
    let surface = parse_surface(&req)?;
    let backend_name = if matches!(surface, SurfaceKind::Desktop) {
        "desktop"
    } else {
        req.backend.as_str()
    };
    let mut backend = if matches!(surface, SurfaceKind::Desktop) {
        Box::new(
            crate::browser::desktop::DesktopBackend::new(state.app_backend.clone()).await?,
        ) as Box<dyn crate::browser::BrowserBackend>
    } else {
        create_backend(backend_name, &cfg, &state.extension_bridge).await?
    };
    let agent_window = backend.ensure_agent_window().await?;
    let mut tabs = backend.list_tabs().await?;
    if !tabs.iter().any(|t| t.agent_owned) {
        tokio::time::sleep(Duration::from_millis(400)).await;
        tabs = backend.list_tabs().await?;
    }
    let active = crate::login_state::pick_login_tab(
        &tabs,
        &req.browser,
        req.app_id.as_deref(),
    )
    .or_else(|| tabs.iter().find(|t| t.agent_owned).map(|t| t.tab_id.clone()))
    .or_else(|| tabs.first().map(|t| t.tab_id.clone()));

    let browser = match req.browser.as_str() {
        "chrome" => BrowserKind::Chrome,
        "edge" => BrowserKind::Edge,
        "mock" => BrowserKind::Mock,
        _ => BrowserKind::Auto,
    };
    let backend_kind = match backend_name {
        "cdp" => BackendKind::Cdp,
        "extension" => BackendKind::Extension,
        "desktop" => BackendKind::Desktop,
        _ => BackendKind::Mock,
    };
    let vision_policy = req
        .vision_policy
        .as_deref()
        .and_then(VisionPolicy::parse)
        .unwrap_or(cfg.vision_policy);

    let session_id = new_id();
    let session = Session {
        session_id: session_id.clone(),
        adapter: if matches!(surface, SurfaceKind::Desktop) {
            AdapterKind::Desktop
        } else {
            AdapterKind::Browser
        },
        browser,
        backend: backend_kind,
        surface,
        policy: SessionPolicy::default(),
        vision_policy,
        created_at: Utc::now(),
        closed_at: None,
        revision: 1,
        active_tab_id: active.clone(),
        agent_window_id: Some(agent_window),
        active_app_id: if matches!(surface, SurfaceKind::Desktop) {
            active.clone()
        } else {
            None
        },
        stage_hud: backend.stage_hud().map(|(shown, _)| shown),
        stage_presenter: backend.stage_hud().map(|(_, p)| p.to_string()),
    };
    let blackboard = Blackboard {
        session_id: session_id.clone(),
        revision: 1,
        ..Default::default()
    };
    let rev = session.revision;
    let abort_path = backend.abort_watch_path();
    state.sessions.write().await.insert(
        session_id.clone(),
        SessionSlot {
            session: session.clone(),
            backend,
            blackboard,
            idempotency: Default::default(),
            borrows: Default::default(),
        },
    );
    if let Some(path) = abort_path {
        spawn_stage_abort_watch(state.clone(), session_id.clone(), path);
    }
    Ok((session, rev))
}

fn spawn_stage_abort_watch(state: Arc<AppState>, session_id: String, path: std::path::PathBuf) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(50)).await;
            if path.is_file() {
                let mut sessions = state.sessions.write().await;
                if sessions.remove(&session_id).is_some() {
                    tracing::info!(session_id, "desktop session aborted by Stage Esc");
                }
                let _ = fs::remove_file(&path);
                break;
            }
            if !state.sessions.read().await.contains_key(&session_id) {
                break;
            }
        }
    });
}

async fn session_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let sessions = state.sessions.read().await;
    let list: Vec<_> = sessions.values().map(|s| s.session.clone()).collect();
    Json(Envelope::ok(list)).into_response()
}

async fn session_show(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let sessions = state.sessions.read().await;
    match sessions.get(&id) {
        Some(s) => Json(Envelope::ok_rev(s.session.clone(), s.session.revision)).into_response(),
        None => err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        )),
    }
}

async fn session_stop(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    match sessions.remove(&id) {
        Some(mut slot) => {
            // return all borrows
            slot.borrows.clear();
            slot.session.closed_at = Some(Utc::now());
            slot.session.revision += 1;
            Json(Envelope::ok(json!({"stopped": id, "returned_borrows": true}))).into_response()
        }
        None => err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        )),
    }
}

async fn session_abort(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    match sessions.remove(&id) {
        Some(slot) => {
            let signaled = slot.backend.request_stage_abort().is_ok();
            Json(Envelope::ok(json!({
                "aborted": true,
                "session_id": id,
                "stage_signaled": signaled,
                "hud": false
            }))).into_response()
        }
        None => err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        )),
    }
}

async fn session_checkpoint(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        ));
    };
    slot.session.revision += 1;
    slot.blackboard.revision = slot.session.revision;
    // persist blackboard
    let dir = state.paths.sessions_dir().join(&id);
    let _ = fs::create_dir_all(&dir);
    let path = dir.join("blackboard.json");
    let _ = fs::write(
        &path,
        serde_json::to_string_pretty(&slot.blackboard).unwrap_or_default(),
    );
    Json(Envelope::ok_rev(
        json!({"path": path.display().to_string(), "blackboard": slot.blackboard}),
        slot.session.revision,
    ))
    .into_response()
}

#[derive(Deserialize)]
struct HelpReq {
    reason: String,
}

async fn session_request_help(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<HelpReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        ));
    };
    slot.blackboard
        .open_loops
        .push(format!("human_help:{}", req.reason));
    slot.session.revision += 1;
    slot.blackboard.revision = slot.session.revision;
    Json(Envelope::ok_rev(
        json!({"status": "waiting_human", "reason": req.reason}),
        slot.session.revision,
    ))
    .into_response()
}

async fn tabs_list(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        ));
    };
    match slot.backend.list_tabs().await {
        Ok(mut tabs) => {
            for t in &mut tabs {
                if slot.borrows.get(&t.tab_id).copied().unwrap_or(false) {
                    t.borrowed_by = Some(id.clone());
                }
            }
            Json(Envelope::ok(tabs)).into_response()
        }
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct TabReq {
    tab_id: String,
}

async fn tabs_borrow(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<TabReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    // ensure no other session holds it
    for (sid, other) in sessions.iter() {
        if sid != &id && other.borrows.get(&req.tab_id).copied().unwrap_or(false) {
            return err_response(VcuError::coded(
                ErrorCode::TabAlreadyBorrowed,
                format!("tab {} held by {sid}", req.tab_id),
            ));
        }
    }
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        ));
    };
    let tabs = match slot.backend.list_tabs().await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    if !tabs.iter().any(|t| t.tab_id == req.tab_id) {
        return err_response(VcuError::coded(
            ErrorCode::TabNotFound,
            format!("tab {}", req.tab_id),
        ));
    }
    slot.borrows.insert(req.tab_id.clone(), true);
    slot.session.revision += 1;
    Json(Envelope::ok_rev(
        json!({"borrowed": req.tab_id}),
        slot.session.revision,
    ))
    .into_response()
}

async fn tabs_return(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<serde_json::Value>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(
            ErrorCode::SessionNotFound,
            format!("session {id}"),
        ));
    };
    if let Some(tab) = req.get("tab_id").and_then(|v| v.as_str()) {
        slot.borrows.remove(tab);
    } else {
        slot.borrows.clear();
    }
    slot.session.revision += 1;
    Json(Envelope::ok_rev(json!({"returned": true}), slot.session.revision)).into_response()
}

fn ensure_writable(slot: &SessionSlot, tab_id: &str) -> Result<(), VcuError> {
    // load tab meta from last known - check backend list is async; use borrows + session
    // Caller should pass agent ownership via helper
    let _ = slot;
    let _ = tab_id;
    Ok(())
}

fn desktop_tab_ids_equiv(listed: &str, requested: &str) -> bool {
    if listed == requested || listed.eq_ignore_ascii_case(requested) {
        return true;
    }
    let Some(p_listed) = listed.rsplit(':').next() else {
        return false;
    };
    let Some(p_req) = requested.rsplit(':').next() else {
        return false;
    };
    listed.starts_with("win:")
        && requested.starts_with("win:")
        && p_listed == p_req
        && p_listed.chars().all(|c| c.is_ascii_digit())
}

async fn resolve_tab(slot: &mut SessionSlot, tab_id: Option<String>) -> Result<String, VcuError> {
    if let Some(t) = tab_id {
        return Ok(t);
    }
    let tabs = slot.backend.list_tabs().await?;
    // Prefer the session's active tab/app. The first agent-owned window is often
    // USER Edge on a real Mac, which made desktop `type` miss TextEdit (CU-D-023).
    if let Some(active) = slot.session.active_tab_id.clone() {
        if tabs.iter().any(|t| t.tab_id == active) {
            return Ok(active);
        }
    }
    if let Some(t) = tabs.iter().find(|t| t.agent_owned) {
        return Ok(t.tab_id.clone());
    }
    tabs.into_iter()
        .next()
        .map(|t| t.tab_id)
        .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, "no active tab"))
}

async fn ensure_tab_writable(slot: &mut SessionSlot, tab_id: &str) -> Result<(), VcuError> {
    let tabs = slot.backend.list_tabs().await?;
    let tab = tabs
        .iter()
        .find(|t| desktop_tab_ids_equiv(&t.tab_id, tab_id))
        .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))?;
    if tab.agent_owned {
        return Ok(());
    }
    if slot.session.policy.borrow_required_for_user_tabs
        && !slot.borrows.get(tab_id).copied().unwrap_or(false)
    {
        return Err(VcuError::coded(
            ErrorCode::BorrowRequired,
            format!("user tab {tab_id} is not borrowed"),
        ));
    }
    Ok(())
}

#[derive(Deserialize)]
struct NavigateReq {
    url: String,
    tab_id: Option<String>,
}

async fn navigate(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<NavigateReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    let tab = match resolve_tab(slot, req.tab_id).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    if let Err(e) = ensure_tab_writable(slot, &tab).await {
        return err_response(e);
    }
    if let Err(e) = slot.backend.navigate(&tab, &req.url).await {
        return err_response(e);
    }
    slot.session.active_tab_id = Some(tab.clone());
    slot.session.revision += 1;
    Json(Envelope::ok_rev(
        json!({"tab_id": tab, "url": req.url}),
        slot.session.revision,
    ))
    .into_response()
}

#[derive(Deserialize)]
struct SnapshotReq {
    #[serde(default = "default_mode")]
    mode: String,
    #[serde(default = "default_budget")]
    budget: u64,
    tab_id: Option<String>,
    #[serde(default)]
    force_vision: bool,
}

fn default_mode() -> String {
    "a11y".into()
}
fn default_budget() -> u64 {
    4000
}

async fn snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<SnapshotReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    let mode = match SnapshotMode::parse(&req.mode) {
        Some(m) => m,
        None => {
            return err_response(VcuError::coded(
                ErrorCode::InvalidInput,
                format!("bad mode {}", req.mode),
            ))
        }
    };
    let tab = match resolve_tab(slot, req.tab_id).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    // snapshot is allowed without borrow for user tabs (read-only) if policy allows - MVP: allow read
    let snap = match slot.backend.snapshot(&tab, mode, req.budget).await {
        Ok(s) => s,
        Err(e) => return err_response(e),
    };
    let tabs = slot.backend.list_tabs().await.unwrap_or_default();
    let targets: Vec<TabInfo> = tabs.into_iter().filter(|t| t.tab_id == tab).collect();

    let mut vision = VisionInfo {
        used: false,
        provider: None,
        summary: None,
    };
    let dom_empty = snap.dom_refs.is_empty();
    let need = match VisionService::should_use_vision(slot.session.vision_policy, req.force_vision, dom_empty)
    {
        Ok(v) => v,
        Err(e) => return err_response(e),
    };
    if need {
        if matches!(slot.session.vision_policy, VisionPolicy::DomOnly) && req.force_vision {
            return err_response(VcuError::coded(
                ErrorCode::VisionProviderRequired,
                "vision_policy=dom_only rejects force_vision",
            ));
        }
        let cfg = state.config.read().await.clone();
        match VisionService::resolve_model(&cfg) {
            Ok(model) => {
                if let Some(png) = &snap.screenshot_png {
                    match VisionService::describe_image(
                        model,
                        png,
                        "Summarize the UI elements and likely next actions in <=80 words.",
                    )
                    .await
                    {
                        Ok(summary) => {
                            vision = VisionInfo {
                                used: true,
                                provider: Some(model.name.clone()),
                                summary: Some(summary),
                            };
                        }
                        Err(e) => return err_response(e),
                    }
                } else if req.force_vision
                    || matches!(
                        slot.session.vision_policy,
                        VisionPolicy::VisionAlways | VisionPolicy::VisionFirst
                    )
                {
                    return err_response(VcuError::coded(
                        ErrorCode::VisionCallFailed,
                        "no screenshot available for vision",
                    ));
                }
            }
            Err(e) => {
                if req.force_vision
                    || matches!(
                        slot.session.vision_policy,
                        VisionPolicy::VisionAlways | VisionPolicy::VisionFirst
                    )
                {
                    return err_response(e);
                }
                // dom_first + transient empty DOM (e.g. mid-navigation): degrade softly.
                // Callers that truly need vision must pass force_vision or change policy.
                let _ = e;
            }
        }
    }

    let login_state = targets.first().and_then(|t| t.login_state);
    let browser_profile = targets.first().and_then(|t| t.browser_profile.clone());
    let obs = Observation {
        observation_id: new_id(),
        session_id: id.clone(),
        kind: if matches!(slot.session.surface, SurfaceKind::Desktop) {
            "desktop.scene".into()
        } else {
            "browser.snapshot".into()
        },
        targets,
        a11y_summary: snap.a11y_summary.clone(),
        dom_refs: snap.dom_refs.clone(),
        text_excerpt: snap.text_excerpt.clone(),
        screenshot_ref: snap.screenshot_png.as_ref().map(|b| {
            let mut h = Sha256::new();
            h.update(b);
            format!("cas://sha256/{}", hex::encode(h.finalize()))
        }),
        vision: vision.clone(),
        truncated: snap.truncated,
        budget_tokens_est: snap.budget_tokens_est,
        webview: snap.webview,
        webview_ref: snap.webview_ref.clone(),
        webview_screenshot_ref: snap.webview_png.as_ref().map(|b| {
            let mut h = Sha256::new();
            h.update(b);
            format!("cas://sha256/{}", hex::encode(h.finalize()))
        }),
        screenshot_scale: snap.screenshot_scale,
        webview_screenshot_scale: snap.webview_screenshot_scale,
        login_state,
        browser_profile,
        surface: slot.session.surface,
        source: Some(if matches!(slot.session.surface, SurfaceKind::Desktop) {
            let plat = state.app_backend.read().await.platform().to_string();
            if plat == "windows" {
                "uia_scene".into()
            } else {
                "ax_scene".into()
            }
        } else if matches!(slot.session.backend, BackendKind::Extension) {
            "extension_dom".into()
        } else {
            "mock".into()
        }),
    };
    slot.blackboard.observation_id = Some(obs.observation_id.clone());
    slot.blackboard.dom_summary = snap.a11y_summary.clone();
    slot.blackboard.vision_summary = vision.summary.clone();
    slot.blackboard.candidates = snap.dom_refs;
    slot.session.revision += 1;
    slot.blackboard.revision = slot.session.revision;
    crate::audit::append(
        &state.paths,
        "session.snapshot",
        serde_json::json!({
            "session": id,
            "tab": tab,
            "surface": slot.session.surface,
            "source": obs.source,
        }),
    );
    let rev = slot.session.revision;
    let mut body = serde_json::to_value(&obs).unwrap_or_else(|_| json!({}));
    if crate::login_state::browser_kind_from_app_id(&tab).is_some()
        && state.extension_bridge.is_polling().await
    {
        if let Ok(tabs) = state.extension_bridge.list_tabs_merged().await {
            crate::login_state::attach_extension_tabs_as_browser_tabs(&mut body, &tab, &tabs);
            if body.get("tab_id").and_then(|v| v.as_str()).is_some() {
                remember_observe(&state, &tab, &body).await;
            }
        }
    }
    Json(Envelope::ok_rev(body, rev)).into_response()
}

#[derive(Deserialize)]
struct ClickReq {
    r#ref: String,
    tab_id: Option<String>,
}

async fn click(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ClickReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    let tab = match resolve_tab(slot, req.tab_id).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    if let Err(e) = ensure_tab_writable(slot, &tab).await {
        return err_response(e);
    }
    match slot.backend.click(&tab, &req.r#ref).await {
        Ok(detail) => {
            if detail.detail.get("os_cursor_used").and_then(|v| v.as_bool()) == Some(true) {
                return err_response(VcuError::coded(
                    ErrorCode::OsCursorDenied,
                    "backend attempted OS cursor",
                ));
            }
            slot.session.revision += 1;
            crate::audit::append(&state.paths, "browser.click", json!({"session": id, "ref": req.r#ref, "tab": tab}));
            let ar = ActionResult {
                action_id: new_id(),
                session_id: id,
                r#type: "click".into(),
                ok: detail.ok,
                detail: detail.detail,
            };
            Json(Envelope::ok_rev(ar, slot.session.revision)).into_response()
        }
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct TypeReq {
    text: String,
    r#ref: Option<String>,
    tab_id: Option<String>,
}

async fn type_text(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<TypeReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    let tab = match resolve_tab(slot, req.tab_id).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    if let Err(e) = ensure_tab_writable(slot, &tab).await {
        return err_response(e);
    }
    match slot
        .backend
        .type_text(&tab, &req.text, req.r#ref.as_deref())
        .await
    {
        Ok(detail) => {
            slot.session.active_tab_id = Some(tab.clone());
            if matches!(slot.session.surface, SurfaceKind::Desktop) {
                slot.session.active_app_id = Some(tab.clone());
            }
            slot.session.revision += 1;
            let ar = ActionResult {
                action_id: new_id(),
                session_id: id,
                r#type: "type".into(),
                ok: detail.ok,
                detail: detail.detail,
            };
            Json(Envelope::ok_rev(ar, slot.session.revision)).into_response()
        }
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct ExtractReq {
    selector: String,
    tab_id: Option<String>,
}

async fn extract(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ExtractReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    let tab = match resolve_tab(slot, req.tab_id).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    match slot.backend.extract(&tab, &req.selector).await {
        Ok(mut res) => {
            res.session_id = id;
            Json(Envelope::ok(res)).into_response()
        }
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize)]
struct ShotReq {
    #[serde(default)]
    full_page: bool,
    tab_id: Option<String>,
    out: Option<String>,
}

async fn screenshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ShotReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    let tab = match resolve_tab(slot, req.tab_id).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    match slot.backend.screenshot(&tab, req.full_page).await {
        Ok(shot) => {
            let mut hasher = Sha256::new();
            hasher.update(&shot.png);
            let digest = hex::encode(hasher.finalize());
            let path = if let Some(out) = req.out {
                std::path::PathBuf::from(out)
            } else {
                state.paths.captures_dir().join(format!("{digest}.png"))
            };
            let _ = fs::create_dir_all(path.parent().unwrap_or(std::path::Path::new(".")));
            if let Err(e) = fs::write(&path, &shot.png) {
                return err_response(VcuError::with_detail(
                    ErrorCode::Internal,
                    "write screenshot",
                    e.to_string(),
                ));
            }
            let mut body = json!({
                "path": path.display().to_string(),
                "sha256": digest,
                "bytes": shot.png.len(),
                "width": shot.width,
                "height": shot.height,
                "webview": shot.webview_png.is_some(),
            });
            if let Some(frame) = shot.frame {
                body["frame"] = json!(frame);
            }
            if let Some(scale) = shot.scale {
                body["scale"] = json!(scale);
            }
            if let Some(r) = &shot.webview_ref {
                body["webview_ref"] = json!(r);
            }
            if let Some(frame) = shot.webview_frame {
                body["webview_frame"] = json!(frame);
            }
            if let Some(scale) = shot.webview_scale {
                body["webview_scale"] = json!(scale);
            }
            if let Some(png) = &shot.webview_png {
                let mut hasher = Sha256::new();
                hasher.update(png);
                let wv_digest = hex::encode(hasher.finalize());
                let wv_path = state.paths.captures_dir().join(format!("{wv_digest}.png"));
                let _ = fs::create_dir_all(state.paths.captures_dir());
                if fs::write(&wv_path, png).is_ok() {
                    body["webview_path"] = json!(wv_path.display().to_string());
                    body["webview_sha256"] = json!(wv_digest);
                    body["webview_bytes"] = json!(png.len());
                }
            }
            stamp_vision_handoff(&mut body);
            Json(Envelope::ok(body)).into_response()
        }
        Err(e) => err_response(e),
    }
}

async fn act(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(req): Json<ActionRequest>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut sessions = state.sessions.write().await;
    let Some(slot) = sessions.get_mut(&id) else {
        return err_response(VcuError::coded(ErrorCode::SessionNotFound, id));
    };
    if let Some(key) = &req.idempotency_key {
        if let Some(prev) = slot.idempotency.get(key) {
            return Json(Envelope::ok(prev.clone())).into_response();
        }
    }
    let tab_hint = req
        .target
        .get("tab_id")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| req.args.get("tab_id").and_then(|v| v.as_str()).map(|s| s.to_string()));
    let tab = match resolve_tab(slot, tab_hint).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    slot.session.active_tab_id = Some(tab.clone());
    if let Err(e) = ensure_tab_writable(slot, &tab).await {
        return err_response(e);
    }
    match slot.backend.act(&tab, &req).await {
        Ok(detail) => {
            slot.session.revision += 1;
            crate::audit::append(
                &state.paths,
                "session.act",
                serde_json::json!({
                    "session": id,
                    "tab": tab,
                    "type": req.r#type,
                    "surface": slot.session.surface,
                    "ok": detail.ok,
                    "source": detail.detail.get("input_path"),
                }),
            );
            let ar = ActionResult {
                action_id: new_id(),
                session_id: id,
                r#type: req.r#type.clone(),
                ok: detail.ok,
                detail: detail.detail,
            };
            let val = serde_json::to_value(&ar).unwrap_or_default();
            if let Some(key) = &req.idempotency_key {
                slot.idempotency.insert(key.clone(), val.clone());
            }
            Json(Envelope::ok_rev(ar, slot.session.revision)).into_response()
        }
        Err(e) => err_response(e),
    }
}

async fn blackboard_get(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let sessions = state.sessions.read().await;
    match sessions.get(&id) {
        Some(s) => Json(Envelope::ok(s.blackboard.clone())).into_response(),
        None => err_response(VcuError::coded(ErrorCode::SessionNotFound, id)),
    }
}

#[derive(Deserialize)]
struct ModelTestReq {
    name: String,
}

async fn model_test(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ModelTestReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let cfg = state.config.read().await.clone();
    let Some(model) = cfg.models.get(&req.name) else {
        return err_response(VcuError::coded(
            ErrorCode::ModelNotFound,
            format!("model {}", req.name),
        ));
    };
    match VisionService::test_model(model).await {
        Ok(text) => Json(Envelope::ok(json!({"reply": text}))).into_response(),
        Err(e) => err_response(e),
    }
}

// silence unused warning helper
#[allow(dead_code)]
fn _ensure(slot: &SessionSlot, tab: &str) {
    let _ = ensure_writable(slot, tab);
}


#[derive(Deserialize)]
struct ExtPollQuery {
    #[serde(default = "default_wait")]
    wait_ms: u64,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
}
fn default_wait() -> u64 { 5000 }

#[derive(Debug, Default, Deserialize)]
struct ExtBootstrapReq {
    #[serde(default)]
    client: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

fn require_loopback(addr: SocketAddr) -> Result<(), VcuError> {
    if addr.ip().is_loopback() {
        Ok(())
    } else {
        Err(VcuError::coded(
            ErrorCode::DaemonAuthFailed,
            "extension bootstrap is only allowed from loopback",
        ))
    }
}

fn require_extension_origin(headers: &HeaderMap) -> Result<(), VcuError> {
    match headers
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
    {
        None => Ok(()),
        Some(o)
            if o.starts_with("chrome-extension://")
                || o.starts_with("moz-extension://")
                || o.starts_with("safari-web-extension://") =>
        {
            Ok(())
        }
        Some(_) => Err(VcuError::coded(
            ErrorCode::DaemonAuthFailed,
            "extension bootstrap origin is not a browser extension",
        )),
    }
}

async fn extension_bootstrap(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ExtBootstrapReq>,
) -> impl IntoResponse {
    if let Err(e) = require_loopback(addr) {
        return err_response(e);
    }
    if let Err(e) = require_extension_origin(&headers) {
        return err_response(e);
    }
    let cfg = state.config.read().await.clone();
    let endpoint = VcuPaths::endpoint_url(&cfg);
    let connected = state.extension_bridge.is_connected().await;
    Json(Envelope::ok(json!({
        "token": cfg.pairing_token,
        "endpoint": endpoint,
        "host": cfg.daemon_host,
        "port": cfg.daemon_port,
        "version": state.version,
        "already_connected": connected,
        "client": req.client.unwrap_or_else(|| "unknown".into()),
        "accepted_version": req.version.unwrap_or_default(),
    }))).into_response()
}

#[derive(Deserialize, Default)]
struct ExtHelloReq {
    #[serde(default)]
    likely_user_profile: bool,
    #[serde(default)]
    hosts: Vec<String>,
    #[serde(default)]
    client_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
    #[serde(default)]
    command_pull: bool,
}

async fn extension_hello(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ExtHelloReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let _ = req.hosts;
    state.extension_bridge.mark_hello_client(req.likely_user_profile, req.client_id.clone(), req.browser.clone()).await;
    let command = if req.command_pull {
        state
            .extension_bridge
            .poll_for(4000, req.client_id, req.browser)
            .await
    } else {
        None
    };
    Json(Envelope::ok(json!({"connected": true, "command": command}))).into_response()
}

async fn extension_poll(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ExtPollQuery>,
) -> impl IntoResponse {
    extension_poll_inner(state, headers, q).await
}

async fn extension_poll_post(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(q): Json<ExtPollQuery>,
) -> impl IntoResponse {
    extension_poll_inner(state, headers, q).await
}

async fn extension_poll_inner(
    state: Arc<AppState>,
    headers: HeaderMap,
    q: ExtPollQuery,
) -> axum::response::Response {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    match state.extension_bridge.poll_for(q.wait_ms, q.client_id, q.browser).await {
        Some(cmd) => Json(Envelope::ok(cmd)).into_response(),
        None => Json(Envelope::ok(json!({"empty": true}))).into_response(),
    }
}

#[derive(Deserialize)]
struct ExtResultReq {
    id: String,
    result: serde_json::Value,
}

async fn extension_result(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<ExtResultReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    state.extension_bridge.submit_result(&req.id, req.result).await;
    Json(Envelope::ok(json!({"accepted": true}))).into_response()
}


async fn app_windows(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    // Hot-reload allowlist only for platform backends (do not clobber test mocks).
    {
        let cfg = state.config.read().await;
        let allow = cfg.app_allowlist.clone();
        let mut backend = state.app_backend.write().await;
        if backend.platform() == "macos" || backend.platform() == "windows" {
            *backend = crate::app::detect_app_backend_with_allowlist(
                if allow.is_empty() { None } else { Some(allow) },
            );
        }
    }
    let backend = state.app_backend.read().await;
    match backend.list_windows().await {
        Ok(v) => Json(Envelope::ok(json!({
            "platform": backend.platform(),
            "windows": v
        }))).into_response(),
        Err(e) => err_response(e),
    }
}


fn stamp_vision_handoff(body: &mut Value) {
    const KEYS: &[&str] = &[
        "screenshot_path",
        "webview_screenshot_path",
        "login_latest_png",
        "feishu_latest_png",
        "path",
        "webview_path",
    ];
    let mut must = Vec::new();
    for key in KEYS {
        if let Some(p) = body.get(*key).and_then(|v| v.as_str()) {
            if !p.is_empty() && !must.iter().any(|x: &String| x == p) {
                must.push(p.to_string());
            }
        }
    }
    if must.is_empty() {
        return;
    }
    body["vision_handoff"] = json!({
        "must_view": must,
        "serial": true,
        "rule": "Host vision must view these PNGs in the same turn before click/type. Do not ask the user what is on screen."
    });
}

fn attach_capture(body: &mut Value, dir: &std::path::Path, cap: &AppCapture, prefix: &str) {
    let mut hasher = Sha256::new();
    hasher.update(&cap.png);
    let digest = hex::encode(hasher.finalize());
    let path = dir.join(format!("{digest}.png"));
    let _ = fs::create_dir_all(dir);
    if fs::write(&path, &cap.png).is_ok() {
        body[format!("{prefix}screenshot_path")] = json!(path.display().to_string());
        body[format!("{prefix}screenshot_sha256")] = json!(digest);
        body[format!("{prefix}screenshot_bytes")] = json!(cap.png.len());
        body[format!("{prefix}screenshot_frame")] = json!(cap.frame);
        body[format!("{prefix}screenshot_width")] = json!(cap.width);
        body[format!("{prefix}screenshot_height")] = json!(cap.height);
        if let Some(scale) = pixel_scale(cap.width, cap.height, cap.frame) {
            body[format!("{prefix}screenshot_scale")] = json!(scale);
        }
    }
}

#[derive(Deserialize)]
struct AppSnapshotReq {
    id: String,
    #[serde(default = "default_app_budget")]
    budget: u64,
    #[serde(default)]
    pixels: bool,
    /// Optional Scene extract without raising Stage HUD.
    selector: Option<String>,
}
fn default_app_budget() -> u64 { 4000 }

async fn snapshot_app_json(state: &AppState, req: &AppSnapshotReq) -> Result<Value, VcuError> {
    let backend = state.app_backend.read().await;
    match backend.snapshot(&req.id, req.budget).await {
        Ok(snap) => {
            let mut body = serde_json::to_value(&snap).unwrap_or_else(|_| json!({}));
            if let Some(sel) = req.selector.as_deref() {
                let matches = extract_elements(&snap.elements, sel);
                body["extract"] = json!({
                    "selector": sel,
                    "count": matches.len(),
                    "matches": matches,
                    "hud": false
                });
            }
            if req.pixels {
                match backend.capture_window(&req.id).await {
                    Ok(Some(cap)) => {
                        attach_capture(&mut body, &state.paths.captures_dir(), &cap, "");
                    }
                    Ok(None) => {
                        body["screenshot_skipped"] = json!(true);
                    }
                    Err(e) => return Err(e),
                }
                if let Some(frame) = webview_crop_frame(
                    snap.elements.as_slice(),
                    snap.webview_ref.as_deref(),
                ) {
                    // A raw screen crop can show an overlapping app. Browser page
                    // captures use the extension viewport screenshot instead.
                    if !req.id.contains("Edge") && !req.id.contains("Chrome") {
                    match backend.capture_rect(frame).await {
                        Ok(Some(cap)) => {
                            attach_capture(&mut body, &state.paths.captures_dir(), &cap, "webview_");
                        }
                        Ok(None) => {}
                        Err(e) => return Err(e),
                    }
                    }
                }
                if let Some(src) = body.get("screenshot_path").and_then(|v| v.as_str()).map(|s| s.to_string()) {
                    let mut wv_frame = body.get("webview_screenshot_frame").cloned();
                    let mut wv_scale = body.get("webview_screenshot_scale").and_then(|v| v.as_f64());
                    if wv_frame.is_none() {
                        if let Some(win) = json_frame(body.get("screenshot_frame")) {
                            let h = crate::app::edge_webview_heuristic(win);
                            wv_frame = Some(serde_json::json!(h));
                            wv_scale = body.get("screenshot_scale").and_then(|v| v.as_f64());
                            body["webview_screenshot_frame"] = wv_frame.clone().unwrap();
                            if let Some(sc) = wv_scale {
                                body["webview_screenshot_scale"] = json!(sc);
                            }
                            body["webview_heuristic"] = json!(true);
                        }
                    }
                    let meta = LoginLatestMeta {
                        scale: body.get("screenshot_scale").and_then(|v| v.as_f64()),
                        frame: body.get("screenshot_frame").cloned(),
                        page_title: snap.page_title.clone(),
                        page_url: snap.page_url.clone(),
                        webview_scale: wv_scale,
                        webview_frame: wv_frame,
                    };
                    if let Some(latest) = publish_login_latest(
                        &req.id,
                        std::path::Path::new(&src),
                        &state.paths.captures_dir(),
                        Some(&meta),
                    ) {
                        body["login_latest_png"] = json!(latest.display().to_string());
                        body["login_latest_json"] = json!(latest.with_extension("json").display().to_string());
                    }
                }
                if let Some(src) = body
                    .get("webview_screenshot_path")
                    .and_then(|v| v.as_str())
                    .or_else(|| body.get("screenshot_path").and_then(|v| v.as_str()))
                {
                    let meta = LoginLatestMeta {
                        scale: body
                            .get("webview_screenshot_scale")
                            .or_else(|| body.get("screenshot_scale"))
                            .and_then(|v| v.as_f64()),
                        frame: body
                            .get("webview_screenshot_frame")
                            .cloned()
                            .or_else(|| body.get("screenshot_frame").cloned()),
                        page_title: snap.page_title.clone(),
                        page_url: snap.page_url.clone(),
                        webview_scale: body.get("webview_screenshot_scale").and_then(|v| v.as_f64()),
                        webview_frame: body.get("webview_screenshot_frame").cloned(),
                    };
                    if let Some(latest) = publish_feishu_latest(
                        &req.id,
                        std::path::Path::new(src),
                        &state.paths.captures_dir(),
                        Some(&meta),
                    ) {
                        body["feishu_latest_png"] = json!(latest.display().to_string());
                        body["feishu_latest_json"] = json!(latest.with_extension("json").display().to_string());
                    }
                }
                stamp_vision_handoff(&mut body);
            }
            if crate::login_state::browser_kind_from_app_id(&req.id).is_some() {
                let mut tabs = None;
                for attempt in 0..3 {
                    if !state.extension_bridge.is_polling().await {
                        if attempt == 2 {
                            break;
                        }
                        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        continue;
                    }
                    match state.extension_bridge.list_tabs_merged().await {
                        Ok(v)
                            if v.get("tabs")
                                .and_then(|x| x.as_array())
                                .map(|a| !a.is_empty())
                                .unwrap_or(false) =>
                        {
                            tabs = Some(v);
                            break;
                        }
                        Ok(_) if attempt < 2 => {
                            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
                        }
                        _ => break,
                    }
                }
                if let Some(tabs) = tabs {
                    crate::login_state::merge_extension_tabs_into_scene(&mut body, &req.id, &tabs);
                }
            }
            Ok(body)
        }
        Err(e) => Err(e),
    }
}

async fn app_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AppSnapshotReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    match snapshot_app_json(&state, &req).await {
        Ok(body) => Json(Envelope::ok(body)).into_response(),
        Err(e) => err_response(e),
    }
}

#[derive(Deserialize, Default)]
struct BrowserObserveReq {
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    browser: Option<String>,
    #[serde(default)]
    pixels: Option<bool>,
    #[serde(default)]
    selector: Option<String>,
    #[serde(default)]
    budget: Option<u64>,
}

fn observe_envelope(id: String, user: Value, mut snap: Value, extension_profile: &str) -> Value {
    if let Some(obj) = snap.as_object_mut() {
        obj.remove("elements");
    }
    json!({
        "app_id": id,
        "login_state": true,
        "browser_profile": "user",
        "hud": false,
        "extension_profile": extension_profile,
        "page_title": snap.get("page_title").cloned().unwrap_or(Value::Null),
        "page_url": snap.get("page_url").cloned().unwrap_or(Value::Null),
        "tab_id": snap.get("tab_id").cloned().unwrap_or(Value::Null),
        "tab_id_source": snap.get("tab_id_source").cloned().unwrap_or(Value::Null),
        "browser": snap.get("browser").cloned().unwrap_or(Value::Null),
        "tabs": snap.get("tabs").cloned().unwrap_or(json!([])),
        "tabs_source": snap.get("tabs_source").cloned().unwrap_or(Value::Null),
        "page_url_source": snap.get("page_url_source").cloned().unwrap_or(Value::Null),
        "webview": snap.get("webview").cloned().unwrap_or(json!(false)),
        "ax_enhanced": snap.get("ax_enhanced").cloned().unwrap_or(json!(false)),
        "coordinate_help": "ax = frame_origin + pixel / screenshot_scale; click with --pixel-x/--pixel-y",
        "login": user,
        "snapshot": snap,
    })
}

fn json_tab_id_value(tab: &Value) -> Option<String> {
    tab.get("tab_id")
        .and_then(|id| {
            id.as_str()
                .map(|s| s.to_string())
                .or_else(|| id.as_i64().map(|n| n.to_string()))
                .or_else(|| id.as_u64().map(|n| n.to_string()))
        })
        .filter(|s| !s.is_empty())
}

fn parse_observe_browser(raw: Option<&str>) -> Result<Option<&'static str>, VcuError> {
    let Some(raw) = raw.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    match raw.to_ascii_lowercase().as_str() {
        "chrome" | "google chrome" => Ok(Some("chrome")),
        "edge" | "microsoft edge" => Ok(Some("edge")),
        _ => Err(VcuError::coded(
            ErrorCode::InvalidInput,
            "browser must be chrome or edge",
        )),
    }
}

fn find_extension_tab(tabs: &Value, want: &str, browser: Option<&str>) -> Result<Value, VcuError> {
    let arr = tabs
        .get("tabs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let hits: Vec<Value> = arr
        .iter()
        .filter(|t| json_tab_id_value(t).as_deref() == Some(want))
        .filter(|t| browser.map(|b| kind_from_tab(t) == Some(b)).unwrap_or(true))
        .cloned()
        .collect();
    if hits.is_empty() {
        return Err(VcuError::coded(ErrorCode::ActionFailed, "tab not found"));
    }
    if hits.len() > 1 {
        return Err(VcuError::coded(
            ErrorCode::InvalidInput,
            "tab_id is ambiguous across Chrome/Edge; pass --browser",
        ));
    }
    Ok(hits.into_iter().next().unwrap())
}

fn kind_from_tab(tab: &Value) -> Option<&'static str> {
    match tab.get("browser").and_then(Value::as_str) {
        Some("edge") => Some("edge"),
        Some("chrome") => Some("chrome"),
        _ => None,
    }
}

fn tab_is_visible(tab: &Value) -> bool {
    tab.get("active").and_then(Value::as_bool).unwrap_or(false)
        || tab.get("focused").and_then(Value::as_bool).unwrap_or(false)
}

fn focused_extension_tab(tabs: &Value, kind: Option<&str>) -> Option<Value> {
    let arr = tabs.get("tabs")?.as_array()?;
    let matches_kind = |t: &Value| match kind {
        Some(k) => t.get("browser").and_then(Value::as_str) == Some(k),
        None => true,
    };
    let active = |t: &Value| {
        t.get("active").and_then(Value::as_bool).unwrap_or(false)
            || t.get("focused").and_then(Value::as_bool).unwrap_or(false)
    };
    arr.iter()
        .find(|t| matches_kind(t) && active(t))
        .cloned()
        .or_else(|| arr.iter().find(|t| kind.is_some() && matches_kind(t)).cloned())
}

fn kind_from_frontmost(name: Option<&str>) -> Option<&'static str> {
    let name = name?;
    if crate::login_state::browser_name_is_frontmost("Microsoft Edge", name) {
        Some("edge")
    } else if crate::login_state::browser_name_is_frontmost("Chrome", name) {
        Some("chrome")
    } else {
        None
    }
}

async fn observe_via_lens(
    state: &AppState,
    extension_profile: &str,
    pixels: bool,
    want_tab: Option<&str>,
    want_browser: Option<&str>,
) -> Result<Value, VcuError> {
    user_extension_ready(state).await?;
    let frontmost = crate::login_state::frontmost_user_browser_name();
    let mut tabs_v = state.extension_bridge.list_tabs_merged().await?;
    let requested = want_tab.map(str::trim).filter(|s| !s.is_empty());
    let want_kind = parse_observe_browser(want_browser)?;
    let (mut tab, kind) = if let Some(want) = requested {
        let tab = find_extension_tab(&tabs_v, want, want_kind)?;
        let kind = kind_from_tab(&tab).or(want_kind).or_else(|| kind_from_frontmost(frontmost.as_deref()));
        (tab, kind)
    } else if let Some(kind) = want_kind {
        let tab = focused_extension_tab(&tabs_v, Some(kind)).ok_or_else(|| {
            VcuError::coded(
                ErrorCode::ActionFailed,
                "no focused USER tab for observe --browser; pass --tab",
            )
        })?;
        if kind_from_tab(&tab) != Some(kind) {
            return Err(VcuError::coded(
                ErrorCode::ActionFailed,
                "no focused USER tab for observe --browser; pass --tab",
            ));
        }
        (tab, Some(kind))
    } else {
        let kind = kind_from_frontmost(frontmost.as_deref()).ok_or_else(|| {
            VcuError::coded(
                ErrorCode::InvalidInput,
                "frontmost is not USER Chrome/Edge; pass observe --tab or --browser",
            )
        })?;
        let tab = focused_extension_tab(&tabs_v, Some(kind)).ok_or_else(|| {
            VcuError::coded(
                ErrorCode::ActionFailed,
                "no focused USER tab for observe; pass --tab",
            )
        })?;
        if kind_from_tab(&tab) != Some(kind) || !tab_is_visible(&tab) {
            return Err(VcuError::coded(
                ErrorCode::ActionFailed,
                "no focused USER tab for observe; pass --tab",
            ));
        }
        (tab, Some(kind))
    };
    let tab_id = json_tab_id_value(&tab).ok_or_else(|| {
        VcuError::coded(ErrorCode::ActionFailed, "observe tab missing tab_id")
    })?;
    if requested.is_some() && !tab_is_visible(&tab) {
        state
            .extension_bridge
            .call_timeout_hinted(
                "select_tab",
                json!({"tab_id": tab_id, "focus_window": false}),
                8,
                kind,
            )
            .await?;
        tabs_v = state.extension_bridge.list_tabs_merged().await?;
        if let Ok(fresh) = find_extension_tab(&tabs_v, &tab_id, kind) {
            tab = fresh;
        }
    }
    let report = crate::login_state::inspect_login_browsers();
    let users = crate::login_state::order_user_browsers_frontmost_first(
        report.user_browsers.clone(),
        frontmost.as_deref(),
    );
    let user = users.iter().find(|u| {
        let n = u.name.to_ascii_lowercase();
        match kind {
            Some("edge") => n.contains("edge"),
            Some("chrome") => n.contains("chrome") && !n.contains("edge"),
            _ => true,
        }
    });
    let user = if kind.is_some() { user } else { user.or(users.first()) };
    let app_id = user
        .map(|u| format!("proc:{}:{}", u.name.replace(' ', "_"), u.pid))
        .unwrap_or_else(|| format!("proc:{}:lens", kind.unwrap_or("Chrome")));
    let mut snap = json!({
        "tabs": tabs_v.get("tabs").cloned().unwrap_or(json!([])),
        "tab_id": tab_id,
        "tab_id_source": "extension_tabs",
        "tabs_source": "extension_tabs",
        "page_url": tab.get("url").cloned().unwrap_or(Value::Null),
        "page_url_source": "extension_tabs",
        "page_title": tab.get("title").cloned().unwrap_or(Value::Null),
        "hud": false,
        "source": "extension_viewport",
        "browser": kind,
    });
    if pixels {
        let captured = state
            .extension_bridge
            .call_timeout_hinted("capture_tab", json!({"tab_id": tab_id}), 8, kind)
            .await?;
        let encoded = captured
            .get("png_base64")
            .and_then(Value::as_str)
            .ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "extension observe screenshot missing PNG"))?;
        if encoded.len() > 24_000_000 {
            return Err(VcuError::coded(ErrorCode::ActionFailed, "extension observe screenshot too large"));
        }
        let png = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|_| VcuError::coded(ErrorCode::ActionFailed, "extension observe screenshot is not PNG"))?;
        let (width, height) = crate::app::png_ihdr_size(&png).ok_or_else(|| {
            VcuError::coded(ErrorCode::ActionFailed, "extension observe screenshot is not PNG")
        })?;
        let capture_id = new_id();
        let dir = state.paths.captures_dir();
        let path = dir.join(format!("viewport-{capture_id}.png"));
        let viewport = captured.get("viewport").cloned().unwrap_or_else(|| {
            json!({"document_id": "observe", "width": width as f64, "height": height as f64})
        });
        let metadata = json!({
            "capture_id": capture_id,
            "tab_id": tab_id,
            "viewport": viewport,
            "width": width,
            "height": height,
            "created_at_ms": Utc::now().timestamp_millis()
        });
        fs::create_dir_all(&dir)
            .and_then(|_| fs::write(&path, &png))
            .and_then(|_| fs::write(dir.join(format!("viewport-{capture_id}.json")), serde_json::to_vec(&metadata).unwrap()))
            .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "save observe screenshot", e.to_string()))?;
        snap["screenshot_path"] = json!(path.display().to_string());
        snap["screenshot_width"] = json!(width);
        snap["screenshot_height"] = json!(height);
        snap["capture_id"] = json!(capture_id);
        snap["source"] = json!("extension_viewport");
        snap["vision_handoff"] = json!({
            "must_view": [path.display().to_string()],
            "serial": true,
            "rule": "View this PNG before click/type. Use capture_id with space=viewport."
        });
        let meta = crate::app::LoginLatestMeta {
            scale: Some(if height >= 1200 { 2.0 } else { 1.0 }),
            frame: None,
            page_title: tab.get("title").and_then(Value::as_str).map(|s| s.to_string()),
            page_url: tab.get("url").and_then(Value::as_str).map(|s| s.to_string()),
            webview_scale: None,
            webview_frame: None,
        };
        if let Some(latest) = crate::app::publish_login_latest(&app_id, &path, &dir, Some(&meta)) {
            snap["login_latest_png"] = json!(latest.display().to_string());
        }
    }
    remember_observe(state, &app_id, &snap).await;
    let user_json = user
        .and_then(|u| serde_json::to_value(u).ok())
        .unwrap_or_else(|| json!({"id": app_id}));
    let mut env = observe_envelope(app_id, user_json, snap.clone(), extension_profile);
    if let Some(cid) = snap.get("capture_id") {
        env["capture_id"] = cid.clone();
        env["screenshot_path"] = snap.get("screenshot_path").cloned().unwrap_or(Value::Null);
        env["source"] = json!("extension_viewport");
        env["vision_handoff"] = snap.get("vision_handoff").cloned().unwrap_or(Value::Null);
    }
    if let Some(front) = frontmost.as_deref() {
        env["frontmost_app"] = json!(front);
        env["frontmost_matched"] = json!(crate::login_state::browser_name_is_frontmost(
            env.get("app_id").and_then(Value::as_str).unwrap_or(""),
            front,
        ));
    }
    Ok(env)
}

async fn browser_observe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<BrowserObserveReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let pixels = req.pixels.unwrap_or(true);
    let budget = req.budget.unwrap_or(2500);
    let selector = req.selector.clone().unwrap_or_else(|| "*".into());
    let report = crate::login_state::inspect_login_browsers();
    let extension_profile = if state.extension_bridge.is_polling().await
        && state.extension_bridge.likely_user_profile().await
    {
        "user"
    } else {
        report.extension_profile
    };
    let want_tab = req
        .tab_id
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());
    let lens_ready = state.extension_bridge.is_polling().await
        && state.extension_bridge.likely_user_profile().await;
    if let Some(tab) = want_tab.as_deref() {
        if !lens_ready {
            return err_response(VcuError::coded(
                ErrorCode::ExtensionDisconnected,
                "observe --tab needs USER lens polling",
            ));
        }
        return match observe_via_lens(&state, extension_profile, pixels, Some(tab), req.browser.as_deref()).await {
            Ok(env) => Json(Envelope::ok(env)).into_response(),
            Err(e) => err_response(e),
        };
    }
    if let Some(id) = req.id.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        let snap_req = AppSnapshotReq {
            id: id.to_string(),
            budget,
            pixels,
            selector: Some(selector),
        };
        return match snapshot_app_json(&state, &snap_req).await {
            Ok(snap) => {
                remember_observe(&state, id, &snap).await;
                let user = json!({"id": id});
                Json(Envelope::ok(observe_envelope(id.to_string(), user, snap, extension_profile))).into_response()
            }
            Err(e) => err_response(e),
        };
    }
    if req.id.is_none()
        && state.extension_bridge.is_polling().await
        && state.extension_bridge.likely_user_profile().await
    {
        match observe_via_lens(&state, extension_profile, pixels, None, req.browser.as_deref()).await {
            Ok(env) => return Json(Envelope::ok(env)).into_response(),
            Err(e) if e.code() == ErrorCode::InvalidInput
                || req.browser.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some() =>
            {
                return err_response(e);
            }
            Err(_) => {}
        }
    }
    if report.user_browsers.is_empty() {
        return err_response(VcuError::coded(
            ErrorCode::ActionFailed,
            "no user Chrome/Edge process; login-state observe needs the user browser window",
        ));
    }
    let frontmost = crate::login_state::frontmost_user_browser_name();
    let users = crate::login_state::order_user_browsers_frontmost_first(
        report.user_browsers.clone(),
        frontmost.as_deref(),
    );
    let mut chosen_user = None;
    let mut snap_body = json!({});
    let mut last_err: Option<VcuError> = None;
    let mut id = String::new();
    for user in &users {
        let candidate_id = format!("proc:{}:{}", user.name.replace(' ', "_"), user.pid);
        let snap_req = AppSnapshotReq {
            id: candidate_id.clone(),
            budget,
            pixels,
            selector: Some(selector.clone()),
        };
        match snapshot_app_json(&state, &snap_req).await {
            Ok(body) => {
                let has_png = crate::login_state::snapshot_has_png(&body);
                let is_front = frontmost
                    .as_deref()
                    .map(|f| crate::login_state::browser_name_is_frontmost(&user.name, f))
                    .unwrap_or(false);
                chosen_user = Some(serde_json::to_value(user).unwrap_or_else(|_| json!({})));
                snap_body = body;
                id = candidate_id;
                if has_png || is_front {
                    break;
                }
            }
            Err(e) => last_err = Some(e),
        }
    }
    let Some(user) = chosen_user else {
        return err_response(last_err.unwrap_or_else(|| {
            VcuError::coded(ErrorCode::ActionFailed, "login-state observe failed")
        }));
    };
    remember_observe(&state, &id, &snap_body).await;
    let mut env = observe_envelope(id, user, snap_body, extension_profile);
    if let Some(front) = frontmost.as_deref() {
        env["frontmost_app"] = json!(front);
        env["frontmost_matched"] = json!(crate::login_state::browser_name_is_frontmost(
            env.get("app_id").and_then(Value::as_str).unwrap_or(""),
            front,
        ));
    }
    Json(Envelope::ok(env)).into_response()
}

#[derive(Deserialize)]
struct AppInvokeReq {
    id: String,
    r#ref: String,
}

async fn app_invoke(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AppInvokeReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut backend = state.app_backend.write().await;
    match backend.invoke(&req.id, &req.r#ref).await {
        Ok(v) => {
            crate::audit::append(&state.paths, "app.invoke", json!({"id": req.id, "ref": req.r#ref, "ok": true}));
            Json(Envelope::ok(v)).into_response()
        }
        Err(e) => {
            crate::audit::append(&state.paths, "app.invoke", json!({"id": req.id, "ref": req.r#ref, "ok": false, "code": format!("{:?}", e.code())}));
            err_response(e)
        }
    }
}

#[derive(Deserialize)]
struct AppFocusReq {
    id: String,
    #[serde(default)]
    allow_focus_steal: bool,
}

async fn app_focus(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AppFocusReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let mut backend = state.app_backend.write().await;
    match backend.focus_window(&req.id, req.allow_focus_steal).await {
        Ok(()) => Json(Envelope::ok(json!({"focused": req.id}))).into_response(),
        Err(e) => err_response(e),
    }
}


#[cfg(test)]
mod abort_watch_tests {
    use super::*;
    use crate::browser::MockBackend;
    use vcu_core::{AdapterKind, BackendKind, BrowserKind, Session, SurfaceKind, UserConfig, VcuPaths, VisionPolicy};

    #[tokio::test]
    async fn abort_watch_removes_session() {
        let dir = tempfile::tempdir().unwrap();
        let paths = VcuPaths::from_root(dir.path());
        let mut cfg = UserConfig::default();
        cfg.daemon_port = 0;
        paths.save_config(&cfg).unwrap();
        let handle = crate::start_daemon(paths, cfg).await.unwrap();
        let sid = "abort-watch-sid".to_string();
        let session = Session {
            session_id: sid.clone(),
            adapter: AdapterKind::Desktop,
            browser: BrowserKind::Auto,
            backend: BackendKind::Desktop,
            surface: SurfaceKind::Desktop,
            policy: Default::default(),
            vision_policy: VisionPolicy::DomFirst,
            created_at: Utc::now(),
            closed_at: None,
            revision: 1,
            active_tab_id: None,
            agent_window_id: None,
            active_app_id: None,
            stage_hud: Some(true),
            stage_presenter: Some("noop".into()),
        };
        handle
            .state
            .sessions
            .write()
            .await
            .insert(
                sid.clone(),
                SessionSlot {
                    session,
                    backend: Box::new(MockBackend::new()),
                    blackboard: Default::default(),
                    idempotency: Default::default(),
                    borrows: Default::default(),
                },
            );
        let abort = dir.path().join("stage.abort");
        spawn_stage_abort_watch(Arc::new(handle.state.clone()), sid.clone(), abort.clone());
        std::fs::write(&abort, b"1").unwrap();
        tokio::time::sleep(Duration::from_millis(180)).await;
        assert!(handle.state.sessions.read().await.get(&sid).is_none());
        handle.join.abort();
    }
}

#[cfg(test)]
mod vision_handoff_unit_tests {
    use super::stamp_vision_handoff;
    use serde_json::json;

    #[test]
    fn stamp_vision_handoff_lists_pngs() {
        let mut body = json!({
            "screenshot_path": "/tmp/a.png",
            "webview_screenshot_path": "/tmp/b.png",
            "feishu_latest_png": "/tmp/b.png"
        });
        stamp_vision_handoff(&mut body);
        let must = body["vision_handoff"]["must_view"].as_array().unwrap();
        let s: Vec<&str> = must.iter().filter_map(|v| v.as_str()).collect();
        assert!(s.contains(&"/tmp/a.png"));
        assert!(s.contains(&"/tmp/b.png"));
        assert_eq!(s.iter().filter(|p| **p == "/tmp/b.png").count(), 1);
        assert_eq!(body["vision_handoff"]["serial"], true);
        assert!(body["vision_handoff"]["rule"].as_str().unwrap().contains("same turn"));
    }

    #[test]
    fn stamp_vision_handoff_skips_empty() {
        let mut body = json!({"hello": 1});
        stamp_vision_handoff(&mut body);
        assert!(body.get("vision_handoff").is_none());
    }
}

#[cfg(test)]
mod viewport_coordinate_tests {
    use super::*;
    #[test]
    fn model_receives_summary_while_internal_capture_keeps_geometry_binding() {
        let internal = json!({"viewport":{"layout_signature":{"visible_count":400,"overflow":false,"nodes":[{"id":"i1","left":20}]}}});
        let mut public = internal.clone();
        summarize_viewport_result(&mut public);
        assert!(public.pointer("/viewport/layout_signature/nodes").is_none());
        assert_eq!(public.pointer("/viewport/layout_signature/visible_count"), Some(&json!(400)));
        assert!(internal.pointer("/viewport/layout_signature/nodes").is_some());
    }

    #[test]
    fn screenshot_pixels_map_at_retina_and_browser_zoom_without_guessing_scale() {
        let meta = json!({"width":1600,"height":960,"viewport":{"width":1000,"height":600}});
        assert_eq!(viewport_pixel_point(Some(800.0), Some(480.0), &meta), Some((500.0,300.0)));
        for (x,y) in [(1600.0,0.0),(-1.0,0.0),(0.0,960.0),(f64::NAN,0.0),(0.0,f64::INFINITY)] {
            assert_eq!(viewport_pixel_point(Some(x),Some(y),&meta), None);
        }
    }
}
