//! macOS app adapter via osascript / System Events (AX).
//! Desktop Actuator uses AXPress / AXSetValue and never warps the user cursor.
use std::process::Command;

use async_trait::async_trait;
use vcu_core::{ErrorCode, VcuError, VcuResult};

use super::{denied_app_error, is_denied_app, AppBackend, AppCapture, AppElement, AppSnapshot, AppTarget};

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
            "Preview".into(),
            "Code".into(),
            "Cursor".into(),
            "Microsoft Edge".into(),
            "Edge".into(),
            "Google Chrome".into(),
            "Chrome".into(),
            "Feishu".into(),
            "Lark".into(),
            "飞书".into(),
            "System Settings".into(),
            "系统设置".into(),
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
            if err.to_lowercase().contains("not allowed")
                || err.to_lowercase().contains("assistive")
                || err.to_lowercase().contains("1002")
            {
                return Err(VcuError::with_detail(
                    ErrorCode::AccessibilityDenied,
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

    fn run_jxa_timeout(script: &str, timeout_ms: u64) -> VcuResult<String> {
        let child = Command::new("osascript")
            .args(["-l", "JavaScript", "-e", script])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::Internal, "jxa spawn failed", e.to_string())
            })?;
        let pid = child.id();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        });
        let output = child.wait_with_output().map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "jxa wait", e.to_string())
        })?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if err.is_empty() {
                return Err(VcuError::coded(ErrorCode::ActionFailed, "jxa timed out"));
            }
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "jxa failed", err));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn run_osascript_timeout(script: &str, timeout_ms: u64) -> VcuResult<String> {
        let child = Command::new("osascript")
            .arg("-e")
            .arg(script)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::Internal, "osascript spawn failed", e.to_string())
            })?;
        let pid = child.id();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(timeout_ms));
            let _ = Command::new("kill").arg("-9").arg(pid.to_string()).status();
        });
        let output = child.wait_with_output().map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "osascript wait", e.to_string())
        })?;
        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if err.is_empty() {
                return Err(VcuError::coded(ErrorCode::ActionFailed, "ax command timed out"));
            }
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "osascript failed", err));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn allowed(&self, name: &str) -> bool {
        if is_denied_app(name) {
            return false;
        }
        if self.allowlist.is_empty() {
            return true;
        }
        let lower = name.to_lowercase();
        self.allowlist
            .iter()
            .any(|a| lower.contains(&a.to_lowercase()))
    }

    fn process_name_from_id(id: &str) -> String {
        id.strip_prefix("proc:")
            .and_then(|rest| rest.split(':').next())
            .map(|s| s.replace('_', " "))
            .unwrap_or_else(|| id.to_string())
    }

    fn pid_from_id(id: &str) -> Option<i32> {
        id.strip_prefix("proc:")
            .and_then(|rest| rest.rsplit(':').next())
            .and_then(|s| s.parse().ok())
    }

    fn ensure_operable(&self, name: &str) -> VcuResult<()> {
        if is_denied_app(name) {
            return Err(denied_app_error(name));
        }
        if !self.allowed(name) {
            return Err(VcuError::coded(
                ErrorCode::AppDenied,
                format!("process '{name}' not in app allowlist"),
            ));
        }
        Ok(())
    }

    fn as_literal(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

/// True when this process may capture pixels without prompting.
/// `VCU_ALLOW_SCREENCAPTURE=0` forces off. Never requests TCC.
pub fn screen_capture_preflight() -> bool {
    #[cfg(target_os = "macos")]
    unsafe {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGPreflightScreenCaptureAccess() -> bool;
        }
        CGPreflightScreenCaptureAccess()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub fn screen_capture_enabled() -> bool {
    match std::env::var("VCU_ALLOW_SCREENCAPTURE") {
        Ok(v) if v == "0" || v.eq_ignore_ascii_case("false") || v.eq_ignore_ascii_case("off") => {
            return false;
        }
        _ => {}
    }
    screen_capture_preflight()
}

fn parse_frame(s: &str) -> Option<[f64; 4]> {
    let parts: Vec<&str> = s.split(',').map(|x| x.trim()).collect();
    if parts.len() != 4 {
        return None;
    }
    let nums: Vec<f64> = parts.iter().map(|p| p.parse().ok()).collect::<Option<Vec<_>>>()?;
    if nums.iter().all(|n| *n == 0.0) {
        return None;
    }
    Some([nums[0], nums[1], nums[2], nums[3]])
}



struct ParsedAxSnapshot {
    window_frame: Option<[f64; 4]>,
    elements: Vec<AppElement>,
    truncated: bool,
    page_title: Option<String>,
    page_url: Option<String>,
    tabs: Vec<super::AppTab>,
    web_area: bool,
}

fn nonempty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() || t == "missing value" {
        None
    } else {
        Some(t.to_string())
    }
}

fn tab_label(raw: &str) -> String {
    let s = raw.trim();
    for needle in [" - 内存", " - Memory"] {
        if let Some(i) = s.find(needle) {
            return s[..i].trim().to_string();
        }
    }
    s.to_string()
}

fn parse_ax_snapshot(raw: &str) -> ParsedAxSnapshot {
    let mut window_frame = None;
    let mut elements = Vec::new();
    let mut truncated = false;
    let mut page_title = None;
    let mut page_url = None;
    let mut tabs = Vec::new();
    let mut web_area = false;
    let mut idx = 0usize;
    for line in raw.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if line == "TRUNCATED" {
            truncated = true;
            continue;
        }
        if let Some(rest) = line
            .strip_prefix("WINDOW|")
            .or_else(|| line.strip_prefix("WINDOW\t"))
        {
            window_frame = parse_frame(rest);
            continue;
        }
        if let Some(rest) = line.strip_prefix("PAGE|").or_else(|| line.strip_prefix("PAGE\t")) {
            let delim = if line.contains('\t') { '\t' } else { '|' };
            let mut sp = rest.splitn(2, delim);
            page_title = nonempty(sp.next().unwrap_or(""));
            page_url = nonempty(sp.next().unwrap_or(""));
            continue;
        }
        if let Some(rest) = line.strip_prefix("TAB|").or_else(|| line.strip_prefix("TAB\t")) {
            let delim = if line.contains('\t') { '\t' } else { '|' };
            let mut sp = rest.splitn(2, delim);
            let sel = sp.next().unwrap_or("").trim();
            let name = tab_label(sp.next().unwrap_or(""));
            if !name.is_empty() {
                tabs.push(super::AppTab {
                    name,
                    selected: sel == "1" || sel.eq_ignore_ascii_case("true"),
                });
            }
            continue;
        }
        if let Some(_rest) = line.strip_prefix("WEB|").or_else(|| line.strip_prefix("WEB\t")) {
            web_area = true;
            continue;
        }
        let delim = if line.contains('|') { '|' } else { '\t' };
        let mut sp = line.splitn(4, delim);
        let role = sp.next().unwrap_or("").trim();
        let nm = sp.next().unwrap_or("").trim();
        let fr = sp.next().unwrap_or("").trim();
        let val = nonempty(sp.next().unwrap_or(""));
        if role.is_empty() && nm.is_empty() {
            continue;
        }
        idx += 1;
        elements.push(AppElement {
            r#ref: format!("e{idx}"),
            role: role.to_string(),
            name: nm.to_string(),
            value: val,
            frame: parse_frame(fr),
        });
    }
    ParsedAxSnapshot {
        window_frame,
        elements,
        truncated,
        page_title,
        page_url,
        tabs,
        web_area,
    }
}

