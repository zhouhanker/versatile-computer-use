//! Deterministic app backend for tests (no OS dependency).
use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};

use super::{denied_app_error, is_denied_app, webview_hint, AppBackend, AppCapture, AppElement, AppSnapshot, AppTarget};

pub struct MockAppBackend {
    pub targets: Vec<AppTarget>,
}

impl Default for MockAppBackend {
    fn default() -> Self {
        Self {
            targets: vec![
                AppTarget {
                    id: "proc:TextEdit:1".into(),
                    title: "TextEdit".into(),
                    bundle_or_exe: "TextEdit".into(),
                    pid: Some(1),
                    allowed: true,
                    browser_profile: None,
                },
                AppTarget {
                    id: "proc:WeChat:2".into(),
                    title: "WeChat".into(),
                    bundle_or_exe: "WeChat".into(),
                    pid: Some(2),
                    allowed: false,
                    browser_profile: None,
                },
                AppTarget {
                    id: "proc:Feishu:3".into(),
                    title: "Feishu".into(),
                    bundle_or_exe: "Feishu".into(),
                    pid: Some(3),
                    allowed: true,
                    browser_profile: None,
                },
                AppTarget {
                    id: "proc:Finder:4".into(),
                    title: "Finder".into(),
                    bundle_or_exe: "Finder".into(),
                    pid: Some(4),
                    allowed: true,
                    browser_profile: None,
                },
                AppTarget {
                    id: "proc:Microsoft_Edge:10".into(),
                    title: "Microsoft Edge".into(),
                    bundle_or_exe: "Microsoft Edge".into(),
                    pid: Some(10),
                    allowed: true,
                    browser_profile: Some("user".into()),
                },
                AppTarget {
                    id: "proc:Microsoft_Edge:11".into(),
                    title: "Microsoft Edge".into(),
                    bundle_or_exe: "Microsoft Edge".into(),
                    pid: Some(11),
                    allowed: true,
                    browser_profile: Some("agent".into()),
                },
            ],
        }
    }
}

impl MockAppBackend {
    fn target(&self, id: &str) -> VcuResult<AppTarget> {
        self.targets
            .iter()
            .find(|t| t.id == id)
            .cloned()
            .ok_or_else(|| VcuError::coded(ErrorCode::TabNotFound, format!("app target {id}")))
    }

    fn ensure_operable(&self, target: &AppTarget) -> VcuResult<()> {
        if is_denied_app(&target.title) || is_denied_app(&target.bundle_or_exe) || !target.allowed {
            return Err(denied_app_error(&target.title));
        }
        Ok(())
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
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        let elements = if target.title.eq_ignore_ascii_case("Feishu")
            || target.title.eq_ignore_ascii_case("Lark")
        {
            vec![
                AppElement {
                    r#ref: "e1".into(),
                    role: "AXButton".into(),
                    name: "搜索（⌘＋K）".into(),
                    value: None,
                    frame: Some([0.0, 0.0, 163.0, 32.0]),
                },
                AppElement {
                    r#ref: "e14".into(),
                    role: "AXGroup".into(),
                    name: "MultiWebView - messenger".into(),
                    value: None,
                    frame: Some([1907.0, 36.0, 305.0, 1038.0]),
                },
                AppElement {
                    r#ref: "e15".into(),
                    role: "AXGroup".into(),
                    name: "MultiWebView - messenger:messenger-chat:default".into(),
                    value: None,
                    frame: Some([2218.0, 36.0, 1424.0, 1038.0]),
                },
                AppElement {
                    r#ref: "e_send".into(),
                    role: "AXButton".into(),
                    name: "发送".into(),
                    value: None,
                    frame: Some([3500.0, 1000.0, 80.0, 28.0]),
                },
            ]
        } else if target.browser_profile.as_deref() == Some("user") {
            vec![
                AppElement {
                    r#ref: "e_web".into(),
                    role: "AXWebArea".into(),
                    name: "AXWebArea".into(),
                    value: None,
                    frame: Some([0.0, 80.0, 800.0, 520.0]),
                },
            ]
        } else {
            vec![
                AppElement {
                    r#ref: "e1".into(),
                    role: "button".into(),
                    name: "OK".into(),
                    value: None,
                    frame: Some([20.0, 20.0, 80.0, 24.0]),
                },
                AppElement {
                    r#ref: "e2".into(),
                    role: "text".into(),
                    name: "Hello".into(),
                    value: Some("Hello".into()),
                    frame: Some([20.0, 60.0, 240.0, 20.0]),
                },
            ]
        };
        let (webview, webview_ref) = webview_hint(&elements);
        Ok(AppSnapshot {
            target: target.clone(),
            summary: format!(
                "process=\"{}\" elements={} webview={webview}",
                target.title,
                elements.len()
            ),
            elements,
            truncated: false,
            window_frame: Some([0.0, 0.0, 800.0, 600.0]),
            webview,
            webview_ref,
            page_title: None,
            page_url: None,
            tabs: vec![],
            ax_enhanced: false,
        })
    }

