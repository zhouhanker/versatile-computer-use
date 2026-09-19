//! Windows app adapter (PowerShell list on Windows hosts).
use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};
use super::{is_denied_app, AppBackend, AppSnapshot, AppTarget};
#[cfg(windows)]
use super::AppElement;

pub struct WindowsAppBackend {
    pub allowlist: Vec<String>,
}

impl WindowsAppBackend {
    pub fn new() -> Self {
        Self {
            allowlist: vec![
                "notepad".into(),
                "explorer".into(),
                "msedge".into(),
                "chrome".into(),
                "windows terminal".into(),
                "powershell".into(),
                "cmd".into(),
            ],
        }
    }

    #[allow(dead_code)]
    fn allowed(&self, name: &str) -> bool {
        if is_denied_app(name) {
            return false;
        }
        if self.allowlist.is_empty() {
            return true;
        }
        let lower = name.to_lowercase();
        self.allowlist.iter().any(|a| lower.contains(&a.to_lowercase()))
    }

    #[cfg(windows)]
    fn run_powershell(script: &str) -> VcuResult<String> {
        use std::process::Command;
        let output = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .output()
            .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "powershell spawn failed", e.to_string()))?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "powershell failed", err));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

/// Parse `name\tpid\ttitle` lines from the Windows process listing script.
pub fn parse_process_list_lines(raw: &str, allowed: impl Fn(&str) -> bool) -> Vec<AppTarget> {
    let mut out = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let mut parts = line.split('\t');
        let name = parts.next().unwrap_or("").trim();
        let pid = parts.next().and_then(|s| s.trim().parse().ok());
        let title = parts.next().unwrap_or(name).trim();
        if name.is_empty() {
            continue;
        }
        if is_denied_app(name) || is_denied_app(title) {
            continue;
        }
        if !allowed(name) && !allowed(title) {
            continue;
        }
        out.push(AppTarget {
            id: format!("win:{}:{}", name.replace(' ', "_"), pid.unwrap_or(idx as i32)),
            title: if title.is_empty() {
                name.to_string()
            } else {
                title.to_string()
            },
            bundle_or_exe: name.to_string(),
            pid,
            allowed: true,
            browser_profile: None,
        });
    }
    out
}

impl Default for WindowsAppBackend {
    fn default() -> Self { Self::new() }
}

#[async_trait]
impl AppBackend for WindowsAppBackend {
    fn platform(&self) -> &str { "windows" }

    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>> {
        #[cfg(not(windows))]
        {
            Err(VcuError::coded(ErrorCode::NotImplemented, "WindowsAppBackend::list_windows only runs on Windows hosts"))
        }
        #[cfg(windows)]
        {
            let script = r#"
Get-Process | Where-Object { $_.MainWindowTitle -ne '' } |
  ForEach-Object { '{0}`t{1}`t{2}' -f $_.ProcessName, $_.Id, ($_.MainWindowTitle -replace '[\r\n\t]',' ') }
"#;
            let raw = Self::run_powershell(script)?;
            Ok(parse_process_list_lines(&raw, |n| self.allowed(n)))
        }
    }

    async fn focus_window(&mut self, _id: &str, allow_focus_steal: bool) -> VcuResult<()> {
        if !allow_focus_steal {
            return Err(VcuError::coded(ErrorCode::FocusPolicyViolation, "refusing to steal app focus on Windows"));
        }
        Err(VcuError::coded(ErrorCode::NotImplemented, "explicit Windows focus is gated"))
    }

