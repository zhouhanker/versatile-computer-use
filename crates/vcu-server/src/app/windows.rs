//! Windows app adapter (PowerShell list on Windows hosts).
use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};
use super::{AppBackend, AppSnapshot, AppTarget};
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
            let mut out = Vec::new();
            for (idx, line) in raw.lines().enumerate() {
                let mut parts = line.split('\t');
                let name = parts.next().unwrap_or("").trim();
                let pid = parts.next().and_then(|s| s.trim().parse().ok());
                let title = parts.next().unwrap_or(name).trim();
                if name.is_empty() { continue; }
                if !self.allowed(name) && !self.allowed(title) { continue; }
                out.push(AppTarget {
                    id: format!("win:{}:{}", name.replace(' ', "_"), pid.unwrap_or(idx as i32)),
                    title: if title.is_empty() { name.to_string() } else { title.to_string() },
                    bundle_or_exe: name.to_string(),
                    pid,
                    allowed: true,
                });
            }
            Ok(out)
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
            }];
            if budget > 0 && budget < 10 { elements.clear(); }
            Ok(AppSnapshot {
                target: AppTarget { id: id.to_string(), title: elements.first().map(|e| e.name.clone()).unwrap_or_else(|| name.clone()), bundle_or_exe: name.clone(), pid: None, allowed: true },
                summary: format!("process=\"{name}\" elements={} note=uia_tree_mvp_title_only", elements.len()),
                elements,
                truncated: true,
            })
        }
    }

    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value> {
        let _ = (id, element_ref);
        Err(VcuError::coded(ErrorCode::OsCursorDenied, "Windows app invoke is denied by default safety policy (no SendInput / no cursor)"))
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
    }
}
