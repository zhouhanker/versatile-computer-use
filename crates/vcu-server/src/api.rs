use std::fs;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tower_http::cors::CorsLayer;
use vcu_core::{
    new_id, ActionRequest, ActionResult, AdapterKind, BackendKind, Blackboard, BrowserKind,
    Envelope, ErrorCode, Observation, Session, SessionPolicy, SnapshotMode, TabInfo, VcuError,
    VisionInfo, VisionPolicy,
};

use crate::doctor;
use crate::state::{create_backend, AppState, SessionSlot};
use crate::vision::VisionService;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/extension/hello", post(extension_hello))
        .route("/v1/extension/poll", get(extension_poll))
        .route("/v1/extension/result", post(extension_result))
        .route("/v1/app/windows", get(app_windows))
        .route("/v1/app/snapshot", post(app_snapshot))
        .route("/v1/app/invoke", post(app_invoke))
        .route("/v1/app/focus", post(app_focus))
        .route("/v1/doctor", get(doctor_handler))
        .route("/v1/session/start", post(session_start))
        .route("/v1/session/list", get(session_list))
        .route("/v1/session/{id}", get(session_show))
        .route("/v1/session/{id}/stop", post(session_stop))
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
        .layer(CorsLayer::permissive())
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
        | ErrorCode::VisionProviderRequired => StatusCode::CONFLICT,
        ErrorCode::InvalidInput => StatusCode::BAD_REQUEST,
        _ => StatusCode::BAD_REQUEST,
    };
    let body = Envelope::<Value>::from_error(&err);
    (status, Json(body)).into_response()
}

async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(Envelope::ok(json!({
        "version": state.version,
        "started_at": state.started_at,
        "sessions": state.sessions.read().await.len(),
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

#[derive(Debug, Deserialize)]
pub struct StartSessionReq {
    #[serde(default = "default_browser")]
    pub browser: String,
    #[serde(default = "default_backend")]
    pub backend: String,
    pub vision_policy: Option<String>,
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
    match session_start_inner(&state, req).await {
        Ok(v) => Json(Envelope::ok_rev(v.0, v.1)).into_response(),
        Err(e) => err_response(e),
    }
}

async fn session_start_inner(
    state: &AppState,
    req: StartSessionReq,
) -> Result<(Session, u64), VcuError> {
    let cfg = state.config.read().await.clone();
    let backend_name = req.backend.as_str();
    let mut backend = create_backend(backend_name, &cfg, &state.extension_bridge).await?;
    let agent_window = backend.ensure_agent_window().await?;
    let tabs = backend.list_tabs().await?;
    let active = tabs
        .iter()
        .find(|t| t.agent_owned)
        .map(|t| t.tab_id.clone())
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
        adapter: AdapterKind::Browser,
        browser,
        backend: backend_kind,
        policy: SessionPolicy::default(),
        vision_policy,
        created_at: Utc::now(),
        closed_at: None,
        revision: 1,
        active_tab_id: active,
        agent_window_id: Some(agent_window),
    };
    let blackboard = Blackboard {
        session_id: session_id.clone(),
        revision: 1,
        ..Default::default()
    };
    let rev = session.revision;
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
    Ok((session, rev))
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

async fn resolve_tab(slot: &mut SessionSlot, tab_id: Option<String>) -> Result<String, VcuError> {
    if let Some(t) = tab_id {
        return Ok(t);
    }
    if let Some(t) = slot.session.active_tab_id.clone() {
        return Ok(t);
    }
    let tabs = slot.backend.list_tabs().await?;
    tabs.into_iter()
        .find(|t| t.agent_owned)
        .map(|t| t.tab_id)
        .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, "no active tab"))
}

async fn ensure_tab_writable(slot: &mut SessionSlot, tab_id: &str) -> Result<(), VcuError> {
    let tabs = slot.backend.list_tabs().await?;
    let tab = tabs
        .iter()
        .find(|t| t.tab_id == tab_id)
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

    let obs = Observation {
        observation_id: new_id(),
        session_id: id.clone(),
        kind: "browser.snapshot".into(),
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
    };
    slot.blackboard.observation_id = Some(obs.observation_id.clone());
    slot.blackboard.dom_summary = snap.a11y_summary.clone();
    slot.blackboard.vision_summary = vision.summary.clone();
    slot.blackboard.candidates = snap.dom_refs;
    slot.session.revision += 1;
    slot.blackboard.revision = slot.session.revision;
    Json(Envelope::ok_rev(obs, slot.session.revision)).into_response()
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
            Json(Envelope::ok(json!({
                "path": path.display().to_string(),
                "sha256": digest,
                "bytes": shot.png.len()
            })))
            .into_response()
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
    let tab = match resolve_tab(slot, None).await {
        Ok(t) => t,
        Err(e) => return err_response(e),
    };
    if let Err(e) = ensure_tab_writable(slot, &tab).await {
        return err_response(e);
    }
    match slot.backend.act(&tab, &req).await {
        Ok(detail) => {
            slot.session.revision += 1;
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
}
fn default_wait() -> u64 { 5000 }

async fn extension_hello(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    state.extension_bridge.mark_hello().await;
    Json(Envelope::ok(json!({"connected": true}))).into_response()
}

async fn extension_poll(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(q): Query<ExtPollQuery>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    match state.extension_bridge.poll(q.wait_ms).await {
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

#[derive(Deserialize)]
struct AppSnapshotReq {
    id: String,
    #[serde(default = "default_app_budget")]
    budget: u64,
}
fn default_app_budget() -> u64 { 4000 }

async fn app_snapshot(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(req): Json<AppSnapshotReq>,
) -> impl IntoResponse {
    if let Err(e) = require_auth(&headers, &state).await {
        return err_response(e);
    }
    let backend = state.app_backend.read().await;
    match backend.snapshot(&req.id, req.budget).await {
        Ok(v) => Json(Envelope::ok(v)).into_response(),
        Err(e) => err_response(e),
    }
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