    async fn snapshot(&self, id: &str, budget: u64) -> VcuResult<AppSnapshot> {
        #[cfg(not(windows))]
        {
            let _ = budget;
            Err(VcuError::coded(ErrorCode::NotImplemented, format!("Windows snapshot unavailable on this host for {id}")))
        }
        #[cfg(windows)]
        {
            let name = id.strip_prefix("win:").and_then(|rest| rest.split(':').next()).map(|s| s.replace('_', " ")).unwrap_or_else(|| id.to_string());
            if !self.allowed(&name) {
                return Err(VcuError::coded(ErrorCode::FocusPolicyViolation, format!("process '{name}' not in app allowlist")));
            }
            let script = format!(
                "$p = Get-Process -Name '{name}' -ErrorAction SilentlyContinue | Where-Object {{ $_.MainWindowTitle -ne '' }} | Select-Object -First 1; if ($null -eq $p) {{ 'MISSING' }} else {{ $p.MainWindowTitle }}",
                name = name.replace('\'', "")
            );
            let title = Self::run_powershell(&script).unwrap_or_default();
            let mut elements = vec![AppElement {
                r#ref: "e1".into(),
                role: "window".into(),
                name: if title.is_empty() || title == "MISSING" { name.clone() } else { title },
                value: None,
                frame: None,
            }];
            if budget > 0 && budget < 10 { elements.clear(); }
            Ok(AppSnapshot {
                target: AppTarget { id: id.to_string(), title: elements.first().map(|e| e.name.clone()).unwrap_or_else(|| name.clone()), bundle_or_exe: name.clone(), pid: None, allowed: true, browser_profile: None },
                summary: format!("process=\"{name}\" elements={} note=uia_tree_mvp_title_only", elements.len()),
                elements,
                truncated: true,
                window_frame: None,
                webview: false,
                webview_ref: None,
                page_title: None,
                page_url: None,
                tabs: vec![],
                ax_enhanced: false,
            })
        }
    }

    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value> {
        let _ = (id, element_ref);
        Err(VcuError::coded(ErrorCode::OsCursorDenied, "Windows app invoke is denied by default safety policy (no SendInput / no cursor)"))
    }

    async fn set_value(&mut self, id: &str, element_ref: &str, value: &str) -> VcuResult<serde_json::Value> {
        let _ = (id, element_ref, value);
        Err(VcuError::coded(ErrorCode::NotImplemented, "Windows app set_value is not implemented in slice 1"))
    }

    async fn capture_window(&self, id: &str) -> VcuResult<Option<super::AppCapture>> {
        let _ = id;
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn windows_backend_platform_and_denials() {
        let mut b = WindowsAppBackend::new();
        assert_eq!(b.platform(), "windows");
        let err = b.focus_window("win:x:1", false).await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::FocusPolicyViolation);
        let err = b.invoke("win:x:1", "e1").await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::OsCursorDenied);
        assert!(err.message().to_ascii_lowercase().contains("sendinput") || err.message().to_ascii_lowercase().contains("cursor"));
        let err = b.set_value("win:notepad:1", "e1", "hi").await.unwrap_err();
        assert_eq!(err.code(), ErrorCode::NotImplemented);
        let cap = b.capture_window("win:notepad:1").await.unwrap();
        assert!(cap.is_none());
    }

    #[test]
    fn parse_process_list_keeps_allowlist_drops_wechat() {
        let b = WindowsAppBackend::new();
        let raw = "notepad\t1001\tUntitled - Notepad\nWeChat\t2002\tWeChat\nexplorer\t3003\tDocuments\nmsedge\t4004\tMicrosoft Edge\n";
        let wins = parse_process_list_lines(raw, |n| b.allowed(n));
        let names: Vec<_> = wins.iter().map(|w| w.bundle_or_exe.as_str()).collect();
        assert!(names.contains(&"notepad"));
        assert!(names.contains(&"explorer"));
        assert!(names.contains(&"msedge"));
        assert!(!names.iter().any(|n| n.to_lowercase().contains("wechat")));
        assert!(wins.iter().any(|w| w.id.starts_with("win:notepad:")));
    }

    #[cfg(not(windows))]
    #[tokio::test]
    async fn windows_list_and_snapshot_are_host_gated_off_windows() {
        let b = WindowsAppBackend::new();
        let list = b.list_windows().await.unwrap_err();
        assert_eq!(list.code(), ErrorCode::NotImplemented);
        let snap = b.snapshot("win:notepad:1", 1000).await.unwrap_err();
        assert_eq!(snap.code(), ErrorCode::NotImplemented);
    }
}
