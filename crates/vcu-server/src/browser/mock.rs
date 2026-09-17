//! Deterministic in-process browser for POC and unit tests.
use std::collections::HashMap;

use async_trait::async_trait;
use base64::Engine;
use vcu_core::{
    new_id, ActionRequest, DomRef, ErrorCode, ExtractResult, SnapshotMode, TabInfo, VcuError,
    VcuResult,
};

use super::{ActionResultDetail, BrowserBackend, ScreenshotData, SnapshotData};

#[derive(Debug, Clone)]
struct MockNode {
    id: String,
    role: String,
    name: String,
    value: String,
    tag: String,
    text: String,
}

#[derive(Debug, Clone)]
struct MockTab {
    info: TabInfo,
    nodes: Vec<MockNode>,
    body_text: String,
}

pub struct MockBackend {
    tabs: HashMap<String, MockTab>,
    agent_window_id: String,
}

impl MockBackend {
    pub fn new() -> Self {
        let window_id = new_id();
        let tab_id = new_id();
        let mut tabs = HashMap::new();
        tabs.insert(
            tab_id.clone(),
            MockTab {
                info: TabInfo {
                    tab_id: tab_id.clone(),
                    window_id: window_id.clone(),
                    title: "Agent Window — New Tab".into(),
                    url: "about:blank".into(),
                    agent_owned: true,
                    borrowed_by: None,
                },
                nodes: default_blank_nodes(),
                body_text: String::new(),
            },
        );
        let user_tab = new_id();
        tabs.insert(
            user_tab.clone(),
            MockTab {
                info: TabInfo {
                    tab_id: user_tab,
                    window_id: new_id(),
                    title: "User Mail".into(),
                    url: "https://mail.example.com/inbox".into(),
                    agent_owned: false,
                    borrowed_by: None,
                },
                nodes: vec![MockNode {
                    id: "e1".into(),
                    role: "link".into(),
                    name: "Inbox".into(),
                    value: String::new(),
                    tag: "a".into(),
                    text: "Inbox".into(),
                }],
                body_text: "Inbox messages".into(),
            },
        );
        Self {
            tabs,
            agent_window_id: window_id,
        }
    }

    pub fn agent_tab_id(&self) -> VcuResult<String> {
        self.tabs
            .values()
            .find(|t| t.info.agent_owned)
            .map(|t| t.info.tab_id.clone())
            .ok_or_else(|| VcuError::coded(ErrorCode::Internal, "mock agent tab missing"))
    }

    fn load_fixture(url: &str) -> (String, Vec<MockNode>, String) {
        if url.contains("example.com") {
            let nodes = vec![
                MockNode {
                    id: "e1".into(),
                    role: "heading".into(),
                    name: "Example Domain".into(),
                    value: String::new(),
                    tag: "h1".into(),
                    text: "Example Domain".into(),
                },
                MockNode {
                    id: "e2".into(),
                    role: "link".into(),
                    name: "More information...".into(),
                    value: String::new(),
                    tag: "a".into(),
                    text: "More information...".into(),
                },
                MockNode {
                    id: "e3".into(),
                    role: "button".into(),
                    name: "Accept".into(),
                    value: String::new(),
                    tag: "button".into(),
                    text: "Accept".into(),
                },
                MockNode {
                    id: "e4".into(),
                    role: "textbox".into(),
                    name: "Search".into(),
                    value: String::new(),
                    tag: "input".into(),
                    text: String::new(),
                },
            ];
            let text = "Example Domain. This domain is for use in illustrative examples.".into();
            ("Example Domain".into(), nodes, text)
        } else if url.contains("form") {
            let nodes = vec![
                MockNode {
                    id: "e1".into(),
                    role: "textbox".into(),
                    name: "Email".into(),
                    value: String::new(),
                    tag: "input".into(),
                    text: String::new(),
                },
                MockNode {
                    id: "e2".into(),
                    role: "button".into(),
                    name: "Submit".into(),
                    value: String::new(),
                    tag: "button".into(),
                    text: "Submit".into(),
                },
            ];
            ("Form".into(), nodes, "Email form".into())
        } else {
            (url.to_string(), default_blank_nodes(), format!("Loaded {url}"))
        }
    }
}

fn default_blank_nodes() -> Vec<MockNode> {
    vec![MockNode {
        id: "e1".into(),
        role: "document".into(),
        name: "blank".into(),
        value: String::new(),
        tag: "body".into(),
        text: String::new(),
    }]
}

impl Default for MockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BrowserBackend for MockBackend {
    fn name(&self) -> &str {
        "mock"
    }

    async fn ensure_agent_window(&mut self) -> VcuResult<String> {
        Ok(self.agent_window_id.clone())
    }

    async fn list_tabs(&self) -> VcuResult<Vec<TabInfo>> {
        let mut v: Vec<_> = self.tabs.values().map(|t| t.info.clone()).collect();
        v.sort_by(|a, b| a.tab_id.cmp(&b.tab_id));
        Ok(v)
    }