const AX_BFS_MAX_DEPTH: i32 = 13;
const AX_BFS_MAX_NODES: i32 = 80;
const AX_BFS_BROWSER_NODES: i32 = 160;
const AX_BFS_TIMEOUT_MS: u64 = 4500;
const AX_BFS_BROWSER_TIMEOUT_MS: u64 = 8000;

fn ax_wants_enhanced(process: &str) -> bool {
    let p = process.to_ascii_lowercase();
    p.contains("edge")
        || p.contains("chrome")
        || p.contains("chromium")
        || p.contains("safari")
        || p.contains("feishu")
        || p.contains("lark")
        || process.contains("飞书")
        || p.contains("system settings")
        || process.contains("系统设置")
}

fn ax_prefer_text_kids(process: &str) -> bool {
    process.eq_ignore_ascii_case("TextEdit")
}

fn ax_is_notes(process: &str) -> bool {
    process.eq_ignore_ascii_case("Notes") || process.contains("备忘录")
}

fn uses_cg_windows(process: &str) -> bool {
    let p = process.to_ascii_lowercase();
    p == "finder"
        || p == "terminal"
        || p.contains("ghostty")
        || p.contains("feishu")
        || p.contains("lark")
        || process.contains("飞书")
        || p.contains("system settings")
        || process.contains("系统设置")
}

fn uses_menu_paste(process: &str) -> bool {
    let p = process.to_ascii_lowercase();
    p == "terminal" || p.contains("ghostty")
}

fn ax_bfs_max_depth(process: &str) -> i32 {
    if process.to_ascii_lowercase().contains("finder") || ax_is_notes(process) {
        4
    } else if ax_wants_enhanced(process) {
        14
    } else {
        AX_BFS_MAX_DEPTH
    }
}

fn ax_bfs_max_nodes(process: &str) -> i32 {
    if process.to_ascii_lowercase().contains("finder") || ax_is_notes(process) {
        AX_BFS_MAX_NODES
    } else if ax_wants_enhanced(process) {
        AX_BFS_BROWSER_NODES
    } else {
        AX_BFS_MAX_NODES
    }
}

fn ax_bfs_timeout_ms(process: &str) -> u64 {
    if ax_is_notes(process) {
        4000
    } else if ax_wants_enhanced(process) {
        AX_BFS_BROWSER_TIMEOUT_MS
    } else {
        AX_BFS_TIMEOUT_MS
    }
}

fn ax_enhanced_prelude(process: &str) -> &'static str {
    if ax_wants_enhanced(process) {
        r#"                try
                  set value of attribute "AXEnhancedUserInterface" to true
                end try
"#
    } else {
        ""
    }
}

/// Shared AX attribute read. Name falls back to description/help so snapshot
/// refs match invoke/set_value. Never uses `entire contents`.
fn ax_read_el_snippet() -> &'static str {
    r#"
          set r to ""
          set nm to ""
          set fr to "0,0,0,0"
          try
            set r to (role of el as text)
          end try
          try
            set nm to (name of el as text)
          end try
          if nm is "missing value" then set nm to ""
          if r is "missing value" then set r to ""
          if nm is "" then
            try
              set nm to (description of el as text)
            end try
            if nm is "missing value" then set nm to ""
          end if
          if nm is "" then
            try
              set nm to (help of el as text)
            end try
            if nm is "missing value" then set nm to ""
          end if
          try
            set p to position of el
            set sz to size of el
            set fr to (item 1 of p as text) & "," & (item 2 of p as text) & "," & (item 1 of sz as text) & "," & (item 2 of sz as text)
          end try
"#
}

/// BFS over `UI elements of window 1`. Finder stays shallow (huge folder trees).
/// Lists are named vcuFront/vcuNext so `set end of` does not collide with UI.
/// Browser/Electron: enable AXEnhancedUserInterface and prefer AXWebArea kids.
fn prefer_browser_content_window(script: String, process: &str) -> String {
    let lower = process.to_ascii_lowercase();
    if !lower.contains("edge") && !lower.contains("chrome") { return script; }
    let marker = format!("tell process \"{process}\"");
    let prelude = r#"
                try
                  set value of attribute "AXEnhancedUserInterface" to true
                end try
                repeat 4 times
                  if (count of windows) > 0 then exit repeat
                  delay 0.05
                end repeat
                set vcuBrowserWindow to missing value
                repeat with candidateWindow in windows
                  try
                    set candidateSize to size of candidateWindow
                    if (item 1 of candidateSize >= 200) and (item 2 of candidateSize >= 150) then
                      set vcuBrowserWindow to candidateWindow
                      exit repeat
                    end if
                  end try
                end repeat
                if vcuBrowserWindow is missing value then error "no browser content window"
"#;
    script.replace("window 1", "vcuBrowserWindow").replacen(&marker, &format!("{marker}\n{prelude}"), 1)
}

fn ax_bfs_script(process: &str, on_match: &str, not_found: &str) -> String {
    prefer_browser_content_window(format!(
        r#"
            tell application "System Events"
              tell process "{process}"
{prelude}
                set vcuAcc to ""
                set n to 0
                set vcuW to "0,0,0,0"
                set vcuPageTitle to ""
                set vcuPageUrl to ""
                set vcuTabs to ""
                set vcuWeb to ""
                try
                  set wp to position of window 1
                  set ws to size of window 1
                  set vcuW to (item 1 of wp as text) & "," & (item 2 of wp as text) & "," & (item 1 of ws as text) & "," & (item 2 of ws as text)
                end try
                try
                  set vcuPageTitle to name of window 1 as text
                end try
                try
                  set vcuFront to UI elements of window 1
                  set vcuDepth to 1
                  repeat while ((count of vcuFront) > 0) and (n < {max_nodes}) and (vcuDepth <= {max_depth})
                    set vcuNext to {{}}
                    repeat with vcuIdx from 1 to (count of vcuFront)
                      set el to item vcuIdx of vcuFront
{read_el}
                      set n to n + 1
{on_match}
                      if n >= {max_nodes} then exit repeat
                      if vcuDepth < {max_depth} then
                        set vcuLeaf to false
                        if r contains "Button" then set vcuLeaf to true
                        if r contains "StaticText" then set vcuLeaf to true
                        if r contains "static text" then set vcuLeaf to true
                        if r contains "Splitter" then set vcuLeaf to true
                        if r contains "splitter" then set vcuLeaf to true
{notes_leaf}
                        if r contains "WebArea" then set vcuLeaf to false
                        if vcuLeaf is false then
                          try
                            set vcuKids to UI elements of el
                            set vcuKidMax to 16
                            if r contains "WebArea"{web_pref} then set vcuKidMax to 32
                            if r contains "WebArea"{web_pref} then
                              set vcuPref to {{}}
                              repeat with vcuKidIdx from 1 to (count of vcuKids)
                                if vcuKidIdx > vcuKidMax then exit repeat
                                set end of vcuPref to item vcuKidIdx of vcuKids
                              end repeat
                              set vcuNext to vcuPref & vcuNext
                            else
                              repeat with vcuKidIdx from 1 to (count of vcuKids)
                                if vcuKidIdx > vcuKidMax then exit repeat
                                set end of vcuNext to item vcuKidIdx of vcuKids
                              end repeat
                            end if
                          end try
                        end if
                      end if
                    end repeat
                    set vcuFront to vcuNext
                    set vcuDepth to vcuDepth + 1
                  end repeat
                on error errMsg
                  return "ERROR:" & errMsg
                end try
                {not_found}
              end tell
            end tell
            "#,
        process = process,
        prelude = ax_enhanced_prelude(process),
        max_nodes = ax_bfs_max_nodes(process),
        max_depth = ax_bfs_max_depth(process),
        read_el = ax_read_el_snippet(),
        on_match = on_match,
        not_found = not_found,
        web_pref = if ax_prefer_text_kids(process) {
            r#" or r contains "ScrollArea" or r contains "scroll area" or r contains "TextArea" or r contains "text area""#
        } else {
            ""
        },
        notes_leaf = if ax_is_notes(process) {
            r#"                        if r contains "SplitGroup" then set vcuLeaf to true
                        if r contains "split group" then set vcuLeaf to true
"#
        } else {
            ""
        },
    ), process)
}

