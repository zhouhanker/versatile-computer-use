//! CDP backend with dual discovery + persistent browser WebSocket.
//!
//! ## Why persistent WS?
//! Edge/Chrome UI remote-debugging (`edge://inspect/#remote-debugging`) may prompt
//! "Allow debugging" on **each new client connection**. Opening a WS per CDP call
//! forces the user to click repeatedly. Codex Computer Use avoids this by using a
//! long-lived helper + OS Accessibility (one grant). VCU mirrors that UX for CDP by
//! keeping **one browser WS for the whole backend lifetime** after a single Allow.
//!
//! ## Discovery modes
//! 1) Classic HTTP `/json/version` + `/json/list` (flag-launched browser)
//! 2) Browser WS `ws://host:port/devtools/browser` (UI remote-debugging; HTTP often 404,
//!    page WS often 403 — must use Target.attachToTarget flatten sessions)
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vcu_core::{
    ActionRequest, DomRef, ErrorCode, ExtractResult, SnapshotMode, TabInfo, VcuError, VcuResult,
};

use super::{ActionResultDetail, BrowserBackend, ScreenshotData, SnapshotData};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CdpDiscovery {
    HttpJson,
    BrowserWs,
}

type WsStream = tokio_tungstenite::WebSocketStream<
    tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
>;

struct Pending {
    tx: oneshot::Sender<Result<Value, String>>,
}

/// Long-lived browser-level CDP connection (single Allow dialog).
struct BrowserConn {
    tx: mpsc::UnboundedSender<ConnReq>,
    next_id: AtomicU64,
    /// targetId -> sessionId (flat attach cache)
    sessions: Mutex<HashMap<String, String>>,
}

enum ConnReq {
    Call {
        id: u64,
        method: String,
        params: Value,
        session_id: Option<String>,
        resp: oneshot::Sender<Result<Value, String>>,
    },
}

impl BrowserConn {
    async fn connect(ws_url: &str) -> VcuResult<Arc<Self>> {
        let (ws, _) = connect_async(ws_url).await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "ws connect", e.to_string())
        })?;
        let (tx, rx) = mpsc::unbounded_channel::<ConnReq>();
        let conn = Arc::new(Self {
            tx,
            next_id: AtomicU64::new(1),
            sessions: Mutex::new(HashMap::new()),
        });
        tokio::spawn(browser_conn_loop(ws, rx));
        // Warm-up / validate
        conn.call("Browser.getVersion", serde_json::json!({}), None)
            .await
            .map_err(|e| {
                VcuError::with_detail(
                    ErrorCode::CdpConnectFailed,
                    "Browser.getVersion after connect",
                    e.to_string(),
                )
            })?;
        Ok(conn)
    }

    async fn call(
        &self,
        method: &str,
        params: Value,
        session_id: Option<String>,
    ) -> VcuResult<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(ConnReq::Call {
                id,
                method: method.to_string(),
                params,
                session_id,
                resp: resp_tx,
            })
            .map_err(|_| {
                VcuError::coded(ErrorCode::CdpConnectFailed, "browser CDP connection closed")
            })?;
        match tokio::time::timeout(std::time::Duration::from_secs(20), resp_rx).await {
            Ok(Ok(Ok(v))) => Ok(v),
            Ok(Ok(Err(e))) => Err(VcuError::with_detail(ErrorCode::ActionFailed, method, e)),
            Ok(Err(_)) => Err(VcuError::coded(
                ErrorCode::CdpConnectFailed,
                "browser CDP connection dropped",
            )),
            Err(_) => Err(VcuError::coded(ErrorCode::ActionFailed, "cdp timeout")),
        }
    }

    async fn attach(&self, target_id: &str) -> VcuResult<String> {
        {
            let g = self.sessions.lock().await;
            if let Some(sid) = g.get(target_id) {
                return Ok(sid.clone());
            }
        }
        let result = self
            .call(
                "Target.attachToTarget",
                serde_json::json!({"targetId": target_id, "flatten": true}),
                None,
            )
            .await?;
        let sid = result
            .get("sessionId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                VcuError::coded(ErrorCode::ActionFailed, "attachToTarget missing sessionId")
            })?
            .to_string();
        self.sessions
            .lock()
            .await
            .insert(target_id.to_string(), sid.clone());
        Ok(sid)
    }

    async fn page_call(&self, target_id: &str, method: &str, params: Value) -> VcuResult<Value> {
        let sid = self.attach(target_id).await?;
        match self.call(method, params.clone(), Some(sid.clone())).await {
            Ok(v) => Ok(v),
            Err(e) => {
                // Session may have died; drop cache and retry once.
                self.sessions.lock().await.remove(target_id);
                let sid = self.attach(target_id).await?;
                self.call(method, params, Some(sid)).await.map_err(|_| e)
            }
        }
    }
}