    async fn press_at_point(&mut self, id: &str, x: f64, y: f64) -> VcuResult<serde_json::Value> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        let snap = self.snapshot(id, 1000).await?;
        let refs: Vec<(String, [f64; 4])> = snap
            .elements
            .iter()
            .filter_map(|e| e.frame.map(|f| (e.r#ref.clone(), f)))
            .collect();
        let hit = super::smallest_ref_at_point(&refs, x, y).ok_or_else(|| {
            VcuError::coded(
                ErrorCode::InvalidInput,
                format!("no mock element at ({x},{y})"),
            )
        })?;
        let mut detail = self.invoke(id, &hit).await?;
        detail["input_path"] = serde_json::json!("ax_position_press");
        detail["hit_ref"] = serde_json::json!(hit);
        detail["ax_point"] = serde_json::json!({"x": x, "y": y});
        Ok(detail)
    }

    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        let snap = self.snapshot(id, 1000).await?;
        let el = snap.elements.iter().find(|e| e.r#ref == element_ref);
        let webview = el
            .map(|e| super::is_webview_like(&e.role, &e.name))
            .unwrap_or(false);
        let result = if webview { "ok_webview" } else { "ok" };
        let input_path = if webview { "ax_frame_hit" } else { "ax_press" };
        Ok(serde_json::json!({
            "ok": true,
            "process": target.title,
            "ref": element_ref,
            "result": result,
            "input_path": input_path,
            "os_cursor_used": false,
            "hid_injected": false
        }))
    }

    async fn set_value(&mut self, id: &str, element_ref: &str, value: &str) -> VcuResult<serde_json::Value> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        Ok(serde_json::json!({
            "ok": true,
            "process": target.title,
            "ref": element_ref,
            "value": value,
            "input_path": "ax_set_value",
            "os_cursor_used": false
        }))
    }

    async fn reveal_path(&mut self, id: &str, path: &str) -> VcuResult<serde_json::Value> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        if !target.title.eq_ignore_ascii_case("Finder") {
            return Err(VcuError::coded(
                ErrorCode::InvalidInput,
                "reveal_path is Finder-only",
            ));
        }
        Ok(serde_json::json!({
            "ok": true,
            "process": target.title,
            "path": path,
            "input_path": "nsworkspace_reveal",
            "os_cursor_used": false,
            "hid_injected": false
        }))
    }