fn ax_snapshot_script(process: &str) -> String {
    let max = ax_bfs_max_nodes(process);
    ax_bfs_script(
        process,
        &format!(
            r#"                        if n <= {max} then set vcuAcc to vcuAcc & r & "|" & nm & "|" & fr & linefeed
                        if r is "AXTextField" then
                          if (nm contains "地址") or (nm contains "Address") or (nm contains "Search") then
                            try
                              set vcuTmp to value of el as text
                              if vcuTmp is not "missing value" then
                                if vcuTmp is not "" then set vcuPageUrl to vcuTmp
                              end if
                            end try
                          end if
                        end if
                        if r is "AXRadioButton" then
                          set vcuSel to "0"
                          try
                            set vcuRv to value of el as text
                            if vcuRv is "true" then set vcuSel to "1"
                          end try
                          set vcuTabs to vcuTabs & "TAB" & tab & vcuSel & tab & nm & linefeed
                        end if
                        if r contains "WebArea" then
                          set vcuWeb to "WEB" & tab & nm & tab & fr & linefeed
                        end if"#
        ),
        &format!(
            r#"set vcuMeta to "PAGE" & tab & vcuPageTitle & tab & vcuPageUrl & linefeed & vcuTabs & vcuWeb
                if n >= {max} then
                  return "WINDOW|" & vcuW & linefeed & vcuAcc & "TRUNCATED" & linefeed & vcuMeta
                end if
                return "WINDOW|" & vcuW & linefeed & vcuAcc & vcuMeta"#
        ),
    )
}



/// Depth-1 names for Allow-dialog / automation infobar detection. Not a Scene dump.


pub fn login_set_address_field(app_id: &str, value: &str) -> VcuResult<String> {
    let name = MacosAppBackend::process_name_from_id(app_id);
    if super::is_denied_app(&name) {
        return Err(denied_app_error(&name));
    }
    let raw = MacosAppBackend::run_osascript_timeout(
        &ax_set_address_script(&MacosAppBackend::as_literal(&name), &MacosAppBackend::as_literal(value)),
        1500,
    )?;
    if raw.starts_with("FOUND") || raw.starts_with("ok") {
        Ok(raw)
    } else {
        Err(VcuError::with_detail(ErrorCode::ActionFailed, "address field set failed", raw))
    }
}


/// Chrome-only BFS: no frames, do not enter AXWebArea (page DOM). For address bar.
fn ax_chrome_bfs_script(process: &str, on_match: &str, not_found: &str) -> String {
    prefer_browser_content_window(format!(
        r#"
            tell application "System Events"
              tell process "{process}"
                try
                  set value of attribute "AXEnhancedUserInterface" to true
                end try
                set n to 0
                try
                  set vcuFront to UI elements of window 1
                  set vcuDepth to 1
                  repeat while ((count of vcuFront) > 0) and (n < 80) and (vcuDepth <= 10)
                    set vcuNext to {{}}
                    repeat with vcuIdx from 1 to (count of vcuFront)
                      set el to item vcuIdx of vcuFront
                      set r to ""
                      set nm to ""
                      try
                        set r to (role of el as text)
                      end try
                      try
                        set nm to (name of el as text)
                      end try
                      if nm is "missing value" then set nm to ""
                      if r is "missing value" then set r to ""
                      set n to n + 1
{on_match}
                      if n >= 80 then exit repeat
                      if vcuDepth < 10 then
                        set vcuLeaf to false
                        if r contains "Button" then set vcuLeaf to true
                        if r contains "StaticText" then set vcuLeaf to true
                        if r contains "static text" then set vcuLeaf to true
                        if r contains "WebArea" then set vcuLeaf to true
                        if r contains "Splitter" then set vcuLeaf to true
                        if vcuLeaf is false then
                          try
                            set vcuKids to UI elements of el
                            repeat with vcuKidIdx from 1 to (count of vcuKids)
                              if vcuKidIdx > 20 then exit repeat
                              set end of vcuNext to item vcuKidIdx of vcuKids
                            end repeat
                          end try
                        end if
                      end if
                    end repeat
                    set vcuFront to vcuNext
                    set vcuDepth to vcuDepth + 1
                  end repeat
                on error errMsg
                  return "ERROR:" & errMsg
                end try
                {not_found}
              end tell
            end tell
            "#,
        process = process,
        on_match = on_match,
        not_found = not_found,
    ), process)
}

fn ax_set_address_script(process: &str, val: &str) -> String {
    ax_chrome_bfs_script(
        process,
        &format!(
            r#"                        if (r contains "TextField") or (r contains "ComboBox") or (r contains "text field") or (r contains "combo box") then
                          if (nm contains "地址") or (nm contains "网址") or (nm contains "Address") or (nm contains "Search") or (nm contains "搜索") or (nm contains "URL") or (nm contains "Url") then
                            try
                              set value of el to "{val}"
                              return "ok-address"
                            on error errMsg
                              return "error:" & errMsg
                            end try
                          end if
                        end if"#
        ),
        r#"return "not-found""#,
    )
}

pub fn looks_like_address_field(role: &str, name: &str) -> bool {
    let r = role.to_ascii_lowercase();
    let n = name.to_ascii_lowercase();
    let role_ok = r.contains("textfield")
        || r.contains("text field")
        || r.contains("combobox")
        || r.contains("combo box");
    if !role_ok {
        return false;
    }
    n.contains("地址")
        || n.contains("网址")
        || n.contains("address")
        || n.contains("search")
        || n.contains("搜索")
        || n.contains("url")
        || n.contains("omnibox")
}

pub fn login_find_address_field(app_id: &str) -> Option<(String, String)> {
    let name = MacosAppBackend::process_name_from_id(app_id);
    if super::is_denied_app(&name) {
        return None;
    }
    let raw = MacosAppBackend::run_osascript_timeout(
        &ax_find_address_script(&MacosAppBackend::as_literal(&name)),
        1500,
    )
    .ok()?;
    let raw = raw.trim();
    let rest = raw.strip_prefix("FOUND|")?;
    let mut sp = rest.splitn(2, '|');
    let nm = sp.next().unwrap_or("").trim().to_string();
    let val = sp.next().unwrap_or("").trim().to_string();
    if nm.is_empty() {
        None
    } else {
        Some((nm, val))
    }
}

fn ax_find_address_script(process: &str) -> String {
    ax_chrome_bfs_script(
        process,
        r#"                        if (r contains "TextField") or (r contains "ComboBox") or (r contains "text field") or (r contains "combo box") then
                          if (nm contains "地址") or (nm contains "网址") or (nm contains "Address") or (nm contains "Search") or (nm contains "搜索") or (nm contains "URL") or (nm contains "Url") then
                            set vcuVal to ""
                            try
                              set vcuVal to value of el as text
                            end try
                            if vcuVal is "missing value" then set vcuVal to ""
                            return "FOUND|" & nm & "|" & vcuVal
                          end if
                        end if"#,
        r#"return "not-found""#,
    )
}