async fn browser_conn_loop(mut ws: WsStream, mut rx: mpsc::UnboundedReceiver<ConnReq>) {
    let mut pending: HashMap<u64, Pending> = HashMap::new();
    loop {
        tokio::select! {
            req = rx.recv() => {
                let Some(req) = req else { break; };
                match req {
                    ConnReq::Call { id, method, params, session_id, resp } => {
                        let mut msg = serde_json::json!({
                            "id": id,
                            "method": method,
                            "params": params
                        });
                        if let Some(sid) = session_id {
                            msg.as_object_mut().unwrap().insert(
                                "sessionId".into(),
                                Value::String(sid),
                            );
                        }
                        pending.insert(id, Pending { tx: resp });
                        if ws.send(Message::Text(msg.to_string().into())).await.is_err() {
                            if let Some(p) = pending.remove(&id) {
                                let _ = p.tx.send(Err("ws send failed".into()));
                            }
                            break;
                        }
                    }
                }
            }
            got = ws.next() => {
                match got {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(v) = serde_json::from_str::<Value>(&text) else { continue; };
                        if let Some(id) = v.get("id").and_then(|x| x.as_u64()) {
                            if let Some(p) = pending.remove(&id) {
                                if let Some(err) = v.get("error") {
                                    let _ = p.tx.send(Err(err.to_string()));
                                } else {
                                    let _ = p.tx.send(Ok(
                                        v.get("result").cloned().unwrap_or(Value::Null)
                                    ));
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws.send(Message::Pong(data)).await;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) | None => break,
                }
            }
        }
    }
    for (_, p) in pending.drain() {
        let _ = p.tx.send(Err("browser CDP connection closed".into()));
    }
}

pub struct CdpBackend {
    http_base: String,
    #[allow(dead_code)]
    browser_ws_url: String,
    discovery: CdpDiscovery,
    client: reqwest::Client,
    conn: Arc<BrowserConn>,
    agent_tab_id: String,
    pages: Vec<CdpPage>,
}

#[derive(Clone)]
struct CdpPage {
    id: String,
    title: String,
    url: String,
}

impl CdpBackend {
    pub async fn connect(http_base: &str) -> VcuResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::CdpConnectFailed, "http client", e.to_string())
            })?;
        let base = http_base.trim_end_matches('/').to_string();
        let (browser_ws_url, discovery) = resolve_browser_ws(&client, &base).await?;
        // Single connection — user should Allow at most once for this session.
        let conn = BrowserConn::connect(&browser_ws_url).await?;

        let mut backend = Self {
            http_base: base,
            browser_ws_url,
            discovery,
            client,
            conn,
            agent_tab_id: String::new(),
            pages: vec![],
        };
        backend.refresh_pages().await?;

        // Dedicated agent tab (Codex-like): do not thrash user tabs.
        if let Ok(id) = backend.ensure_dedicated_agent_page("about:blank").await {
            backend.agent_tab_id = id;
        } else if let Some(p) = backend.pages.first() {
            backend.agent_tab_id = p.id.clone();
        } else {
            return Err(VcuError::coded(
                ErrorCode::BrowserUnavailable,
                "CDP connected but no pages available",
            ));
        }
        backend.refresh_pages().await.ok();
        Ok(backend)
    }

    async fn ensure_dedicated_agent_page(&mut self, url: &str) -> VcuResult<String> {
        for p in &self.pages {
            if p.url == "about:blank" || p.url.starts_with("about:blank") {
                return Ok(p.id.clone());
            }
        }
        let result = self
            .conn
            .call(
                "Target.createTarget",
                serde_json::json!({ "url": url }),
                None,
            )
            .await?;
        result
            .get("targetId")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                VcuError::coded(ErrorCode::BrowserUnavailable, "Target.createTarget missing id")
            })
    }

    async fn refresh_pages(&mut self) -> VcuResult<()> {
        if self.discovery == CdpDiscovery::HttpJson {
            if let Ok(pages) = self.refresh_pages_http().await {
                if !pages.is_empty() {
                    self.pages = pages;
                    return Ok(());
                }
            }
        }
        self.pages = self.refresh_pages_ws().await?;
        Ok(())
    }

    async fn refresh_pages_http(&self) -> VcuResult<Vec<CdpPage>> {
        let list_url = format!("{}/json/list", self.http_base);
        let resp = self.client.get(&list_url).send().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "json/list", e.to_string())
        })?;
        if !resp.status().is_success() {
            return Err(VcuError::coded(
                ErrorCode::CdpConnectFailed,
                format!("json/list status {}", resp.status()),
            ));
        }
        let pages: Value = resp.json().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "json/list body", e.to_string())
        })?;
        let arr = pages.as_array().cloned().unwrap_or_default();
        Ok(arr
            .into_iter()
            .filter_map(|p| {
                let ty = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if ty != "page" {
                    return None;
                }
                let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.starts_with("devtools://") || url.starts_with("chrome://omnibox") {
                    return None;
                }
                Some(CdpPage {
                    id: p.get("id")?.as_str()?.to_string(),
                    title: p
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    url: url.to_string(),
                })
            })
            .collect())
    }

    async fn refresh_pages_ws(&self) -> VcuResult<Vec<CdpPage>> {
        let result = self
            .conn
            .call("Target.getTargets", serde_json::json!({}), None)
            .await?;
        let infos = result
            .get("targetInfos")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(infos
            .into_iter()
            .filter_map(|p| {
                let ty = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                if ty != "page" {
                    return None;
                }
                let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.starts_with("devtools://") {
                    return None;
                }
                Some(CdpPage {
                    id: p.get("targetId")?.as_str()?.to_string(),
                    title: p
                        .get("title")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    url: url.to_string(),
                })
            })
            .collect())
    }

    fn page(&self, tab_id: &str) -> VcuResult<&CdpPage> {
        self.pages
            .iter()
            .find(|p| p.id == tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))
    }

    async fn runtime_eval(&self, tab_id: &str, expression: &str) -> VcuResult<Value> {
        self.conn
            .page_call(
                tab_id,
                "Runtime.evaluate",
                serde_json::json!({
                    "expression": expression,
                    "returnByValue": true,
                    "awaitPromise": true
                }),
            )
            .await
    }
}

