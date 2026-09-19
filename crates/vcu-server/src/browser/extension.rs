//! Extension-connected backend with HTTP command bridge.
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::{oneshot, Mutex};
use vcu_core::{
    new_id, ActionRequest, DomRef, ErrorCode, ExtractResult, SnapshotMode, TabInfo, VcuError,
    VcuResult,
};

use super::{ActionResultDetail, BrowserBackend, ScreenshotData, SnapshotData};

#[derive(Clone)]
pub struct ExtensionBridge {
    inner: Arc<Mutex<BridgeState>>,
    lease_ms: u64,
}

impl Default for ExtensionBridge {
    fn default() -> Self {
        Self {
            inner: Arc::new(Mutex::new(BridgeState::default())),
            lease_ms: 1_500,
        }
    }
}

#[derive(Default)]
struct BridgeState {
    connected: bool,
    last_seen_ms: u64,
    last_poll_ms: u64,
    pending: Vec<PendingCmd>,
    waiters: HashMap<String, oneshot::Sender<serde_json::Value>>,
    likely_user_profile: bool,
}

struct PendingCmd {
    id: String,
    method: String,
    params: serde_json::Value,
    leased_until: Option<u64>,
    retries: u8,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtensionCommand {
    pub id: String,
    pub method: String,
    pub params: serde_json::Value,
}

impl ExtensionBridge {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_lease_ms(lease_ms: u64) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BridgeState::default())),
            lease_ms: lease_ms.max(1),
        }
    }

    pub async fn mark_hello(&self, likely_user_profile: bool) {
        let mut g = self.inner.lock().await;
        g.connected = true;
        g.last_seen_ms = now_ms();
        if likely_user_profile {
            g.likely_user_profile = true;
        }
    }

    pub async fn likely_user_profile(&self) -> bool {
        self.inner.lock().await.likely_user_profile
    }

    pub async fn is_connected(&self) -> bool {
        let g = self.inner.lock().await;
        g.connected && now_ms().saturating_sub(g.last_seen_ms) < 120_000
    }

    pub async fn is_polling(&self) -> bool {
        let g = self.inner.lock().await;
        g.connected && now_ms().saturating_sub(g.last_poll_ms) < 15_000
    }

    pub async fn pending_len(&self) -> usize {
        self.inner.lock().await.pending.len()
    }

    pub async fn waiter_len(&self) -> usize {
        self.inner.lock().await.waiters.len()
    }

    pub async fn last_poll_age_ms(&self) -> Option<u64> {
        let g = self.inner.lock().await;
        if g.last_poll_ms == 0 {
            None
        } else {
            Some(now_ms().saturating_sub(g.last_poll_ms))
        }
    }

    pub async fn poll(&self, wait_ms: u64) -> Option<ExtensionCommand> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(wait_ms.max(1));
        loop {
            {
                let mut g = self.inner.lock().await;
                g.last_seen_ms = now_ms();
                g.last_poll_ms = now_ms();
                let now = now_ms();
                let lease = self.lease_ms;
                if let Some(cmd) = g.pending.iter_mut().find(|c| {
                    // A lost reply does not mean a mutation failed. Replaying a click,
                    // type or open can produce a second user-visible side effect.
                    c.leased_until.map(|t| t <= now && matches!(c.method.as_str(),
                        "ping" | "list_tabs" | "extract" | "snapshot" | "screenshot"
                    )).unwrap_or(true)
                }) {
                    cmd.leased_until = Some(now.saturating_add(lease));
                    return Some(ExtensionCommand {
                        id: cmd.id.clone(),
                        method: cmd.method.clone(),
                        params: cmd.params.clone(),
                    });
                }
            }
            if tokio::time::Instant::now() >= deadline {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn submit_result(&self, id: &str, result: serde_json::Value) {
        let retryable = result.get("retryable").and_then(|x| x.as_bool()) == Some(true);
        let mut g = self.inner.lock().await;
        if retryable {
            if let Some(cmd) = g.pending.iter_mut().find(|c| c.id == id) {
                if cmd.retries < 8 {
                    cmd.retries = cmd.retries.saturating_add(1);
                    cmd.leased_until = None;
                    return;
                }
            }
        }
        g.pending.retain(|c| c.id != id);
        if let Some(tx) = g.waiters.remove(id) {
            let _ = tx.send(result);
        }
    }

    pub async fn call(&self, method: &str, params: serde_json::Value) -> VcuResult<serde_json::Value> {
        self.call_timeout(method, params, 30).await
    }

    pub async fn call_timeout(&self, method: &str, params: serde_json::Value, timeout_secs: u64) -> VcuResult<serde_json::Value> {
        if !self.is_connected().await {
            return Err(VcuError::coded(
                ErrorCode::ExtensionDisconnected,
                "no browser extension is paired with this daemon",
            ));
        }
        let id = new_id();
        let (tx, rx) = oneshot::channel();
        {
            let mut g = self.inner.lock().await;
            g.waiters.insert(id.clone(), tx);
            g.pending.push(PendingCmd {
                id: id.clone(),
                method: method.to_string(),
                params,
                leased_until: None,
                retries: 0,
            });
        }
        match tokio::time::timeout(Duration::from_secs(timeout_secs.max(1)), rx).await {
            Ok(Ok(v)) => {
                if v.get("ok").and_then(|x| x.as_bool()) != Some(true) {
                    return Err(VcuError::with_detail(
                        ErrorCode::ActionFailed,
                        method,
                        v.to_string(),
                    ));
                }
                Ok(v)
            }
            Ok(Err(_)) => Err(VcuError::coded(
                ErrorCode::ExtensionDisconnected,
                "extension dropped command",
            )),
            Err(_) => {
                let mut g = self.inner.lock().await;
                g.waiters.remove(&id);
                g.pending.retain(|c| c.id != id);
                Err(VcuError::coded(
                    ErrorCode::ActionFailed,
                    "extension command timeout",
                ))
            }
        }
    }
}

fn now_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub struct ExtensionBackend {
    bridge: ExtensionBridge,
}

impl ExtensionBackend {
    pub fn new() -> Self {
        Self {
            bridge: ExtensionBridge::new(),
        }
    }

    pub fn with_bridge(bridge: ExtensionBridge) -> Self {
        Self { bridge }
    }
}

impl Default for ExtensionBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserBackend for ExtensionBackend {
    fn name(&self) -> &str {
        "extension"
    }

    async fn ensure_agent_window(&mut self) -> VcuResult<String> {
        let v = self
            .bridge
            .call("ensure_agent_window", serde_json::json!({}))
            .await?;
        Ok(v.get("window_id")
            .and_then(|x| x.as_str())
            .unwrap_or("ext")
            .to_string())
    }

    async fn list_tabs(&self) -> VcuResult<Vec<TabInfo>> {
        let v = self.bridge.call("list_tabs", serde_json::json!({})).await?;
        let tabs = v.get("tabs").cloned().unwrap_or(serde_json::json!([]));
        serde_json::from_value(tabs).map_err(|e| {
            VcuError::with_detail(ErrorCode::ActionFailed, "tabs decode", e.to_string())
        })
    }

    async fn navigate(&mut self, tab_id: &str, url: &str) -> VcuResult<()> {
        self.bridge
            .call(
                "navigate",
                serde_json::json!({"tab_id": tab_id, "url": url}),
            )
            .await?;
        Ok(())
    }

    async fn snapshot(
        &self,
        tab_id: &str,
        mode: SnapshotMode,
        budget_tokens: u64,
    ) -> VcuResult<SnapshotData> {
        let mode_s = match mode {
            SnapshotMode::A11y => "a11y",
            SnapshotMode::Dom => "dom",
            SnapshotMode::Text => "text",
            SnapshotMode::Full => "full",
        };
        let v = self
            .bridge
            .call(
                "snapshot",
                serde_json::json!({
                    "tab_id": tab_id,
                    "mode": mode_s,
                    "budget": budget_tokens
                }),
            )
            .await?;
        let refs = v
            .get("dom_refs")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                Some(DomRef {
                    r#ref: r.get("ref")?.as_str()?.to_string(),
                    role: r.get("role").and_then(|x| x.as_str()).unwrap_or("").into(),
                    name: r.get("name").and_then(|x| x.as_str()).unwrap_or("").into(),
                    value: None,
                    selector: r
                        .get("selector")
                        .and_then(|x| x.as_str())
                        .map(|s| s.to_string()),
                    frame: None,
                })
            })
            .collect();
        Ok(SnapshotData {
            a11y_summary: v
                .get("a11y_summary")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            dom_refs: refs,
            text_excerpt: v
                .get("text_excerpt")
                .and_then(|x| x.as_str())
                .map(|s| s.to_string()),
            screenshot_png: None,
            truncated: v.get("truncated").and_then(|x| x.as_bool()).unwrap_or(false),
            webview: false,
            webview_ref: None,
            webview_png: None,
            screenshot_scale: None,
            webview_screenshot_scale: None,
            budget_tokens_est: v
                .get("budget_tokens_est")
                .and_then(|x| x.as_u64())
                .unwrap_or(0),
        })
    }

    async fn click(&mut self, tab_id: &str, target_ref: &str) -> VcuResult<ActionResultDetail> {
        let v = self
            .bridge
            .call(
                "click",
                serde_json::json!({"tab_id": tab_id, "ref": target_ref}),
            )
            .await?;
        Ok(ActionResultDetail {
            ok: true,
            detail: v,
        })
    }

    async fn type_text(
        &mut self,
        tab_id: &str,
        text: &str,
        target_ref: Option<&str>,
    ) -> VcuResult<ActionResultDetail> {
        let v = self
            .bridge
            .call(
                "type",
                serde_json::json!({"tab_id": tab_id, "text": text, "ref": target_ref}),
            )
            .await?;
        Ok(ActionResultDetail {
            ok: true,
            detail: v,
        })
    }

    async fn extract(&self, tab_id: &str, selector: &str) -> VcuResult<ExtractResult> {
        let v = self
            .bridge
            .call(
                "extract",
                serde_json::json!({"tab_id": tab_id, "selector": selector}),
            )
            .await?;
        let matches = v
            .get("matches")
            .and_then(|x| x.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(ExtractResult {
            session_id: String::new(),
            selector: Some(selector.into()),
            count: matches.len(),
            matches,
        })
    }

    async fn screenshot(&self, tab_id: &str, full_page: bool) -> VcuResult<ScreenshotData> {
        let v = self
            .bridge
            .call(
                "screenshot",
                serde_json::json!({"tab_id": tab_id, "full_page": full_page}),
            )
            .await?;
        let b64 = v.get("png_base64").and_then(|x| x.as_str()).unwrap_or("");
        let png = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .unwrap_or_default();
        Ok(ScreenshotData {
            png,
            width: 0,
            height: 0,
            frame: None,
            scale: None,
            webview_png: None,
            webview_ref: None,
            webview_frame: None,
            webview_scale: None,
        })
    }

    async fn act(&mut self, tab_id: &str, action: &ActionRequest) -> VcuResult<ActionResultDetail> {
        match action.r#type.as_str() {
            "os_cursor_move" | "os_click" => Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "OS cursor actions are denied by browser session policy",
            )),
            "click" => {
                let r = action
                    .target
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "click requires target.ref")
                    })?;
                self.click(tab_id, r).await
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
            "navigate" => {
                let url = action
                    .args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "navigate requires args.url")
                    })?;
                self.navigate(tab_id, url).await?;
                Ok(ActionResultDetail {
                    ok: true,
                    detail: serde_json::json!({"url": url}),
                })
            }
            "scroll" => {
                let dy = action.args.get("dy").and_then(|v| v.as_i64()).unwrap_or(400);
                let v = self
                    .bridge
                    .call("scroll", serde_json::json!({"tab_id": tab_id, "dy": dy}))
                    .await?;
                Ok(ActionResultDetail { ok: true, detail: v })
            }
            "wait" => {
                let ms = action.args.get("ms").and_then(|v| v.as_u64()).unwrap_or(100);
                let v = self
                    .bridge
                    .call("wait", serde_json::json!({"tab_id": tab_id, "ms": ms}))
                    .await?;
                Ok(ActionResultDetail { ok: true, detail: v })
            }
            "hover" => {
                let r = action
                    .target
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "hover requires target.ref")
                    })?;
                let v = self
                    .bridge
                    .call("hover", serde_json::json!({"tab_id": tab_id, "ref": r}))
                    .await?;
                Ok(ActionResultDetail { ok: true, detail: v })
            }
            "keypress" => {
                let key = action
                    .args
                    .get("key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        VcuError::coded(ErrorCode::InvalidInput, "keypress requires args.key")
                    })?;
                let v = self
                    .bridge
                    .call(
                        "keypress",
                        serde_json::json!({"tab_id": tab_id, "key": key}),
                    )
                    .await?;
                Ok(ActionResultDetail { ok: true, detail: v })
            }
            other => Err(VcuError::coded(
                ErrorCode::NotImplemented,
                format!("extension action not implemented: {other}"),
            )),
        }
    }
}