pub fn login_debug_ui_blob(app_id: &str) -> String {
    let name = MacosAppBackend::process_name_from_id(app_id);
    if super::is_denied_app(&name) {
        return String::new();
    }
    MacosAppBackend::run_osascript_timeout(
        &ax_debug_ui_blob_script(&MacosAppBackend::as_literal(&name)),
        800,
    )
    .unwrap_or_default()
}

fn ax_debug_ui_blob_script(process: &str) -> String {
    format!(
        r#"
            tell application "System Events"
              tell process "{process}"
                set out to ""
                try
                  set out to name of window 1 as text
                end try
                try
                  set kids to UI elements of window 1
                  repeat with el in kids
                    set r to ""
                    set nm to ""
                    try
                      set r to role of el as text
                    end try
                    try
                      set nm to name of el as text
                    end try
                    if nm is "missing value" then set nm to ""
                    set out to out & linefeed & r & " " & nm
                  end repeat
                end try
                return out
              end tell
            end tell
            "#,
        process = process,
    )
}

fn ax_window_meta_script(process: &str) -> String {
    prefer_browser_content_window(format!(
        r#"
            tell application "System Events"
              tell process "{process}"
                try
                  set value of attribute "AXEnhancedUserInterface" to true
                end try
                set vcuW to "0,0,0,0"
                set vcuPageTitle to ""
                try
                  set wp to position of window 1
                  set ws to size of window 1
                  set vcuW to (item 1 of wp as text) & "," & (item 2 of wp as text) & "," & (item 1 of ws as text) & "," & (item 2 of ws as text)
                end try
                try
                  set vcuPageTitle to name of window 1 as text
                end try
                return "WINDOW|" & vcuW & linefeed & "PAGE" & tab & vcuPageTitle & tab & linefeed
              end tell
            end tell
            "#,
        process = process,
    ), process)
}

#[cfg(test)]
fn ax_login_scene_script(process: &str) -> String {
    let max = 80;
    ax_bfs_script(
        process,
        &format!(
            r#"                        if r contains "WebArea" then
                          set vcuAcc to vcuAcc & r & "|" & nm & "|" & fr & linefeed
                          set vcuWeb to "WEB" & tab & nm & tab & fr & linefeed
                        end if
                        if r is "AXTextField" then
                          if (nm contains "地址") or (nm contains "Address") or (nm contains "Search") then
                            set vcuAcc to vcuAcc & r & "|" & nm & "|" & fr & linefeed
                            try
                              set vcuTmp to value of el as text
                              if vcuTmp is not "missing value" then
                                if vcuTmp is not "" then set vcuPageUrl to vcuTmp
                              end if
                            end try
                          end if
                        end if
                        if r is "AXRadioButton" then
                          set vcuSel to "0"
                          try
                            set vcuRv to value of el as text
                            if vcuRv is "true" then set vcuSel to "1"
                          end try
                          set vcuTabs to vcuTabs & "TAB" & tab & vcuSel & tab & nm & linefeed
                        end if
                        if vcuPageUrl is not "" then
                          if vcuWeb is not "" then
                            if n > 40 then set n to {max}
                          end if
                        end if"#
        ),
        &format!(
            r#"set vcuMeta to "PAGE" & tab & vcuPageTitle & tab & vcuPageUrl & linefeed & vcuTabs & vcuWeb
                if n >= {max} then
                  return "WINDOW|" & vcuW & linefeed & vcuAcc & "TRUNCATED" & linefeed & vcuMeta
                end if
                return "WINDOW|" & vcuW & linefeed & vcuAcc & vcuMeta"#
        ),
    )
}

fn ax_invoke_script(process: &str, eref: &str) -> String {
    ax_bfs_script(
        process,
        &format!(
            r#"                        if ("e" & n) is "{eref}" then
                          try
                            perform action "AXPress" of el
                            if (nm contains "WebView") or (nm contains "messenger") or (r contains "WebArea") then
                              return "ok-webview:ax_press:axpress:0"
                            end if
                            return "ok:ax_press:axpress:0"
                          on error errMsg
                            return "error:ax_press:" & errMsg
                          end try
                        end if"#
        ),
        r#"return "not-found""#,
    )
}

fn ax_terminal_paste_script(process: &str, val: &str) -> String {
    format!(
        r#"
            tell application "System Events"
              tell process "{process}"
                set frontmost to true
              end tell
              set the clipboard to "{val}"
              tell process "{process}"
                try
                  click menu item "粘贴" of menu "编辑" of menu bar 1
                  return "ok-paste-zh"
                on error
                  try
                    click menu item "Paste" of menu "Edit" of menu bar 1
                    return "ok-paste-en"
                  on error errMsg
                    return "error:" & errMsg
                  end try
                end try
              end tell
            end tell
            "#
    )
}

fn ax_textedit_document_script(val: &str) -> String {
    format!(
        r#"
            tell application "System Events"
              tell process "TextEdit"
                try
                  set ta to text area 1 of window 1
                  try
                    set focused of ta to true
                  end try
                  set value of ta to "{val}"
                  return "ok-textedit-textarea"
                on error errMsg
                  return "error:" & errMsg
                end try
              end tell
            end tell
            "#
    )
}

fn ax_set_value_script(process: &str, eref: &str, val: &str) -> String {
    ax_bfs_script(
        process,
        &format!(
            r#"                        if ("e" & n) is "{eref}" then
                          set vcuTarget to el
                          set vcuRole to r
                          if (r does not contain "TextArea") and (r does not contain "text area") and (r does not contain "TextField") and (r does not contain "text field") and (r does not contain "SearchField") and (r does not contain "ComboBox") and (r is not "text") then
                            try
                              set vcuKids to UI elements of el
                              repeat with vcuKidIdx from 1 to (count of vcuKids)
                                set vcuKid to item vcuKidIdx of vcuKids
                                set vcuKr to ""
                                try
                                  set vcuKr to role of vcuKid as text
                                end try
                                if (vcuKr contains "TextArea") or (vcuKr contains "text area") or (vcuKr contains "TextField") or (vcuKr contains "text field") then
                                  set vcuTarget to vcuKid
                                  set vcuRole to vcuKr
                                  exit repeat
                                end if
                              end repeat
                            end try
                          end if
                          set vcuOkRole to false
                          if vcuRole contains "TextArea" then set vcuOkRole to true
                          if vcuRole contains "text area" then set vcuOkRole to true
                          if vcuRole contains "TextField" then set vcuOkRole to true
                          if vcuRole contains "text field" then set vcuOkRole to true
                          if vcuRole contains "SearchField" then set vcuOkRole to true
                          if vcuRole contains "ComboBox" then set vcuOkRole to true
                          if vcuRole is "text" then set vcuOkRole to true
                          if vcuOkRole is false then
                            return "error:not-text:" & vcuRole
                          end if
                          try
                            set focused of vcuTarget to true
                          end try
                          try
                            set value of vcuTarget to "{val}"
                            return "ok:" & vcuRole
                          on error
                            try
                              set value of attribute "AXValue" of vcuTarget to "{val}"
                              return "ok-axvalue:" & vcuRole
                            on error errMsg
                              return "error:" & errMsg
                            end try
                          end try
                        end if"#
        ),
        r#"return "not-found""#,
    )
}


#[derive(Debug, Clone)]
struct CgWindowInfo {
    window_id: u64,
    title: String,
    frame: [f64; 4],
}