async fn resolve_browser_ws(
    client: &reqwest::Client,
    base: &str,
) -> VcuResult<(String, CdpDiscovery)> {
    let version_url = format!("{base}/json/version");
    if let Ok(resp) = client.get(&version_url).send().await {
        if resp.status().is_success() {
            if let Ok(ver_json) = resp.json::<Value>().await {
                if let Some(ws) = ver_json
                    .get("webSocketDebuggerUrl")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                {
                    return Ok((ws.to_string(), CdpDiscovery::HttpJson));
                }
            }
        }
    }

    // Do NOT open a temporary WS here — each new client connection can trigger
    // Edge/Chrome "Allow debugging" UI. connect() opens the only persistent WS.
    let ws = http_base_to_browser_ws(base);
    Ok((ws, CdpDiscovery::BrowserWs))
}

fn http_base_to_browser_ws(base: &str) -> String {
    let base = base.trim_end_matches('/');
    if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}/devtools/browser")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}/devtools/browser")
    } else if base.starts_with("ws://") || base.starts_with("wss://") {
        if base.contains("/devtools/") {
            base.to_string()
        } else {
            format!("{base}/devtools/browser")
        }
    } else {
        format!("ws://{base}/devtools/browser")
    }
}


#[async_trait]
impl BrowserBackend for CdpBackend {
    fn name(&self) -> &str {
        "cdp"
    }

    async fn ensure_agent_window(&mut self) -> VcuResult<String> {
        self.refresh_pages().await?;
        if self.agent_tab_id.is_empty() || self.page(&self.agent_tab_id).is_err() {
            if let Ok(id) = self.ensure_dedicated_agent_page("about:blank").await {
                self.agent_tab_id = id;
                self.refresh_pages().await.ok();
            }
        }
        Ok(self.agent_tab_id.clone())
    }

    async fn list_tabs(&self) -> VcuResult<Vec<TabInfo>> {
        Ok(self
            .pages
            .iter()
            .map(|p| TabInfo {
                tab_id: p.id.clone(),
                window_id: match self.discovery {
                    CdpDiscovery::HttpJson => "cdp:http_json".into(),
                    CdpDiscovery::BrowserWs => "cdp:browser_ws".into(),
                },
                title: p.title.clone(),
                url: p.url.clone(),
                agent_owned: p.id == self.agent_tab_id,
                borrowed_by: None,
                login_state: Some(false),
                browser_profile: Some("agent".into()),
            })
            .collect())
    }

