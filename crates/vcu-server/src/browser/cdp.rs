//! Minimal CDP backend: connect to Chrome/Edge remote debugging HTTP endpoint.
use async_trait::async_trait;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use vcu_core::{
    ActionRequest, DomRef, ErrorCode, ExtractResult, SnapshotMode, TabInfo, VcuError,
    VcuResult,
};

use super::{ActionResultDetail, BrowserBackend, ScreenshotData, SnapshotData};

pub struct CdpBackend {
    http_base: String,
    ws_url: String,
    client: reqwest::Client,
    agent_tab_id: String,
    pages: Vec<CdpPage>,
}

#[derive(Clone)]
struct CdpPage {
    id: String,
    title: String,
    url: String,
    web_socket_debugger_url: String,
}

impl CdpBackend {
    pub async fn connect(http_base: &str) -> VcuResult<Self> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .map_err(|e| VcuError::with_detail(ErrorCode::CdpConnectFailed, "http client", e.to_string()))?;
        let base = http_base.trim_end_matches('/');
        let version_url = format!("{base}/json/version");
        let ver = client
            .get(&version_url)
            .send()
            .await
            .map_err(|e| {
                VcuError::with_detail(
                    ErrorCode::CdpConnectFailed,
                    format!("cannot reach {version_url}"),
                    e.to_string(),
                )
            })?
            .error_for_status()
            .map_err(|e| VcuError::with_detail(ErrorCode::CdpConnectFailed, "cdp version", e.to_string()))?;
        let ver_json: Value = ver.json().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "cdp version json", e.to_string())
        })?;
        let ws_url = ver_json
            .get("webSocketDebuggerUrl")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut backend = Self {
            http_base: base.to_string(),
            ws_url,
            client,
            agent_tab_id: String::new(),
            pages: vec![],
        };
        backend.refresh_pages().await?;
        if backend.pages.is_empty() {
            // Prefer explicit about:blank page target.
            for path in [
                format!("{base}/json/new?about:blank"),
                format!("{base}/json/new?url=about:blank"),
            ] {
                let _ = backend.client.put(&path).send().await;
                let _ = backend.client.get(&path).send().await;
                backend.refresh_pages().await?;
                if !backend.pages.is_empty() {
                    break;
                }
            }
        }
        if backend.pages.is_empty() && !backend.ws_url.is_empty() {
            // Last resort: Target.createTarget on browser websocket.
            if let Ok(result) = Self::cdp_call(
                &backend.ws_url,
                "Target.createTarget",
                serde_json::json!({"url": "about:blank"}),
            )
            .await
            {
                let _ = result.get("targetId");
                backend.refresh_pages().await?;
            }
        }
        if let Some(p) = backend.pages.first() {
            backend.agent_tab_id = p.id.clone();
        } else {
            return Err(VcuError::coded(
                ErrorCode::BrowserUnavailable,
                "CDP connected but no pages available",
            ));
        }
        Ok(backend)
    }

    async fn refresh_pages(&mut self) -> VcuResult<()> {
        let list_url = format!("{}/json/list", self.http_base);
        let resp = self.client.get(&list_url).send().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "json/list", e.to_string())
        })?;
        let pages: Value = resp.json().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "json/list body", e.to_string())
        })?;
        let arr = pages.as_array().cloned().unwrap_or_default();
        self.pages = arr
            .into_iter()
            .filter_map(|p| {
                let ty = p.get("type").and_then(|v| v.as_str()).unwrap_or("");
                // Only real page targets — never browser_ui / background_page / etc.
                if ty != "page" {
                    return None;
                }
                let url = p.get("url").and_then(|v| v.as_str()).unwrap_or("");
                if url.starts_with("devtools://") || url.starts_with("chrome://omnibox") {
                    return None;
                }
                let ws = p.get("webSocketDebuggerUrl")?.as_str()?.to_string();
                Some(CdpPage {
                    id: p.get("id")?.as_str()?.to_string(),
                    title: p.get("title").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                    url: url.to_string(),
                    web_socket_debugger_url: ws,
                })
            })
            .collect();
        Ok(())
    }

    fn page(&self, tab_id: &str) -> VcuResult<&CdpPage> {
        self.pages
            .iter()
            .find(|p| p.id == tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("cdp tab {tab_id}")))
    }

    async fn cdp_call(ws_url: &str, method: &str, params: Value) -> VcuResult<Value> {
        let (mut ws, _) = connect_async(ws_url).await.map_err(|e| {
            VcuError::with_detail(ErrorCode::CdpConnectFailed, "ws connect", e.to_string())
        })?;
        let id = 1u64;
        let msg = serde_json::json!({"id": id, "method": method, "params": params});
        ws.send(Message::Text(msg.to_string().into()))
            .await
            .map_err(|e| VcuError::with_detail(ErrorCode::ActionFailed, "ws send", e.to_string()))?;

        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            let left = deadline.saturating_duration_since(tokio::time::Instant::now());
            if left.is_zero() {
                return Err(VcuError::coded(ErrorCode::ActionFailed, "cdp timeout"));
            }
            let got = tokio::time::timeout(left, ws.next())
                .await
                .map_err(|_| VcuError::coded(ErrorCode::ActionFailed, "cdp timeout"))?
                .ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "cdp closed"))?
                .map_err(|e| VcuError::with_detail(ErrorCode::ActionFailed, "ws read", e.to_string()))?;
            let Message::Text(text) = got else { continue };
            let v: Value = serde_json::from_str(&text).map_err(|e| {
                VcuError::with_detail(ErrorCode::ActionFailed, "cdp json", e.to_string())
            })?;
            if v.get("id").and_then(|x| x.as_u64()) == Some(id) {
                if let Some(err) = v.get("error") {
                    return Err(VcuError::with_detail(
                        ErrorCode::ActionFailed,
                        method,
                        err.to_string(),
                    ));
                }
                return Ok(v.get("result").cloned().unwrap_or(Value::Null));
            }
        }
    }

    async fn runtime_eval(&self, tab_id: &str, expression: &str) -> VcuResult<Value> {
        let page = self.page(tab_id)?;
        let ws = page.web_socket_debugger_url.clone();
        Self::cdp_call(
            &ws,
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

#[async_trait]
impl BrowserBackend for CdpBackend {
    fn name(&self) -> &str {
        "cdp"
    }

    async fn ensure_agent_window(&mut self) -> VcuResult<String> {
        self.refresh_pages().await?;
        Ok(self.agent_tab_id.clone())
    }

    async fn list_tabs(&self) -> VcuResult<Vec<TabInfo>> {
        Ok(self
            .pages
            .iter()
            .map(|p| TabInfo {
                tab_id: p.id.clone(),
                window_id: "cdp".into(),
                title: p.title.clone(),
                url: p.url.clone(),
                // CDP pages are treated as agent-writable in MVP (dedicated debug profile recommended).
                agent_owned: true,
                borrowed_by: None,
            })
            .collect())
    }

    async fn navigate(&mut self, tab_id: &str, url: &str) -> VcuResult<()> {
        let page = self.page(tab_id)?.clone();
        // Enable page events then navigate and wait for load.
        let _ = Self::cdp_call(
            &page.web_socket_debugger_url,
            "Page.enable",
            serde_json::json!({}),
        )
        .await;
        Self::cdp_call(
            &page.web_socket_debugger_url,
            "Page.navigate",
            serde_json::json!({"url": url}),
        )
        .await?;
        // Poll document readyState via Runtime.evaluate
        for _ in 0..40 {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            if let Ok(result) = self
                .runtime_eval(tab_id, "document.readyState")
                .await
            {
                let state = result
                    .pointer("/result/value")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if state == "complete" || state == "interactive" {
                    break;
                }
            }
        }
        // tiny settle for client-side renders
        tokio::time::sleep(std::time::Duration::from_millis(150)).await;
        self.refresh_pages().await?;
        Ok(())
    }

    async fn snapshot(
        &self,
        tab_id: &str,
        mode: SnapshotMode,
        budget_tokens: u64,
    ) -> VcuResult<SnapshotData> {
        let expr = r#"(function(){
          const refs=[];
          let i=0;
          const push=(el,role,name)=>{ const id='e'+(++i); el.setAttribute('data-vcu-ref', id);
            refs.push({ref:id, role:role, name:name||'', selector: el.tagName.toLowerCase()+'[data-vcu-ref=\"'+id+'\"]'}); };
          document.querySelectorAll('a,button,input,textarea,select,h1,h2,h3,[role]').forEach(el=>{
            const role = el.getAttribute('role') || el.tagName.toLowerCase();
            const name = el.getAttribute('aria-label') || el.innerText || el.value || '';
            push(el, role, (name||'').trim().slice(0,80));
          });
          return {
            title: document.title,
            url: location.href,
            text: (document.body && document.body.innerText || '').slice(0, 8000),
            refs
          };
        })()"#;
        let result = self.runtime_eval(tab_id, expr).await?;
        let value = result
            .pointer("/result/value")
            .cloned()
            .unwrap_or(Value::Null);
        let title = value.get("title").and_then(|v| v.as_str()).unwrap_or("");
        let url = value.get("url").and_then(|v| v.as_str()).unwrap_or("");
        let text = value
            .get("text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut refs = Vec::new();
        if let Some(arr) = value.get("refs").and_then(|v| v.as_array()) {
            for r in arr {
                refs.push(DomRef {
                    r#ref: r.get("ref").and_then(|v| v.as_str()).unwrap_or("").into(),
                    role: r.get("role").and_then(|v| v.as_str()).unwrap_or("").into(),
                    name: r.get("name").and_then(|v| v.as_str()).unwrap_or("").into(),
                    value: None,
                    selector: r
                        .get("selector")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                });
            }
        }
        let a11y = format!("document title=\"{title}\" url=\"{url}\" nodes={}", refs.len());
        let mut truncated = false;
        let est = (a11y.len() + text.len() + refs.len() * 24) as u64 / 4;
        let budget = if budget_tokens == 0 { u64::MAX } else { budget_tokens };
        let mut text = text;
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
        // Navigation may follow; wait briefly for settle.
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
        Ok(ActionResultDetail { ok: true, detail: value })
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
            selector: Some(selector.into()),
            count: matches.len(),
            matches,
        })
    }

    async fn screenshot(&self, tab_id: &str, _full_page: bool) -> VcuResult<ScreenshotData> {
        let page = self.page(tab_id)?;
        let result = Self::cdp_call(
            &page.web_socket_debugger_url,
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
        })
    }

    async fn act(&mut self, tab_id: &str, action: &ActionRequest) -> VcuResult<ActionResultDetail> {
        match action.r#type.as_str() {
            "click" => {
                let r = action
                    .target
                    .get("ref")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| VcuError::coded(ErrorCode::InvalidInput, "click requires target.ref"))?;
                self.click(tab_id, r).await
            }
            "type" => {
                let text = action
                    .args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| VcuError::coded(ErrorCode::InvalidInput, "type requires args.text"))?;
                let r = action.target.get("ref").and_then(|v| v.as_str());
                self.type_text(tab_id, text, r).await
            }
            "navigate" => {
                let url = action
                    .args
                    .get("url")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| VcuError::coded(ErrorCode::InvalidInput, "navigate requires args.url"))?;
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
