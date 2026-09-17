//! Desktop App computer-use adapters.
pub mod macos;
pub mod mock_app;
pub mod windows;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use vcu_core::{ErrorCode, VcuError, VcuResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTarget {
    pub id: String,
    pub title: String,
    pub bundle_or_exe: String,
    #[serde(default)]
    pub pid: Option<i32>,
    #[serde(default = "default_true")]
    pub allowed: bool,
}
fn default_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppElement {
    pub r#ref: String,
    pub role: String,
    pub name: String,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub target: AppTarget,
    pub summary: String,
    pub elements: Vec<AppElement>,
    pub truncated: bool,
}

#[async_trait]
pub trait AppBackend: Send + Sync {
    fn platform(&self) -> &str;
    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>>;
    /// Focus is explicit and opt-in; default backends may refuse.
    async fn focus_window(&mut self, id: &str, allow_focus_steal: bool) -> VcuResult<()>;
    async fn snapshot(&self, id: &str, budget: u64) -> VcuResult<AppSnapshot>;
    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value>;
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

    #[tokio::test]
    async fn detect_returns_platform_backend() {
        let b = detect_app_backend();
        assert!(!b.platform().is_empty());
    }
}