    async fn navigate(&mut self, tab_id: &str, url: &str) -> VcuResult<()> {
        let _ = self
            .conn
            .page_call(tab_id, "Page.enable", serde_json::json!({}))
            .await;
        self.conn
            .page_call(tab_id, "Page.navigate", serde_json::json!({ "url": url }))
            .await?;
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Ok(result) = self.runtime_eval(tab_id, "document.readyState").await {
                let state = result
                    .pointer("/result/value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if state == "complete" || state == "interactive" {
                    break;
                }
            }
        }
        self.refresh_pages().await.ok();
        Ok(())
    }

    async fn snapshot(
        &self,
        tab_id: &str,
        mode: SnapshotMode,
        budget_tokens: u64,
    ) -> VcuResult<SnapshotData> {
        let _ = self.page(tab_id)?;
        let expr = r#"(function(){
          const refs=[];
          const pick=el=>{
            if(!el||refs.length>=80) return;
            const tag=(el.tagName||'').toLowerCase();
            if(!tag||tag==='script'||tag==='style'||tag==='meta'||tag==='link') return;
            const role=el.getAttribute('role')||'';
            const name=(el.getAttribute('aria-label')||el.innerText||el.value||'').trim().slice(0,80);
            const interesting=['a','button','input','textarea','select','h1','h2','h3','h4','li','summary','label'];
            if(interesting.includes(tag) || role || (name && name.length>1 && name.length<60)){
              const id='e'+(refs.length+1);
              try{ el.setAttribute('data-vcu-ref', id); }catch(e){}
              refs.push({ref:id, role: role||tag, name, tag, href: el.href||null});
            }
          };
          const all=[...document.querySelectorAll('a,button,input,textarea,select,h1,h2,h3,h4,li,[role],label,summary')];
          all.slice(0,200).forEach(pick);
          const title=document.title||'';
          const text=(document.body && document.body.innerText || '').slice(0,8000);
          const a11y = title + '\n' + refs.map(r=>`- ${r.role}: ${r.name}`).join('\n');
          return {a11y, text, refs, url: location.href, title};
        })()"#;
        let result = self.runtime_eval(tab_id, expr).await?;
        let value = result.pointer("/result/value").cloned().unwrap_or(Value::Null);
        let a11y = value
            .get("a11y")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut text = value
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut refs: Vec<DomRef> = value
            .get("refs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|r| {
                Some(DomRef {
                    r#ref: r.get("ref")?.as_str()?.to_string(),
                    role: r
                        .get("role")
                        .and_then(|v| v.as_str())
                        .unwrap_or("generic")
                        .to_string(),
                    name: r
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    value: None,
                    selector: r
                        .get("tag")
                        .and_then(|v| v.as_str())
                        .map(|t| format!("{t}[data-vcu-ref]")),
                    frame: None,
                })
            })
            .collect();
        let mut truncated = false;
        let est = (a11y.len() + text.len()) as u64 / 4 + refs.len() as u64 * 8;
        let budget = if budget_tokens == 0 {
            u64::MAX
        } else {
            budget_tokens
        };
        if est > budget {
            truncated = true;
            text.truncate((budget as usize).saturating_mul(4).min(text.len()));
            let max_refs = (budget as usize / 10).max(1);
            if refs.len() > max_refs {
                refs.truncate(max_refs);
            }
        }
        let (a11y_summary, text_excerpt, dom_refs) = match mode {
            SnapshotMode::A11y => (Some(a11y), None, refs),
            SnapshotMode::Text => (None, Some(text), vec![]),
            SnapshotMode::Dom => (None, None, refs),
            SnapshotMode::Full => (Some(a11y), Some(text), refs),
        };
        Ok(SnapshotData {
            a11y_summary,
            dom_refs,
            text_excerpt,
            screenshot_png: None,
            truncated,
            budget_tokens_est: est.min(budget),
            webview: false,
            webview_ref: None,
            webview_png: None,
            screenshot_scale: None,
            webview_screenshot_scale: None,
        })
    }

    async fn click(&mut self, tab_id: &str, target_ref: &str) -> VcuResult<ActionResultDetail> {
        let expr = format!(
            r#"(function(){{ const el=document.querySelector('[data-vcu-ref="{r}"]');
            if(!el) return {{ok:false, error:'not found'}};
            el.click(); return {{ok:true, ref:'{r}', input_path:'page_dom', os_cursor_used:false}}; }})()"#,
            r = target_ref
        );
        let result = self.runtime_eval(tab_id, &expr).await?;
        let value = result.pointer("/result/value").cloned().unwrap_or(Value::Null);
        let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            return Err(VcuError::with_detail(
                ErrorCode::ActionFailed,
                "click failed",
                value.to_string(),
            ));
        }
        for _ in 0..20 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Ok(result) = self.runtime_eval(tab_id, "document.readyState").await {
                let state = result
                    .pointer("/result/value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if state == "complete" {
                    break;
                }
            }
        }
        self.refresh_pages().await.ok();
        Ok(ActionResultDetail {
            ok: true,
            detail: value,
        })
    }

    async fn type_text(
        &mut self,
        tab_id: &str,
        text: &str,
        target_ref: Option<&str>,
    ) -> VcuResult<ActionResultDetail> {
        let r = target_ref.unwrap_or("");
        let escaped = text.replace('\\', "\\\\").replace('\'', "\\'");
        let expr = if r.is_empty() {
            format!(
                r#"(function(){{ document.execCommand('insertText', false, '{t}');
                return {{ok:true, typed:'{t}', input_path:'page_dom', os_cursor_used:false}}; }})()"#,
                t = escaped
            )
        } else {
            format!(
                r#"(function(){{ const el=document.querySelector('[data-vcu-ref="{r}"]');
                if(!el) return {{ok:false}}; el.focus(); el.value='{t}';
                el.dispatchEvent(new Event('input', {{bubbles:true}}));
                return {{ok:true, typed:'{t}', ref:'{r}', input_path:'page_dom', os_cursor_used:false}}; }})()"#,
                r = r,
                t = escaped
            )
        };
        let result = self.runtime_eval(tab_id, &expr).await?;
        let value = result.pointer("/result/value").cloned().unwrap_or(Value::Null);
        Ok(ActionResultDetail {
            ok: value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false),
            detail: value,
        })
    }

    async fn extract(&self, tab_id: &str, selector: &str) -> VcuResult<ExtractResult> {
        let sel = selector.replace('\\', "\\\\").replace('\'', "\\'");
        let expr = format!(
            r#"(function(){{ const nodes=[...document.querySelectorAll('{s}')].slice(0,50);
            return nodes.map(el=>({{tag:el.tagName, text:(el.innerText||'').slice(0,200),
              href:el.href||null, value:el.value||null}})); }})()"#,
            s = sel
        );
        let result = self.runtime_eval(tab_id, &expr).await?;
        let value = result.pointer("/result/value").cloned().unwrap_or(Value::Null);
        let matches = value.as_array().cloned().unwrap_or_default();
        Ok(ExtractResult {
            session_id: String::new(),
            selector: Some(selector.to_string()),
            count: matches.len(),
            matches,
        })
    }

    async fn screenshot(&self, tab_id: &str, _full_page: bool) -> VcuResult<ScreenshotData> {
        let _ = self.page(tab_id)?;
        let result = self
            .conn
            .page_call(
                tab_id,
                "Page.captureScreenshot",
                serde_json::json!({"format": "png"}),
            )
            .await?;
        let b64 = result
            .get("data")
            .and_then(|v| v.as_str())
            .ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "no screenshot data"))?;
        let png = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64)
            .map_err(|e| VcuError::with_detail(ErrorCode::ActionFailed, "b64", e.to_string()))?;
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
                let expr = format!("window.scrollBy(0, {dy}); true");
                let _ = self.runtime_eval(tab_id, &expr).await?;
                Ok(ActionResultDetail {
                    ok: true,
                    detail: serde_json::json!({
                        "scrolled": dy,
                        "input_path": "page_dom",
                        "os_cursor_used": false
                    }),
                })
            }
            "wait" => {
                let ms = action.args.get("ms").and_then(|v| v.as_u64()).unwrap_or(100);
                tokio::time::sleep(std::time::Duration::from_millis(ms.min(5000))).await;
                Ok(ActionResultDetail {
                    ok: true,
                    detail: serde_json::json!({"waited_ms": ms, "os_cursor_used": false}),
                })
            }
            "os_cursor_move" | "os_click" => Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "OS cursor actions are denied by browser session policy",
            )),
            other => Err(VcuError::coded(
                ErrorCode::NotImplemented,
                format!("cdp action not implemented: {other}"),
            )),
        }
    }
}