    async fn open_path(&mut self, id: &str, path: &str) -> VcuResult<serde_json::Value> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        if !target.title.eq_ignore_ascii_case("Finder") {
            return Err(VcuError::coded(
                ErrorCode::InvalidInput,
                "open_path is Finder-only",
            ));
        }
        Ok(serde_json::json!({
            "ok": true,
            "process": target.title,
            "path": path,
            "input_path": "nsworkspace_open",
            "os_cursor_used": false,
            "hid_injected": false
        }))
    }

    async fn scroll(
        &mut self,
        id: &str,
        element_ref: Option<&str>,
        dy: i32,
    ) -> VcuResult<serde_json::Value> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        Ok(serde_json::json!({
            "ok": true,
            "process": target.title,
            "ref": element_ref,
            "dy": dy,
            "input_path": "ax_scroll",
            "os_cursor_used": false
        }))
    }

    async fn capture_window(&self, id: &str) -> VcuResult<Option<AppCapture>> {
        let target = self.target(id)?;
        self.ensure_operable(&target)?;
        let frame = if target.title.eq_ignore_ascii_case("Feishu")
            || target.title.eq_ignore_ascii_case("Lark")
        {
            [1907.0, 36.0, 1735.0, 1038.0]
        } else {
            [0.0, 0.0, 800.0, 600.0]
        };
        Ok(Some(AppCapture {
            png: MIN_PNG.to_vec(),
            width: frame[2] as u32,
            height: frame[3] as u32,
            frame,
        }))
    }

    async fn capture_rect(&self, frame: [f64; 4]) -> VcuResult<Option<AppCapture>> {
        Ok(Some(AppCapture {
            png: MIN_PNG.to_vec(),
            width: frame[2].max(1.0) as u32,
            height: frame[3].max(1.0) as u32,
            frame,
        }))
    }
}

const MIN_PNG: &[u8] = &[
    0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48, 0x44, 0x52,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00, 0x00, 0x1f, 0x15, 0xc4,
    0x89, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x44, 0x41, 0x54, 0x78, 0xda, 0x63, 0x60, 0x00, 0x02, 0x00,
    0x00, 0x05, 0x00, 0x01, 0xd5, 0xc8, 0x4d, 0x6e, 0x00, 0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44,
    0xae, 0x42, 0x60, 0x82,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_app_list_and_snapshot() {
        let mut b = MockAppBackend::default();
        let wins = b.list_windows().await.unwrap();
        assert_eq!(wins.len(), 6);
        let snap = b.snapshot(&wins[0].id, 1000).await.unwrap();
        assert_eq!(snap.elements.len(), 2);
        assert_eq!(snap.elements[0].frame, Some([20.0, 20.0, 80.0, 24.0]));
        let cap = b.capture_window(&wins[0].id).await.unwrap().unwrap();
        assert!(!cap.png.is_empty());
        let err = b.focus_window(&wins[0].id, false).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        let pressed = b.invoke(&wins[0].id, "e1").await.unwrap();
        assert_eq!(pressed["os_cursor_used"], false);
        let typed = b.set_value(&wins[0].id, "e2", "hi").await.unwrap();
        assert_eq!(typed["input_path"], "ax_set_value");
        let finder = wins.iter().find(|w| w.title == "Finder").unwrap();
        let revealed = b.reveal_path(&finder.id, "/tmp/VCU-D-040").await.unwrap();
        assert_eq!(revealed["input_path"], "nsworkspace_reveal");
        let opened = b.open_path(&finder.id, "/tmp/VCU-D-040").await.unwrap();
        assert_eq!(opened["input_path"], "nsworkspace_open");
        assert!(b.open_path(&wins[0].id, "/tmp/VCU-D-040").await.is_err());
        let wechat = wins.iter().find(|w| w.title == "WeChat").unwrap();
        let err = b.snapshot(&wechat.id, 1000).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::AppDenied);
        let err = b.invoke(&wechat.id, "e1").await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::AppDenied);
        let feishu = wins.iter().find(|w| w.title == "Feishu").unwrap();
        let fs = b.snapshot(&feishu.id, 1000).await.unwrap();
        assert!(fs.webview);
        assert_eq!(fs.webview_ref.as_deref(), Some("e15"));
        let hit = b.invoke(&feishu.id, "e15").await.unwrap();
        assert_eq!(hit["input_path"], "ax_frame_hit");
        assert_eq!(hit["os_cursor_used"], false);
        assert_eq!(hit["hid_injected"], false);
    }
}
