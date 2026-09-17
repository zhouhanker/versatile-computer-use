use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::config::VisionPolicy;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    Browser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BrowserKind {
    Chrome,
    Edge,
    Auto,
    Mock,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    /// In-process simulated browser for tests/POC without Chrome.
    Mock,
    /// Chrome DevTools Protocol attach.
    Cdp,
    /// Extension-connected real browser (Agent Window path).
    Extension,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FocusPolicy {
    AgentWindowOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OsCursorPolicy {
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionPolicy {
    pub focus: FocusPolicy,
    pub os_cursor: OsCursorPolicy,
    pub borrow_required_for_user_tabs: bool,
}

impl Default for SessionPolicy {
    fn default() -> Self {
        Self {
            focus: FocusPolicy::AgentWindowOnly,
            os_cursor: OsCursorPolicy::Deny,
            borrow_required_for_user_tabs: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub session_id: String,
    pub adapter: AdapterKind,
    pub browser: BrowserKind,
    pub backend: BackendKind,
    pub policy: SessionPolicy,
    pub vision_policy: VisionPolicy,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub revision: u64,
    pub active_tab_id: Option<String>,
    pub agent_window_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TabInfo {
    pub tab_id: String,
    pub window_id: String,
    pub title: String,
    pub url: String,
    /// true = Agent Window tab (writable without borrow)
    pub agent_owned: bool,
    pub borrowed_by: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotMode {
    A11y,
    Dom,
    Text,
    Full,
}

impl SnapshotMode {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "a11y" => Some(Self::A11y),
            "dom" => Some(Self::Dom),
            "text" => Some(Self::Text),
            "full" => Some(Self::Full),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomRef {
    pub r#ref: String,
    pub role: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisionInfo {
    pub used: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub observation_id: String,
    pub session_id: String,
    pub kind: String,
    pub targets: Vec<TabInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub a11y_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dom_refs: Vec<DomRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_excerpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub screenshot_ref: Option<String>,
    pub vision: VisionInfo,
    pub truncated: bool,
    pub budget_tokens_est: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    pub r#type: String,
    #[serde(default)]
    pub target: serde_json::Value,
    #[serde(default)]
    pub args: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub action_id: String,
    pub session_id: String,
    pub r#type: String,
    pub ok: bool,
    #[serde(default)]
    pub detail: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Blackboard {
    pub session_id: String,
    pub revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dom_summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_summary: Option<String>,
    #[serde(default)]
    pub candidates: Vec<DomRef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_action_hint: Option<String>,
    #[serde(default)]
    pub open_loops: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorCheck {
    pub name: String,
    pub status: String, // pass | warn | fail
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub checks: Vec<DoctorCheck>,
    pub user_dir: String,
    pub daemon: DaemonStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub running: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractResult {
    pub session_id: String,
    pub selector: Option<String>,
    pub matches: Vec<serde_json::Value>,
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_denies_os_cursor() {
        let p = SessionPolicy::default();
        assert!(matches!(p.os_cursor, OsCursorPolicy::Deny));
        assert!(p.borrow_required_for_user_tabs);
    }

    #[test]
    fn snapshot_mode_parse() {
        assert_eq!(SnapshotMode::parse("a11y"), Some(SnapshotMode::A11y));
        assert_eq!(SnapshotMode::parse("nope"), None);
    }
}
