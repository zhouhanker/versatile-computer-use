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

#[derive(Clone, Default)]
pub struct ExtensionBridge {
    inner: Arc<Mutex<BridgeState>>,
}

#[derive(Default)]
struct BridgeState {
    connected: bool,
    last_seen_ms: u64,
    pending: Vec<PendingCmd>,
    waiters: HashMap<String, oneshot::Sender<serde_json::Value>>,
}

struct PendingCmd {
    id: String,
    method: String,
    params: serde_json::Value,
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

    pub async fn mark_hello(&self) {
        let mut g = self.inner.lock().await;
        g.connected = true;
        g.last_seen_ms = now_ms();
    }

    pub async fn is_connected(&self) -> bool {
        let g = self.inner.lock().await;
        g.connected && now_ms().saturating_sub(g.last_seen_ms) < 30_000
    }

    pub async fn poll(&self, wait_ms: u64) -> Option<ExtensionCommand> {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(wait_ms.max(1));
        loop {
            {
                let mut g = self.inner.lock().await;
                g.last_seen_ms = now_ms();
                if let Some(cmd) = g.pending.pop() {
                    return Some(ExtensionCommand {
                        id: cmd.id,
                        method: cmd.method,
                        params: cmd.params,
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
        let mut g = self.inner.lock().await;
        if let Some(tx) = g.waiters.remove(id) {
            let _ = tx.send(result);
        }
    }

    pub async fn call(&self, method: &str, params: serde_json::Value) -> VcuResult<serde_json::Value> {
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
            });
        }
        match tokio::time::timeout(Duration::from_secs(15), rx).await {
            Ok(Ok(v)) => {
                if v.get("ok").and_then(|x| x.as_bool()) == Some(false) {
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
            other => Err(VcuError::coded(
                ErrorCode::NotImplemented,
                format!("extension action not implemented: {other}"),
            )),
        }
    }
}
