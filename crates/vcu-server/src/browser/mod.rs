pub mod cdp;
pub mod extension;
pub mod mock;

use async_trait::async_trait;
use vcu_core::{ActionRequest, DomRef, ExtractResult, SnapshotMode, TabInfo, VcuResult};

#[async_trait]
pub trait BrowserBackend: Send + Sync {
    fn name(&self) -> &str;
    async fn ensure_agent_window(&mut self) -> VcuResult<String>;
    async fn list_tabs(&self) -> VcuResult<Vec<TabInfo>>;
    async fn navigate(&mut self, tab_id: &str, url: &str) -> VcuResult<()>;
    async fn snapshot(
        &self,
        tab_id: &str,
        mode: SnapshotMode,
        budget_tokens: u64,
    ) -> VcuResult<SnapshotData>;
    async fn click(&mut self, tab_id: &str, target_ref: &str) -> VcuResult<ActionResultDetail>;
    async fn type_text(
        &mut self,
        tab_id: &str,
        text: &str,
        target_ref: Option<&str>,
    ) -> VcuResult<ActionResultDetail>;
    async fn extract(&self, tab_id: &str, selector: &str) -> VcuResult<ExtractResult>;
    async fn screenshot(&self, tab_id: &str, full_page: bool) -> VcuResult<ScreenshotData>;
    async fn act(&mut self, tab_id: &str, action: &ActionRequest) -> VcuResult<ActionResultDetail>;
}

#[derive(Debug, Clone)]
pub struct SnapshotData {
    pub a11y_summary: Option<String>,
    pub dom_refs: Vec<DomRef>,
    pub text_excerpt: Option<String>,
    pub screenshot_png: Option<Vec<u8>>,
    pub truncated: bool,
    pub budget_tokens_est: u64,
}

#[derive(Debug, Clone)]
pub struct ActionResultDetail {
    pub ok: bool,
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone)]
pub struct ScreenshotData {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

pub use mock::MockBackend;

pub use extension::{ExtensionBackend, ExtensionBridge};
