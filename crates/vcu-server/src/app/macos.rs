//! macOS app adapter via osascript / System Events (AX).
//! Default policy: list + snapshot only; focus steal and synthetic cursor denied.
use std::process::Command;

use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};

use super::{AppBackend, AppElement, AppSnapshot, AppTarget};

pub struct MacosAppBackend {
    /// Optional allowlist of process names (case-insensitive contains match).
    pub allowlist: Vec<String>,
}

impl MacosAppBackend {
    pub fn new() -> Self {
        Self::with_allowlist(None)
    }

    pub fn with_allowlist(extra: Option<Vec<String>>) -> Self {
        let mut allowlist = vec![
            "TextEdit".into(),
            "Notes".into(),
            "Safari".into(),
            "Terminal".into(),
            "Ghostty".into(),
            "Finder".into(),
            "TextEdit".into(),
            "Preview".into(),
            "Code".into(),
            "Cursor".into(),
        ];
        if let Some(extra) = extra {
            for e in extra {
                if !e.trim().is_empty() && !allowlist.iter().any(|a: &String| a.eq_ignore_ascii_case(&e)) {
                    allowlist.push(e);
                }
            }
        }
        if let Ok(envl) = std::env::var("VCU_APP_ALLOWLIST") {
            for e in envl.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                if !allowlist.iter().any(|a: &String| a.eq_ignore_ascii_case(e)) {
                    allowlist.push(e.to_string());
                }
            }
        }
        Self { allowlist }
    }

    fn run_osascript(script: &str) -> VcuResult<String> {
        let output = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output()
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::Internal, "osascript spawn failed", e.to_string())
            })?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            // Common: missing Accessibility permission
            if err.to_lowercase().contains("not allowed")
                || err.to_lowercase().contains("assistive")
                || err.to_lowercase().contains("1002")
            {
                return Err(VcuError::with_detail(
                    ErrorCode::BrowserUnavailable, // reuse coded surface; repair points to privacy settings
                    "macOS Accessibility permission missing for osascript/System Events",
                    err,
                ));
            }
            return Err(VcuError::with_detail(
                ErrorCode::ActionFailed,
                "osascript failed",
                err,
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn allowed(&self, name: &str) -> bool {
        if self.allowlist.is_empty() {
            return true;
        }
        let lower = name.to_lowercase();
        self.allowlist
            .iter()
            .any(|a| lower.contains(&a.to_lowercase()))
    }
}

impl Default for MacosAppBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AppBackend for MacosAppBackend {
    fn platform(&self) -> &str {
        "macos"
    }

    async fn list_windows(&self) -> VcuResult<Vec<AppTarget>> {
        // Process list does not require full UI hierarchy access.
        let script = r#"
        tell application "System Events"
          set procs to every process whose background only is false
          set out to {}
          repeat with p in procs
            set end of out to (name of p as text) & tab & (unix id of p as text)
          end repeat
          set AppleScript's text item delimiters to linefeed
          return out as text
        end tell
        "#;
        let raw = Self::run_osascript(script)?;
        let mut targets = Vec::new();
        for (idx, line) in raw.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut parts = line.split('\t');
            let name = parts.next().unwrap_or("").trim();
            let pid: Option<i32> = parts.next().and_then(|s| s.trim().parse().ok());
            if name.is_empty() || !self.allowed(name) {
                continue;
            }
            targets.push(AppTarget {
                id: format!("proc:{}:{}", name.replace(' ', "_"), pid.unwrap_or(idx as i32)),
                title: name.to_string(),
                bundle_or_exe: name.to_string(),
                pid,
            });
        }
        Ok(targets)
    }

    async fn focus_window(&mut self, _id: &str, allow_focus_steal: bool) -> VcuResult<()> {
        if !allow_focus_steal {
            return Err(VcuError::coded(
                ErrorCode::FocusPolicyViolation,
                "refusing to steal app focus; pass allow_focus_steal=true only with user intent",
            ));
        }
        Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "explicit focus is gated; prefer non-focus snapshot paths",
        ))
    }

    async fn snapshot(&self, id: &str, budget: u64) -> VcuResult<AppSnapshot> {
        let name = id
            .strip_prefix("proc:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if !self.allowed(&name) {
            return Err(VcuError::coded(
                ErrorCode::FocusPolicyViolation,
                format!("process '{name}' not in app allowlist"),
            ));
        }
        // Best-effort UI element names; may fail without Accessibility.
        let script = format!(
            r#"
            tell application "System Events"
              tell process "{name}"
                set elems to {{}}
                try
                  set uiElems to entire contents
                  set n to 0
                  repeat with e in uiElems
                    set n to n + 1
                    if n > 80 then exit repeat
                    set r to ""
                    set nm to ""
                    try
                      set r to (role of e as text)
                    end try
                    try
                      set nm to (name of e as text)
                    end try
                    if nm is not "" then
                      set end of elems to (r & "|" & nm)
                    end if
                  end repeat
                on error errMsg
                  return "ERROR:" & errMsg
                end try
                set AppleScript's text item delimiters to linefeed
                return elems as text
              end tell
            end tell
            "#,
            name = name.replace('"', "")
        );
        let raw = match Self::run_osascript(&script) {
            Ok(s) => s,
            Err(e) => {
                // Fallback: process metadata only
                return Ok(AppSnapshot {
                    target: AppTarget {
                        id: id.to_string(),
                        title: name.clone(),
                        bundle_or_exe: name.clone(),
                        pid: None,
                    },
                    summary: format!(
                        "process=\"{name}\" snapshot_limited reason={}",
                        e.message()
                    ),
                    elements: vec![],
                    truncated: true,
                });
            }
        };
        if raw.starts_with("ERROR:") {
            return Ok(AppSnapshot {
                target: AppTarget {
                    id: id.to_string(),
                    title: name.clone(),
                    bundle_or_exe: name.clone(),
                    pid: None,
                },
                summary: format!("process=\"{name}\" ax_error={}", raw.trim_start_matches("ERROR:")),
                elements: vec![],
                truncated: true,
            });
        }
        let mut elements = Vec::new();
        for (i, line) in raw.lines().enumerate() {
            let mut sp = line.splitn(2, '|');
            let role = sp.next().unwrap_or("").trim();
            let nm = sp.next().unwrap_or("").trim();
            if nm.is_empty() {
                continue;
            }
            elements.push(AppElement {
                r#ref: format!("e{}", i + 1),
                role: role.to_string(),
                name: nm.to_string(),
                value: None,
            });
        }
        let mut truncated = false;
        let budget = if budget == 0 { 4000 } else { budget };
        // rough truncate by element count
        let max_elems = (budget as usize / 20).max(5);
        if elements.len() > max_elems {
            elements.truncate(max_elems);
            truncated = true;
        }
        let summary = format!(
            "process=\"{name}\" elements={} truncated={truncated}",
            elements.len()
        );
        Ok(AppSnapshot {
            target: AppTarget {
                id: id.to_string(),
                title: name.clone(),
                bundle_or_exe: name,
                pid: None,
            },
            summary,
            elements,
            truncated,
        })
    }

    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value> {
        let enabled = std::env::var("VCU_ALLOW_APP_INVOKE")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if !enabled {
            return Err(VcuError::coded(
                ErrorCode::OsCursorDenied,
                "app invoke denied; set VCU_ALLOW_APP_INVOKE=1 and use an allowlisted target for AX press",
            ));
        }
        let name = id
            .strip_prefix("proc:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string());
        if !self.allowed(&name) {
            return Err(VcuError::coded(
                ErrorCode::FocusPolicyViolation,
                format!("process '{name}' not in app allowlist"),
            ));
        }
        let script = format!(
            "tell application \"System Events\"\n tell process \"{pname}\"\n try\n set uiElems to entire contents\n set n to 0\n repeat with e in uiElems\n set nm to \"\"\n try\n set nm to (name of e as text)\n end try\n if nm is not \"\" then\n set n to n + 1\n if (\"e\" & n) is \"{eref}\" then\n try\n perform action \"AXPress\" of e\n return \"ok\"\n on error\n click e\n return \"ok-click\"\n end try\n end if\n end if\n end repeat\n return \"not-found\"\n on error errMsg\n return \"error:\" & errMsg\n end try\n end tell\n end tell\n",
            pname=name.replace('"', ""),
            eref=element_ref.replace('"', ""),
        );
        let out = Self::run_osascript(&script)?;
        if out.starts_with("error:") || out == "not-found" {
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "ax invoke failed", out));
        }
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "ref": element_ref,
            "result": out,
            "input_path": "ax_press",
            "os_cursor_used": false
        }))
    }
}
