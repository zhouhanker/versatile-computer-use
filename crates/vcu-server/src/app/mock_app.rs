//! Deterministic app backend for tests (no OS dependency).
use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};

use super::{AppBackend, AppElement, AppSnapshot, AppTarget};

pub struct MockAppBackend {
    pub targets: Vec<AppTarget>,
}

impl Default for MockAppBackend {
    fn default() -> Self {
        Self {
            targets: vec![AppTarget {
                id: "proc:TextEdit:1".into(),
                title: "TextEdit".into(),
                bundle_or_exe: "TextEdit".into(),
                pid: Some(1),
            }],
        }
    }
}

#[async_trait]
impl AppBackend for MockAppBackend {
    fn platform(&self) -> &str {
        "mock-app"
    }

    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>> {
        Ok(self.targets.clone())
    }

    async fn focus_window(&mut self, _id: &str, allow_focus_steal: bool) -> VcuResult<()> {
        if !allow_focus_steal {
            return Err(VcuError::coded(
                ErrorCode::FocusPolicyViolation,
                "focus steal denied",
            ));
        }
        Ok(())
    }

    async fn snapshot(&self, id: &str, _budget: u64) -> VcuResult<AppSnapshot> {
        let target = self
            .targets
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("app target {id}")))?;
        Ok(AppSnapshot {
            target: target.clone(),
            summary: format!("process=\"{}\" elements=2", target.title),
            elements: vec![
                AppElement {
                    r#ref: "e1".into(),
                    role: "button".into(),
                    name: "OK".into(),
                    value: None,
                },
                AppElement {
                    r#ref: "e2".into(),
                    role: "text".into(),
                    name: "Hello".into(),
                    value: Some("Hello".into()),
                },
            ],
            truncated: false,
        })
    }

    async fn invoke(&mut self, _id: &str, _element_ref: &str) -> VcuResult<serde_json::Value> {
        Err(VcuError::coded(
            ErrorCode::OsCursorDenied,
            "mock app invoke denied by policy",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_app_list_and_snapshot() {
        let mut b = MockAppBackend::default();
        let wins = b.list_windows().await.unwrap();
        assert_eq!(wins.len(), 1);
        let snap = b.snapshot(&wins[0].id, 1000).await.unwrap();
        assert_eq!(snap.elements.len(), 2);
        let err = b.focus_window(&wins[0].id, false).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        let err = b.invoke(&wins[0].id, "e1").await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::OsCursorDenied);
    }
}