fn parse_cg_window_list(raw: &str) -> Vec<CgWindowInfo> {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(raw) else {
        return Vec::new();
    };
    let Some(arr) = v.as_array() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in arr {
        let title = item
            .get("title")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        let window_id = item
            .get("window_id")
            .and_then(|x| x.as_u64())
            .unwrap_or(0);
        let frame = item
            .get("frame")
            .and_then(|x| x.as_array())
            .and_then(|a| {
                if a.len() != 4 {
                    return None;
                }
                Some([
                    a[0].as_f64()?,
                    a[1].as_f64()?,
                    a[2].as_f64()?,
                    a[3].as_f64()?,
                ])
            });
        let Some(frame) = frame else {
            continue;
        };
        if frame[2] < 120.0 || frame[3] < 80.0 {
            continue;
        }
        out.push(CgWindowInfo {
            window_id,
            title,
            frame,
        });
    }
    out
}

impl MacosAppBackend {
    fn finder_posix_path(&self, id: &str, path: &str) -> VcuResult<String> {
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        if !name.eq_ignore_ascii_case("Finder") {
            return Err(VcuError::coded(
                ErrorCode::InvalidInput,
                "reveal/open_path is Finder-only",
            ));
        }
        let p = std::path::Path::new(path);
        if !p.is_absolute() {
            return Err(VcuError::coded(
                ErrorCode::InvalidInput,
                "Finder path must be absolute",
            ));
        }
        let canon = p.canonicalize().map_err(|e| {
            VcuError::with_detail(ErrorCode::InvalidInput, "Finder path not found", e.to_string())
        })?;
        let shown = canon.display().to_string();
        if super::is_denied_app(&shown) {
            return Err(super::denied_app_error(&shown));
        }
        Ok(shown)
    }