    async fn navigate(&mut self, tab_id: &str, url: &str) -> VcuResult<()> {
        let tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))?;
        let (title, nodes, text) = Self::load_fixture(url);
        tab.info.url = url.to_string();
        tab.info.title = title;
        tab.nodes = nodes;
        tab.body_text = text;
        Ok(())
    }

    async fn snapshot(
        &self,
        tab_id: &str,
        mode: SnapshotMode,
        budget_tokens: u64,
    ) -> VcuResult<SnapshotData> {
        let tab = self
            .tabs
            .get(tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))?;
        let mut refs: Vec<DomRef> = tab
            .nodes
            .iter()
            .map(|n| DomRef {
                r#ref: n.id.clone(),
                role: n.role.clone(),
                name: n.name.clone(),
                value: if n.value.is_empty() {
                    None
                } else {
                    Some(n.value.clone())
                },
                selector: Some(format!("{}#{}", n.tag, n.id)),
            })
            .collect();

        let a11y = format!(
            "document title=\"{}\" url=\"{}\" nodes={}",
            tab.info.title,
            tab.info.url,
            refs.len()
        );
        let mut text = tab.body_text.clone();
        let mut truncated = false;
        let est = (a11y.len() + text.len() + refs.len() * 24) as u64 / 4;
        let budget = if budget_tokens == 0 { u64::MAX } else { budget_tokens };
        if est > budget {
            truncated = true;
            let keep = (budget as usize).saturating_mul(4).min(text.len());
            text.truncate(keep);
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
            screenshot_png: Some(minimal_png()),
            truncated,
            budget_tokens_est: est.min(budget),
        })
    }

    async fn click(&mut self, tab_id: &str, target_ref: &str) -> VcuResult<ActionResultDetail> {
        let tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))?;
        let node = tab
            .nodes
            .iter()
            .find(|n| n.id == target_ref)
            .ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, format!("unknown ref {target_ref}"))
            })?;
        Ok(ActionResultDetail {
            ok: true,
            detail: serde_json::json!({
                "clicked": target_ref,
                "role": node.role,
                "name": node.name,
                "input_path": "page_dom",
                "os_cursor_used": false
            }),
        })
    }

    async fn type_text(
        &mut self,
        tab_id: &str,
        text: &str,
        target_ref: Option<&str>,
    ) -> VcuResult<ActionResultDetail> {
        let tab = self
            .tabs
            .get_mut(tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))?;
        if let Some(r) = target_ref {
            if let Some(node) = tab.nodes.iter_mut().find(|n| n.id == r) {
                node.value = text.to_string();
            } else {
                return Err(VcuError::coded(
                    ErrorCode::InvalidInput,
                    format!("unknown ref {r}"),
                ));
            }
        }
        Ok(ActionResultDetail {
            ok: true,
            detail: serde_json::json!({
                "typed": text,
                "ref": target_ref,
                "input_path": "page_dom",
                "os_cursor_used": false
            }),
        })
    }

    async fn extract(&self, tab_id: &str, selector: &str) -> VcuResult<ExtractResult> {
        let tab = self
            .tabs
            .get(tab_id)
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")))?;
        let matches: Vec<serde_json::Value> = tab
            .nodes
            .iter()
            .filter(|n| {
                selector == "*"
                    || n.tag == selector
                    || n.role == selector
                    || n.id == selector
                    || selector.strip_prefix('#').is_some_and(|id| id == n.id)
            })
            .map(|n| {
                serde_json::json!({
                    "ref": n.id,
                    "role": n.role,
                    "name": n.name,
                    "value": n.value,
                    "text": n.text,
                })
            })
            .collect();
        Ok(ExtractResult {
            session_id: String::new(),
            selector: Some(selector.into()),
            count: matches.len(),
            matches,
        })
    }

    async fn screenshot(&self, tab_id: &str, _full_page: bool) -> VcuResult<ScreenshotData> {
        if !self.tabs.contains_key(tab_id) {
            return Err(VcuError::coded(ErrorCode::TabNotFound, format!("tab {tab_id}")));
        }
        Ok(ScreenshotData {
            png: minimal_png(),
            width: 1,
            height: 1,
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
                format!("action not implemented in mock: {other}"),
            )),
        }
    }
}

fn minimal_png() -> Vec<u8> {
    Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
    )
    .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn navigate_and_snapshot_example() {
        let mut b = MockBackend::new();
        let tab = b.agent_tab_id().unwrap();
        b.navigate(&tab, "https://example.com/").await.unwrap();
        let snap = b.snapshot(&tab, SnapshotMode::Full, 5000).await.unwrap();
        assert!(snap.a11y_summary.unwrap().contains("Example Domain"));
        assert!(snap.dom_refs.iter().any(|r| r.r#ref == "e3"));
    }

    #[tokio::test]
    async fn os_cursor_denied() {
        let mut b = MockBackend::new();
        let tab = b.agent_tab_id().unwrap();
        let act = ActionRequest {
            r#type: "os_cursor_move".into(),
            target: serde_json::json!({}),
            args: serde_json::json!({"x": 1, "y": 2}),
            idempotency_key: None,
        };
        let err = b.act(&tab, &act).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::OsCursorDenied);
    }
}