    fn run_finder_launch(flag: &str, path: &str) -> VcuResult<serde_json::Value> {
        let helper = crate::stage::resolve_stage_bin().ok_or_else(|| {
            VcuError::coded(
                ErrorCode::ActionFailed,
                "vcu-stage helper is missing for Finder launch services",
            )
        })?;
        let output = Command::new(helper)
            .args([flag, path])
            .output()
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::ActionFailed, "vcu-stage spawn", e.to_string())
            })?;
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if !output.status.success() {
            return Err(VcuError::with_detail(
                ErrorCode::ActionFailed,
                "Finder launch services failed",
                if stderr.is_empty() { stdout } else { stderr },
            ));
        }
        let parsed = serde_json::from_str::<serde_json::Value>(&stdout).unwrap_or(serde_json::json!({
            "ok": true,
            "result": stdout
        }));
        Ok(parsed)
    }

    fn list_cg_windows(pid: i32) -> Vec<CgWindowInfo> {
        let Some(helper) = crate::stage::resolve_stage_bin() else {
            return Vec::new();
        };
        let output = Command::new(helper)
            .args(["--list-windows", &pid.to_string()])
            .output();
        let Ok(output) = output else {
            return Vec::new();
        };
        if !output.status.success() {
            return Vec::new();
        }
        parse_cg_window_list(&String::from_utf8_lossy(&output.stdout))
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
            if name.is_empty() {
                continue;
            }
            let is_allowed = self.allowed(name);
            targets.push(AppTarget {
                id: format!("proc:{}:{}", name.replace(' ', "_"), pid.unwrap_or(idx as i32)),
                title: name.to_string(),
                bundle_or_exe: name.to_string(),
                pid,
                allowed: is_allowed,
                browser_profile: crate::login_state::profile_for_pid(name, pid),
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
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        // budget < 100: window frame+title only (login-state observe).
        // Else full BFS; timeout still falls back to window meta.
        let meta_only = budget > 0 && budget < 100 && ax_wants_enhanced(&name);
        let script = if meta_only {
            ax_window_meta_script(&Self::as_literal(&name))
        } else {
            ax_snapshot_script(&Self::as_literal(&name))
        };
        let timeout = if meta_only { 800 } else { ax_bfs_timeout_ms(&name) };
        let raw = match Self::run_osascript_timeout(&script, timeout) {
            Ok(s) => s,
            Err(e) if e.code() == ErrorCode::AccessibilityDenied => return Err(e),
            Err(e) => {
                let fallback = Self::run_osascript_timeout(
                    &ax_window_meta_script(&Self::as_literal(&name)),
                    800,
                )
                .unwrap_or_default();
                let parsed = parse_ax_snapshot(&fallback);
                return Ok(AppSnapshot {
                    target: AppTarget {
                        id: id.to_string(),
                        title: name.clone(),
                        bundle_or_exe: name.clone(),
                        pid: Self::pid_from_id(id),
                        allowed: true,
                        browser_profile: crate::login_state::profile_for_pid(&name, Self::pid_from_id(id)),
                    },
                    summary: format!(
                        "process=\"{name}\" snapshot_limited reason={}",
                        e.message()
                    ),
                    elements: parsed.elements,
                    truncated: true,
                    window_frame: parsed.window_frame,
                    webview: parsed.web_area,
                    webview_ref: None,
                    page_title: parsed.page_title,
                    page_url: parsed.page_url,
                    tabs: parsed.tabs,
                    ax_enhanced: ax_wants_enhanced(&name),
                });
            }
        };

        if raw.starts_with("ERROR:") {
            return Ok(AppSnapshot {
                target: AppTarget {
                    id: id.to_string(),
                    title: name.clone(),
                    bundle_or_exe: name.clone(),
                    pid: Self::pid_from_id(id),
                    allowed: true,
                    browser_profile: crate::login_state::profile_for_pid(&name, Self::pid_from_id(id)),
                },
                summary: format!("process=\"{name}\" ax_error={}", raw.trim_start_matches("ERROR:")),
                elements: if uses_cg_windows(&name) {
                    Self::pid_from_id(id)
                        .map(Self::list_cg_windows)
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .map(|(i, w)| AppElement {
                            r#ref: format!("w{}", i + 1),
                            role: "CGWindow".into(),
                            name: w.title,
                            value: Some(w.window_id.to_string()),
                            frame: Some(w.frame),
                        })
                        .collect()
                } else {
                    vec![]
                },
                truncated: true,
                window_frame: None,
                webview: false,
                webview_ref: None,
                page_title: None,
                page_url: None,
                tabs: vec![],
                ax_enhanced: ax_wants_enhanced(&name),
            });
        }
        let parsed = parse_ax_snapshot(&raw);
        let mut elements = parsed.elements;
        let mut truncated = parsed.truncated;
        if uses_cg_windows(&name) {
            if let Some(pid) = Self::pid_from_id(id) {
                let cg = Self::list_cg_windows(pid);
                for (i, w) in cg.into_iter().enumerate() {
                    elements.push(AppElement {
                        r#ref: format!("w{}", i + 1),
                        role: "CGWindow".into(),
                        name: w.title,
                        value: Some(w.window_id.to_string()),
                        frame: Some(w.frame),
                    });
                }
            }
        }
        let budget = if budget == 0 { 4000 } else { budget };
        let max_elems = (budget as usize / 20).max(5);
        if elements.len() > max_elems {
            elements.truncate(max_elems);
            truncated = true;
        }
        let (mut webview, webview_ref) = super::webview_hint(&elements);
        if parsed.web_area {
            webview = true;
        }
        let url = parsed.page_url.clone().unwrap_or_default();
        let summary = format!(
            "kind=desktop.scene process=\"{name}\" elements={} truncated={truncated} webview={webview} url={url}",
            elements.len()
        );
        Ok(AppSnapshot {
            target: AppTarget {
                id: id.to_string(),
                title: name.clone(),
                bundle_or_exe: name.clone(),
                pid: Self::pid_from_id(id),
                allowed: true,
                browser_profile: crate::login_state::profile_for_pid(&name, Self::pid_from_id(id)),
            },
            summary,
            elements,
            truncated,
            window_frame: parsed.window_frame,
            webview,
            webview_ref,
            page_title: parsed.page_title,
            page_url: parsed.page_url,
            tabs: parsed.tabs,
            ax_enhanced: ax_wants_enhanced(&name),
        })
    }

    async fn press_at_point(&mut self, id: &str, x: f64, y: f64) -> VcuResult<serde_json::Value> {
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        let pid = Self::pid_from_id(id).ok_or_else(|| {
            VcuError::coded(ErrorCode::InvalidInput, "press_at_point needs proc:Name:pid")
        })?;
        let script = format!(
            r#"
            ObjC.import('ApplicationServices');
            ObjC.import('AppKit');
            function run() {{
              var pid = {pid};
              var x = {x};
              var y = {y};
              function nameOf(p) {{
                try {{
                  var napp = $.NSRunningApplication.runningApplicationWithProcessIdentifier(p);
                  if (!napp) return '';
                  return ObjC.unwrap(napp.localizedName) || '';
                }} catch (e) {{ return ''; }}
              }}
              var sys = $.AXUIElementCreateSystemWide();
              var ptr = Ref();
              var err = $.AXUIElementCopyElementAtPosition(sys, x, y, ptr);
              if (err === 0) {{
                var el = ptr[0];
                var pidRef = Ref('int');
                $.AXUIElementGetPid(el, pidRef);
                var hitPid = pidRef[0];
                var nname = nameOf(hitPid);
                var low = (nname || '').toLowerCase();
                if (low.indexOf('wechat') >= 0 || low.indexOf('weixin') >= 0 || nname.indexOf('微信') >= 0) {{
                  return 'error:denied-app:' + nname;
                }}
                if (hitPid && hitPid !== pid) return 'error:other-pid:' + hitPid + ':' + nname;
                var press = $.AXUIElementPerformAction(el, 'AXPress');
                return 'ok:ax_position_press:pid:' + hitPid + ':axpress:' + press;
              }}
              var app = $.AXUIElementCreateApplication(pid);
              err = $.AXUIElementCopyElementAtPosition(app, x, y, ptr);
              if (err !== 0) return 'error:hit:' + err;
              var el2 = ptr[0];
              var press2 = $.AXUIElementPerformAction(el2, 'AXPress');
              return 'ok:ax_position_press:pid:' + pid + ':axpress:' + press2;
            }}
            "#,
            pid = pid,
            x = x,
            y = y
        );
        let out = Self::run_jxa_timeout(&script, 2500)?;
        if !super::ax_position_press_succeeded(&out) {
            if let Some(code) = super::hit_error_code(&out) {
                if code == ErrorCode::AppDenied {
                    return Err(super::denied_app_error(&out));
                }
            }
            return Err(VcuError::with_detail(
                ErrorCode::ActionFailed,
                "ax press_at_point failed",
                out,
            ));
        }
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "pid": pid,
            "ax_point": {"x": x, "y": y},
            "result": out,
            "input_path": "ax_position_press",
            "os_cursor_used": false,
            "hid_injected": false
        }))
    }

    async fn invoke(&mut self, id: &str, element_ref: &str) -> VcuResult<serde_json::Value> {
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        let script = ax_invoke_script(&Self::as_literal(&name), &Self::as_literal(element_ref));
        let out = Self::run_osascript_timeout(&script, AX_BFS_TIMEOUT_MS)?;
        if !super::ax_ref_press_succeeded(&out) {
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "ax invoke failed", out));
        }
        let webview = out.contains("webview");
        let input_path = if webview { "ax_frame_hit" } else { "ax_press" };
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "ref": element_ref,
            "result": out,
            "input_path": input_path,
            "os_cursor_used": false,
            "hid_injected": false
        }))
    }

    async fn set_value(&mut self, id: &str, element_ref: &str, value: &str) -> VcuResult<serde_json::Value> {
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        if uses_menu_paste(&name) {
            if value.contains('\n') || value.contains('\r') {
                return Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    "Terminal paste refuses newlines; that would execute a command",
                ));
            }
            let script = ax_terminal_paste_script(&Self::as_literal(&name), &Self::as_literal(value));
            let out = Self::run_osascript_timeout(&script, 4000)?;
            if out.to_ascii_lowercase().starts_with("error:") {
                return Err(VcuError::with_detail(ErrorCode::ActionFailed, "terminal paste failed", out));
            }
            return Ok(serde_json::json!({
                "ok": true,
                "process": name,
                "ref": element_ref,
                "result": out,
                "input_path": "ax_menu_paste",
                "os_cursor_used": false,
                "hid_injected": false
            }));
        }
        let script = ax_set_value_script(&Self::as_literal(&name), &Self::as_literal(element_ref), &Self::as_literal(value));
        let mut out = Self::run_osascript_timeout(&script, AX_BFS_TIMEOUT_MS)?;
        if (out.to_ascii_lowercase().starts_with("error:") || out == "not-found")
            && name.eq_ignore_ascii_case("TextEdit")
        {
            // TextEdit document lives in `text area 1`; toolbar/scroll refs are not editable.
            let fallback = ax_textedit_document_script(&Self::as_literal(value));
            if let Ok(alt) = Self::run_osascript_timeout(&fallback, 2500) {
                if !alt.to_ascii_lowercase().starts_with("error:") && alt != "not-found" {
                    out = alt;
                }
            }
        }
        if out.to_ascii_lowercase().starts_with("error:") || out == "not-found" {
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "ax set_value failed", out));
        }
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "ref": element_ref,
            "result": out,
            "input_path": "ax_set_value",
            "os_cursor_used": false
        }))
    }

    async fn reveal_path(&mut self, id: &str, path: &str) -> VcuResult<serde_json::Value> {
        let path = self.finder_posix_path(id, path)?;
        let mut detail = Self::run_finder_launch("--reveal", &path)?;
        detail["os_cursor_used"] = serde_json::json!(false);
        detail["hid_injected"] = serde_json::json!(false);
        detail["path"] = serde_json::json!(path);
        if detail.get("input_path").is_none() {
            detail["input_path"] = serde_json::json!("nsworkspace_reveal");
        }
        Ok(detail)
    }

    async fn open_path(&mut self, id: &str, path: &str) -> VcuResult<serde_json::Value> {
        let path = self.finder_posix_path(id, path)?;
        let mut detail = Self::run_finder_launch("--open-path", &path)?;
        detail["os_cursor_used"] = serde_json::json!(false);
        detail["hid_injected"] = serde_json::json!(false);
        detail["path"] = serde_json::json!(path);
        if detail.get("input_path").is_none() {
            detail["input_path"] = serde_json::json!("nsworkspace_open");
        }
        Ok(detail)
    }

    async fn scroll(
        &mut self,
        id: &str,
        element_ref: Option<&str>,
        dy: i32,
    ) -> VcuResult<serde_json::Value> {
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        let direction = if dy >= 0 { "AXScrollDownByPage" } else { "AXScrollUpByPage" };
        let delta = if dy >= 0 { 0.15 } else { -0.15 };
        // Do not walk `entire contents` — Finder trees hang osascript.
        // Edge/Chrome login-state: prefer AXWebArea / any scroll area, not Finder splitter.
        let script = format!(
            r#"
            tell application "System Events"
              tell process "{pname}"
                try
                  perform action "{dir}" of (first UI element of window 1 whose role is "AXWebArea")
                  return "ok-webarea"
                end try
                try
                  perform action "{dir}" of (first UI element of window 1 whose role is "HTML content")
                  return "ok-html"
                end try
                try
                  perform action "{dir}" of (first scroll area of window 1)
                  return "ok-sa"
                end try
                try
                  perform action "{dir}" of (last scroll area of window 1)
                  return "ok-sa-last"
                end try
                try
                  perform action "{dir}" of window 1
                  return "ok-window"
                end try
                try
                  perform action "{dir}" of scroll area 1 of splitter group 1 of window 1
                  return "ok-split-sa"
                end try
                try
                  set sb to scroll bar 1 of scroll area 1 of splitter group 1 of window 1
                  set cur to value of sb
                  set value of sb to (cur + {delta})
                  return "ok-split-sb"
                end try
                try
                  set sb to scroll bar 1 of scroll area 1 of window 1
                  set cur to value of sb
                  set value of sb to (cur + {delta})
                  return "ok-scrollbar"
                on error errMsg
                  return "error:" & errMsg
                end try
              end tell
            end tell
            "#,
            pname = Self::as_literal(&name),
            dir = direction,
            delta = delta,
        );
        let out = Self::run_osascript_timeout(&script, 2500)?;
        if out.starts_with("error:") {
            return Err(VcuError::with_detail(ErrorCode::ActionFailed, "ax scroll failed", out));
        }
        Ok(serde_json::json!({
            "ok": true,
            "process": name,
            "ref": element_ref,
            "dy": dy,
            "result": out,
            "input_path": "ax_scroll",
            "os_cursor_used": false
        }))
    }

    async fn capture_window(&self, id: &str) -> VcuResult<Option<AppCapture>> {
        // Never call CGRequestScreenCaptureAccess — that pops a TCC dialog.
        if !screen_capture_enabled() {
            return Ok(None);
        }
        let name = Self::process_name_from_id(id);
        self.ensure_operable(&name)?;
        let script = format!(
            r#"
            tell application "System Events"
              tell process "{name}"
                try
                  set p to position of window 1
                  set s to size of window 1
                  return (item 1 of p as text) & "," & (item 2 of p as text) & "," & (item 1 of s as text) & "," & (item 2 of s as text)
                on error errMsg
                  return "ERROR:" & errMsg
                end try
              end tell
            end tell
            "#,
            name = Self::as_literal(&name)
        );
        let raw = Self::run_osascript(&script)?;
        if raw.starts_with("ERROR:") {
            return Ok(None);
        }
        let Some(frame) = parse_frame(&raw) else {
            return Ok(None);
        };
        let pid = Self::pid_from_id(id).ok_or_else(|| VcuError::coded(ErrorCode::InvalidInput, "window capture requires a process ID"))?;
        let helper = crate::stage::resolve_stage_bin().ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "vcu-stage window capture helper is missing; install the current helper"))?;
        let output = Command::new(helper).args(["--window-info", &pid.to_string(), &frame[0].to_string(), &frame[1].to_string(), &frame[2].to_string(), &frame[3].to_string()]).output()
            .map_err(|e| VcuError::with_detail(ErrorCode::ActionFailed, "identify browser capture window", e.to_string()))?;
        let info: serde_json::Value = serde_json::from_slice(&output.stdout).map_err(|_| VcuError::coded(ErrorCode::ActionFailed, "cannot identify browser window for capture; refusing an occluded screen-region screenshot"))?;
        let window_id = info["window_id"].as_u64().filter(|id| *id > 0).ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "invalid native window id"))?;
        // AX window 1 may be an extension popup or native CU status capsule.
        // Screenshot coordinates must describe the actual captured CGWindow.
        let frame = super::json_frame(info.get("frame")).ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "invalid native window frame"))?;
        let out = std::env::temp_dir().join(format!("vcu-window-{}.png", vcu_core::new_id()));
        let status = std::process::Command::new("screencapture")
            .args(["-x", "-o", "-l", &window_id.to_string()])
            .arg(&out).status()
            .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "window screencapture", e.to_string()))?;
        if !status.success() {
            let _ = std::fs::remove_file(&out);
            return Err(VcuError::coded(ErrorCode::ActionFailed, "window screenshot failed"));
        }
        let png = std::fs::read(&out).unwrap_or_default();
        let _ = std::fs::remove_file(&out);
        let (width, height) = super::png_ihdr_size(&png).ok_or_else(|| VcuError::coded(ErrorCode::ActionFailed, "invalid window screenshot PNG"))?;
        Ok(Some(AppCapture { png, width, height, frame }))
    }

    async fn capture_rect(&self, frame: [f64; 4]) -> VcuResult<Option<AppCapture>> {
        if !screen_capture_enabled() {
            return Ok(None);
        }
        if frame[2] < 2.0 || frame[3] < 2.0 {
            return Ok(None);
        }
        let out = std::env::temp_dir().join(format!("vcu-scene-{}.png", std::process::id()));
        let status = std::process::Command::new("screencapture")
            .args([
                "-x",
                "-R",
                &format!("{},{},{},{}", frame[0], frame[1], frame[2], frame[3]),
                out.to_string_lossy().as_ref(),
            ])
            .status()
            .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "screencapture spawn", e.to_string()))?;
        if !status.success() {
            let _ = std::fs::remove_file(&out);
            return Ok(None);
        }
        let png = std::fs::read(&out).unwrap_or_default();
        let _ = std::fs::remove_file(&out);
        if png.is_empty() {
            return Ok(None);
        }
        let (width, height) = super::png_ihdr_size(&png)
            .unwrap_or((frame[2].max(1.0) as u32, frame[3].max(1.0) as u32));
        Ok(Some(AppCapture {
            png,
            width,
            height,
            frame,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_field_matcher_accepts_edge_and_chrome_names() {
        assert!(looks_like_address_field("AXTextField", "地址和搜索栏"));
        assert!(looks_like_address_field("AXComboBox", "Address and search bar"));
        assert!(looks_like_address_field("AXComboBox", "搜索或输入网址"));
        assert!(!looks_like_address_field("AXButton", "Search"));
        assert!(!looks_like_address_field("AXTextField", "Username"));
    }

    #[test]
    fn browser_window_selection_enables_accessibility_before_enumeration() {
        let script = ax_window_meta_script("Microsoft Edge");
        assert!(script.find("AXEnhancedUserInterface").unwrap() < script.find("repeat with candidateWindow").unwrap());
    }


    #[test]
    fn denylist_beats_allowlist_and_edge_feishu_are_allowed() {
        let b = MacosAppBackend::new();
        assert!(!b.allowed("WeChat"));
        assert!(!b.allowed("微信"));
        assert!(b.allowed("Microsoft Edge"));
        assert!(b.allowed("Edge"));
        assert!(b.allowed("Google Chrome"));
        assert!(b.allowed("Chrome"));
        assert!(b.ensure_operable("Chrome").is_ok());
        assert!(b.ensure_operable("Google Chrome").is_ok());
        assert!(b.allowed("Feishu"));
        assert!(b.allowed("Lark"));
        assert!(b.allowed("TextEdit"));
        assert!(b.allowed("System Settings"));
        assert!(b.allowed("系统设置"));
        assert!(b.ensure_operable("WeChat").is_err());
        assert_eq!(b.ensure_operable("WeChat").unwrap_err().code(), ErrorCode::AppDenied);
    }

    #[test]
    fn login_scroll_script_prefers_webarea() {
        let s = include_str!("macos.rs");
        assert!(s.contains("ok-webarea"));
        assert!(s.contains("AXWebArea"));
        assert!(s.contains("first scroll area of window 1"));
        assert!(s.contains("denied-app"));
    }

    #[test]
    fn screen_capture_preflight_does_not_panic() {
        let _ = screen_capture_preflight();
        let _ = screen_capture_enabled();
    }

    #[test]
    fn parse_ax_snapshot_keeps_unnamed_fields_and_send() {
        let raw = "WINDOW|10,20,800,600\nbutton|发送|1,2,40,16\ntext field||5,6,200,24\nTRUNCATED\n";
        let p = parse_ax_snapshot(raw);
        assert_eq!(p.window_frame, Some([10.0, 20.0, 800.0, 600.0]));
        assert_eq!(p.elements.len(), 2);
        assert_eq!(p.elements[0].r#ref, "e1");
        assert_eq!(p.elements[0].name, "发送");
        assert_eq!(p.elements[1].role, "text field");
        assert_eq!(p.elements[1].name, "");
        assert!(p.truncated);
    }

    #[test]
    fn parse_ax_snapshot_page_tabs_webarea() {
        let raw = "WINDOW|0,0,800,600\nAXWebArea|bilibili|10,20,700,500\nTRUNCATED\nPAGE\thome - Microsoft Edge\thttps://www.bilibili.com/video/BV1\nTAB\t1\t大空头 VS 马斯克 - 内存使用率 - 648 MB\nTAB\t0\tGitHub\nWEB\tbilibili\t10,20,700,500\n";
        let p = parse_ax_snapshot(raw);
        assert_eq!(p.elements.len(), 1);
        assert_eq!(p.elements[0].role, "AXWebArea");
        assert_eq!(p.page_title.as_deref(), Some("home - Microsoft Edge"));
        assert_eq!(p.page_url.as_deref(), Some("https://www.bilibili.com/video/BV1"));
        assert_eq!(p.tabs.len(), 2);
        assert!(p.tabs[0].selected);
        assert_eq!(p.tabs[0].name, "大空头 VS 马斯克");
        assert_eq!(p.tabs[1].name, "GitHub");
        assert!(p.web_area);
        assert!(p.truncated);
    }

    #[test]
    fn ax_bfs_scripts_share_walk_and_avoid_entire_contents() {
        let snap = ax_snapshot_script("Feishu");
        let inv = ax_invoke_script("Feishu", "e12");
        let setv = ax_set_value_script("Feishu", "e12", "hello");
        for s in [&snap, &inv, &setv] {
            let low = s.to_lowercase();
            assert!(s.contains("vcuFront"));
            assert!(s.contains("vcuNext"));
            assert!(s.contains("vcuDepth"));
            assert!(s.contains("vcuAcc") || s.contains("not-found") || s.contains("AXPress") || s.contains("AXValue"));
            assert!(!low.contains("entire contents"));
            assert!(s.contains("vcuDepth <= 14") || s.contains("vcuDepth <= 4"));
            assert!(s.contains("n < 160") || s.contains("n < 80"));
            assert!(s.contains("description of el"));
            assert!(s.contains("help of el"));
            assert!(!s.contains("repeat with e3"));
        }
        assert!(snap.contains("WINDOW|"));
        assert!(snap.contains("AXEnhancedUserInterface"));
        assert!(snap.contains("WebArea"));
        assert!(inv.contains("AXPress"));
        assert!(inv.contains("e12"));
        assert!(inv.contains("ok-webview:ax_press:axpress:0"));
        assert!(inv.contains("ok:ax_press:axpress:0"));
        assert!(!inv.contains("click el"));
        assert!(!inv.contains("ok-click"));
        assert!(!inv.to_ascii_lowercase().contains("click at"));
        assert!(!inv.to_ascii_lowercase().contains("mouse location"));
        assert!(!inv.contains("CGWarp"));
        assert!(setv.contains("AXValue"));
        assert!(setv.contains("hello"));
        assert!(setv.contains("not-text"));
        assert!(setv.contains("TextArea"));
        let te_doc = ax_textedit_document_script("hello");
        assert!(te_doc.contains("text area 1"));
        assert!(te_doc.contains("ok-textedit-textarea"));
        assert!(!te_doc.to_ascii_lowercase().contains("mouse"));
        let te = ax_snapshot_script("TextEdit");
        assert!(te.contains("ScrollArea"));
        assert!(te.contains("text area"));
        assert!(ax_prefer_text_kids("TextEdit"));
        assert!(!ax_prefer_text_kids("Finder"));
        assert!(uses_cg_windows("Terminal"));
        assert!(uses_cg_windows("Feishu"));
        assert!(uses_cg_windows("Lark"));
        assert!(uses_cg_windows("System Settings"));
        assert!(uses_menu_paste("Terminal"));
        assert!(!uses_menu_paste("TextEdit"));
        let paste = ax_terminal_paste_script("Terminal", "hello");
        assert!(paste.contains("粘贴") || paste.contains("Paste"));
        assert!(paste.contains("set the clipboard"));
        assert!(!paste.to_ascii_lowercase().contains("keystroke"));
        assert!(!paste.to_ascii_lowercase().contains("key code"));
        let cg = parse_cg_window_list(
            r#"[{"title":"VCU-D-040-probe","window_id":19504,"frame":[2200,154,902,482]},{"title":"tiny","window_id":1,"frame":[0,0,10,10]}]"#,
        );
        assert_eq!(cg.len(), 1);
        assert_eq!(cg[0].title, "VCU-D-040-probe");
        assert_eq!(cg[0].window_id, 19504);
        assert_eq!(cg[0].frame, [2200.0, 154.0, 902.0, 482.0]);
        assert!(parse_cg_window_list("nope").is_empty());
        assert!(!ax_prefer_text_kids("Notes"));
        let notes = ax_snapshot_script("Notes");
        assert!(notes.contains("vcuDepth <= 4"));
        assert!(notes.contains("n < 80"));
        assert!(notes.contains("SplitGroup"));
        assert!(!ax_snapshot_script("TextEdit").contains("SplitGroup"));
        let finder = ax_snapshot_script("Finder");
        assert!(finder.contains("vcuDepth <= 4"));
        assert!(finder.contains("n < 80"));
        assert!(!finder.contains("AXEnhancedUserInterface"));
        let edge = ax_snapshot_script("Microsoft Edge");
        assert!(edge.contains("AXEnhancedUserInterface"));
        assert!(edge.contains("n < 160"));
        assert!(edge.contains("vcuDepth <= 14"));
        let login = ax_login_scene_script("Microsoft Edge");
        assert!(login.contains("AXEnhancedUserInterface"));
        assert!(login.contains("n > 40"));
        let meta = ax_window_meta_script("Microsoft Edge");
        assert!(meta.contains("WINDOW|"));
        assert!(meta.contains("PAGE"));
        assert!(!meta.contains("vcuFront"));
        assert!(ax_wants_enhanced("Microsoft Edge"));
        assert!(!ax_wants_enhanced("Finder"));
        assert!(!ax_wants_enhanced("WeChat"));
        assert!(ax_find_address_script("Microsoft Edge").contains("n < 80"));
        assert!(ax_find_address_script("Microsoft Edge").contains("WebArea"));
        let find = ax_find_address_script("Microsoft Edge");

        assert!(find.contains("FOUND|"));
        assert!(find.contains("地址") || find.contains("Address"));
        assert!(ax_set_address_script("Microsoft Edge", "x").contains("ok-address"));
        let dbg = ax_debug_ui_blob_script("Microsoft Edge");

        assert!(dbg.contains("UI elements of window 1"));
        assert!(!dbg.contains("vcuFront"));

    }
}
