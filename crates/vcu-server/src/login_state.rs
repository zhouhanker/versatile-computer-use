//! User-profile vs empty Agent-profile browsers.
//! Login-state computer use attaches to the *user* window (desktop Scene).
//! Never click Chromium "Allow debugging".

use serde::Serialize;
use serde_json::{json, Value};
use vcu_core::{ErrorCode, TabInfo, VcuError, VcuResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserProfile {
    User,
    Agent,
    Helper,
    App,
}

impl BrowserProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Agent => "agent",
            Self::Helper => "helper",
            Self::App => "app",
        }
    }
}

/// Classify a process from its name + `ps` command line.
pub fn classify_browser_command(process_name: &str, command: &str) -> BrowserProfile {
    let name = process_name.to_ascii_lowercase();
    let cmd = command.to_ascii_lowercase();
    let is_browser = name.contains("edge")
        || name.contains("chrome")
        || name.contains("chromium")
        || cmd.contains("microsoft edge")
        || cmd.contains("google chrome")
        || cmd.contains("chromium");
    if !is_browser {
        return BrowserProfile::App;
    }
    if cmd.contains("--type=")
        || cmd.contains("crashpad")
        || cmd.contains("helper (")
        || cmd.contains("helper.app")
    {
        return BrowserProfile::Helper;
    }
    if cmd.contains("edge-agent-profile")
        || cmd.contains("chrome-agent-profile")
        || (cmd.contains("--user-data-dir=") && command_uses_vcu_dir(&cmd))
    {
        return BrowserProfile::Agent;
    }
    BrowserProfile::User
}

pub fn process_command(pid: i32) -> Option<String> {
    let out = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "command="])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

pub fn profile_for_pid(process_name: &str, pid: Option<i32>) -> Option<String> {
    let pid = pid?;
    let cmd = process_command(pid)?;
    match classify_browser_command(process_name, &cmd) {
        BrowserProfile::App | BrowserProfile::Helper => None,
        other => Some(other.as_str().to_string()),
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct BrowserProc {
    pub pid: i32,
    pub name: String,
    pub profile: String,
    pub command_excerpt: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoginBrowserReport {
    pub preferred_path: &'static str,
    pub user_browsers: Vec<BrowserProc>,
    pub agent_browsers: Vec<BrowserProc>,
    pub cdp_listening: bool,
    pub cdp_note: String,
    /// agent = empty profile extension; user = DOM lens on login-state; none = unpaired
    pub extension_profile: &'static str,
    pub automation_infobar: bool,
    pub allow_dialog_visible: bool,
    pub cdp_handshake: &'static str,
    pub host_vision: &'static str,
    pub never: Vec<&'static str>,
    pub lens_copied: bool,
    pub lens_dir: String,
    pub next_action: String,
    pub never_click_allow: bool,
    pub never_os_cursor: bool,
    pub never_wechat: bool,
}

fn excerpt(cmd: &str) -> String {
    let t = cmd.trim();
    if t.len() <= 180 {
        t.to_string()
    } else {
        format!("{}…", &t[..180])
    }
}

fn cdp_listening() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:9222".parse().unwrap(),
        std::time::Duration::from_millis(150),
    )
    .is_ok()
}

pub fn classify_debug_ui(blob: &str) -> (bool, bool) {
    let b = blob.to_ascii_lowercase();
    let automation = b.contains("自动测试")
        || b.contains("controlled by automated")
        || b.contains("being controlled by automated test");
    let allow = b.contains("allow debugging")
        || b.contains("允许调试")
        || (b.contains("allow") && b.contains("debugging"));
    (automation, allow)
}

pub fn extension_error_is_stale(message: &str, detail: Option<&str>) -> bool {
    let blob = format!("{} {}", message, detail.unwrap_or("")).to_ascii_lowercase();
    blob.contains("unknown method ping") || blob.contains("unknown method")
}

pub fn login_next_action(
    no_user_browser: bool,
    lens_copied: bool,
    extension_profile: &str,
    allow_dialog: bool,
    lens_dir: &str,
    sw_stale: bool,
) -> String {
    let _ = allow_dialog; // CDP abandoned: never ask the user to click Allow.
    if sw_stale {
        return format!(
            "USER Edge extension SW is stale (ping unknown method). Open edge://extensions and click Reload on VCU Browser Bridge (unpacked {lens_dir}). Then `vcu browser ping` must pong. Never click Allow."
        );
    }
    if no_user_browser {
        return "Open the USER Chrome/Edge (logged-in profile), not Agent Edge.".into();
    }
    if !lens_copied {
        return "Run `vcu browser install-lens`, then Load unpacked in USER Edge. CDP is abandoned; never click Allow.".into();
    }
    if extension_profile != "user" {
        return format!(
            "In USER Edge open edge://extensions, Developer mode, Load unpacked → {lens_dir}. Then `vcu browser login-state` should show extension_profile=user. Observe without waiting: `vcu browser observe`. CDP is abandoned; never click Allow."
        );
    }
    "Login-state DOM lens is on the USER browser. `vcu browser ping` must pong; then `vcu browser observe` and view the PNG. For 60s, click/screenshot/open without tab_id bind last observe. `vcu browser extract` source must be extension_dom. Do not use Agent Edge. CDP is abandoned; never click Allow.".into()
}

fn login_home_dir() -> std::path::PathBuf {
    // Windows USERPROFILE is the real profile. HOME may be unset or a Git-bash path.
    let keys: &[&str] = if cfg!(windows) {
        &["USERPROFILE", "HOME"]
    } else {
        &["HOME", "USERPROFILE"]
    };
    for key in keys {
        if let Ok(home) = std::env::var(key) {
            if !home.is_empty() {
                return std::path::PathBuf::from(home);
            }
        }
    }
    std::path::PathBuf::new()
}

fn command_uses_vcu_dir(cmd: &str) -> bool {
    cmd.contains("/.vcu/")
        || cmd.contains("\\.vcu\\")
        || cmd.contains("/.vcu\\")
        || cmd.contains("\\.vcu/")
}

fn lens_status() -> (bool, String) {
    let dir = login_home_dir().join(".vcu").join("lens-extension");
    let copied = dir.join("manifest.json").exists();
    (copied, dir.display().to_string())
}

pub fn classify_extension_profile(user: &[BrowserProc], agent: &[BrowserProc]) -> &'static str {
    let looks_vcu_ext = |c: &str| {
        let c = c.to_ascii_lowercase();
        c.contains("load-extension")
            || c.contains("disable-extensions-except")
            || c.contains("edge-agent-profile")
            || c.contains("share/vcu")
    };
    let user_ext = user.iter().any(|b| {
        let c = b.command_excerpt.to_ascii_lowercase();
        looks_vcu_ext(&c) && !c.contains("edge-agent-profile") && !c.contains("/.vcu/")
    });
    if user_ext {
        "user"
    } else if agent.iter().any(|b| looks_vcu_ext(&b.command_excerpt)) {
        "agent"
    } else {
        "none"
    }
}


fn browser_display_name(process_name: &str, command: &str) -> String {
    let blob = format!("{process_name} {command}").to_ascii_lowercase();
    if blob.contains("edge") {
        "Microsoft Edge".into()
    } else {
        "Chrome".into()
    }
}

fn ingest_scanned_browser(
    pid: i32,
    process_name: &str,
    command: &str,
    user_browsers: &mut Vec<BrowserProc>,
    agent_browsers: &mut Vec<BrowserProc>,
) {
    let kind = classify_browser_command(process_name, command);
    match kind {
        BrowserProfile::User => user_browsers.push(BrowserProc {
            pid,
            name: browser_display_name(process_name, command),
            profile: kind.as_str().to_string(),
            command_excerpt: excerpt(command),
        }),
        BrowserProfile::Agent => agent_browsers.push(BrowserProc {
            pid,
            name: "Agent Edge".into(),
            profile: kind.as_str().to_string(),
            command_excerpt: excerpt(command),
        }),
        _ => {}
    }
}

fn parse_windows_browser_line(line: &str) -> Option<(i32, String, String)> {
    let mut parts = line.splitn(3, '\t');
    let pid = parts.next()?.trim().parse::<i32>().ok()?;
    let name = parts.next()?.trim().to_string();
    let cmd = parts.next()?.trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some((pid, name, cmd))
}

fn scan_login_browsers() -> (Vec<BrowserProc>, Vec<BrowserProc>) {
    let mut user_browsers = Vec::new();
    let mut agent_browsers = Vec::new();
    for (pid, name, cmd) in scan_browser_process_rows() {
        ingest_scanned_browser(pid, &name, &cmd, &mut user_browsers, &mut agent_browsers);
    }
    (user_browsers, agent_browsers)
}

fn scan_browser_process_rows() -> Vec<(i32, String, String)> {
    #[cfg(windows)]
    {
        return scan_browser_process_rows_windows();
    }
    #[cfg(not(windows))]
    {
        scan_browser_process_rows_ps()
    }
}

#[cfg(not(windows))]
fn scan_browser_process_rows_ps() -> Vec<(i32, String, String)> {
    let mut rows = Vec::new();
    let Ok(out) = std::process::Command::new("ps")
        .args(["-ax", "-o", "pid=,command="])
        .output()
    else {
        return rows;
    };
    if !out.status.success() {
        return rows;
    }
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((pid_s, cmd)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        let Ok(pid) = pid_s.trim().parse::<i32>() else {
            continue;
        };
        rows.push((pid, String::new(), cmd.trim().to_string()));
    }
    rows
}

#[cfg(windows)]
fn scan_browser_process_rows_windows() -> Vec<(i32, String, String)> {
    let script = r#"
$OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$tab = [char]9
Get-CimInstance Win32_Process | Where-Object {
  $_.Name -match '^(msedge|chrome|chromium)(\.exe)?$'
} | ForEach-Object {
  $cmd = if ($null -eq $_.CommandLine) { '' } else { [string]$_.CommandLine }
  $cmd = $cmd -replace "[\r\n\t]", ' '
  '{0}{1}{2}{1}{3}' -f $_.ProcessId, $tab, $_.Name, $cmd
}
"#;
    let path = std::env::temp_dir().join(format!(
        "vcu-login-ps-{}-{}.ps1",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let mut bytes = vec![0xEF, 0xBB, 0xBF];
    bytes.extend_from_slice(script.as_bytes());
    if std::fs::write(&path, &bytes).is_err() {
        return Vec::new();
    }
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let ps = std::path::PathBuf::from(root).join(r"System32\WindowsPowerShell\v1.0\powershell.exe");
    let output = std::process::Command::new(ps)
        .args([
            "-NoProfile",
            "-STA",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            path.to_string_lossy().as_ref(),
        ])
        .output();
    let _ = std::fs::remove_file(&path);
    let Ok(output) = output else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(parse_windows_browser_line)
        .collect()
}

/// Scan running Chrome/Edge main processes. Helpers are ignored.
pub fn inspect_login_browsers() -> LoginBrowserReport {
    let (mut user_browsers, agent_browsers) = scan_login_browsers();
    let frontmost = frontmost_user_browser_name();
    user_browsers = order_user_browsers_frontmost_first(user_browsers, frontmost.as_deref());
    let extension_profile = classify_extension_profile(&user_browsers, &agent_browsers);
    let listening = cdp_listening();
    let automation_infobar = false;
    let allow_dialog_visible = false;
    let cdp_handshake = if !listening {
        "down"
    } else {
        "listening_may_block"
    };
    let cdp_note = if listening {
        "CDP is abandoned for login-state even if 127.0.0.1:9222 is listening. Do not click Allow. Use extension extract / observe on the USER browser.".into()
    } else {
        "No CDP on 9222. Login-state path is USER Edge extension extract / observe. CDP is abandoned; never click Allow.".into()
    };
    let lens = lens_status();
    let next_action = login_next_action(
        user_browsers.is_empty(),
        lens.0,
        extension_profile,
        allow_dialog_visible,
        &lens.1,
        false,
    );
    LoginBrowserReport {
        preferred_path: "desktop_user_window",
        user_browsers,
        agent_browsers,
        cdp_listening: listening,
        cdp_note,
        extension_profile,
        automation_infobar,
        allow_dialog_visible,
        cdp_handshake,
        host_vision: "Host multimodal models (e.g. Grok) can use Scene screenshots; vcu init model is optional.",
        never: vec![
            "click Edge Allow debugging",
            "WeChat automation",
            "OS cursor warp",
            "empty agent profile as login-state",
        ],
        lens_copied: lens.0,
        lens_dir: lens.1,
        next_action,
        never_click_allow: true,
        never_os_cursor: true,
        never_wechat: true,
    }
}

/// Prefer an explicit app id, then a user-profile browser, then any allowed non-agent tab.
pub fn pick_login_tab(tabs: &[TabInfo], browser: &str, app_id: Option<&str>) -> Option<String> {
    if let Some(id) = app_id {
        if tabs.iter().any(|t| t.tab_id == id) {
            return Some(id.to_string());
        }
    }
    let browser = browser.to_ascii_lowercase();
    let want_edge = browser == "edge" || browser == "auto";
    let want_chrome = browser == "chrome" || browser == "auto";
    let is_wanted = |t: &TabInfo| {
        let blob = format!("{} {}", t.title, t.url).to_ascii_lowercase();
        let looks_edge = blob.contains("edge");
        let looks_chrome = blob.contains("chrome");
        match t.browser_profile.as_deref() {
            Some("user") if want_edge && looks_edge => true,
            Some("user") if want_chrome && looks_chrome => true,
            Some("user") if browser == "auto" => true,
            _ => false,
        }
    };
    if let Some(t) = tabs.iter().find(|t| t.agent_owned && is_wanted(t)) {
        return Some(t.tab_id.clone());
    }
    tabs.iter()
        .find(|t| t.agent_owned && t.browser_profile.as_deref() != Some("agent"))
        .or_else(|| tabs.iter().find(|t| t.agent_owned))
        .map(|t| t.tab_id.clone())
}


/// Feishu/Lark composer Send lives in the Electron webview, not AX.
/// A scene whose AX names contain 发送/Send is lying or is not the messenger UI.
/// Desktop session must not fall back to another window when the requested id is missing.
fn win_ids_same_window(listed: &str, requested: &str) -> bool {
    if listed == requested || listed.eq_ignore_ascii_case(requested) {
        return true;
    }
    let Some(listed_pid) = listed.rsplit(':').next() else {
        return false;
    };
    let Some(requested_pid) = requested.rsplit(':').next() else {
        return false;
    };
    listed.to_ascii_lowercase().starts_with("win:")
        && requested.to_ascii_lowercase().starts_with("win:")
        && listed_pid == requested_pid
        && !listed_pid.is_empty()
        && listed_pid.chars().all(|c| c.is_ascii_digit())
}


fn packaged_family(requested: &str) -> Option<&'static str> {
    let req = requested.to_ascii_lowercase();
    if req.starts_with("win:notepad:") {
        Some("notepad")
    } else {
        None
    }
}

fn unique_packaged_window<'a>(tabs: &'a [TabInfo], requested: &str) -> Option<&'a str> {
    let family = packaged_family(requested)?;
    let matches: Vec<_> = tabs
        .iter()
        .filter(|t| {
            let id = t.tab_id.to_ascii_lowercase();
            let title = t.title.to_ascii_lowercase();
            id.contains(family) || title.contains(family) || title.contains("记事本")
        })
        .collect();
    if matches.len() == 1 {
        Some(matches[0].tab_id.as_str())
    } else {
        None
    }
}

pub fn require_desktop_window(tabs: &[TabInfo], app_id: Option<&str>) -> VcuResult<Option<String>> {
    let Some(requested) = app_id.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(None);
    };
    if let Some(tab) = tabs.iter().find(|t| win_ids_same_window(&t.tab_id, requested)) {
        return Ok(Some(tab.tab_id.clone()));
    }
    if let Some(resolved) = unique_packaged_window(tabs, requested) {
        return Ok(Some(resolved.to_string()));
    }
    Err(VcuError::coded(
        ErrorCode::TabNotFound,
        format!(
            "desktop window '{requested}' has no visible window; refusing to bind another app"
        ),
    ))
}

pub fn ax_exposes_send_control<I, S>(names: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    names.into_iter().any(|n| {
        let n = n.as_ref().trim();
        n.contains("发送") || n == "Send" || n == "Send message"
    })
}

/// Login-state / Codex-like keys: never HID, never blind Return (would send IM).
pub fn plan_login_key(key: &str, confirm_send: bool, has_send_ref: bool) -> VcuResult<&'static str> {
    match key.trim().to_ascii_lowercase().as_str() {
        "esc" | "escape" => Err(VcuError::coded(
            ErrorCode::FocusPolicyViolation,
            "Escape is not injected; Stage HUD Esc abort is the cancel path. dry_run reports this policy.",
        )),
        "return" | "enter" => {
            if !confirm_send {
                Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    "Return/Enter is gated; confirm_send=true and a Send control ref required after the user asked to send",
                ))
            } else if !has_send_ref {
                Err(VcuError::coded(
                    ErrorCode::FocusPolicyViolation,
                    "blind Return is forbidden; pass --ref of the Send button",
                ))
            } else {
                Ok("ax_press_send")
            }
        }
        "tab" => Err(VcuError::coded(
            ErrorCode::NotImplemented,
            "Tab is not injected; snapshot/click the next control",
        )),
        other => Err(VcuError::coded(
            ErrorCode::NotImplemented,
            format!("login-state key not implemented: {other}"),
        )),
    }
}

/// Map a desktop app id (`proc:Chrome:123`) to an extension browser kind.
pub fn browser_kind_from_app_id(id: &str) -> Option<&'static str> {
    let s = id.to_ascii_lowercase();
    if s.contains("wechat") || s.contains("微信") {
        return None;
    }
    if s.contains("edge") {
        return Some("edge");
    }
    if s.contains("chrome") {
        return Some("chrome");
    }
    None
}

pub fn browser_name_is_frontmost(name: &str, frontmost: &str) -> bool {
    let n = name.to_ascii_lowercase();
    let f = frontmost.to_ascii_lowercase();
    if n.contains("edge") {
        return f.contains("edge");
    }
    if n.contains("chrome") {
        return f.contains("chrome") && !f.contains("edge");
    }
    n == f
}

static FRONTMOST_OVERRIDE: std::sync::Mutex<Option<Option<String>>> = std::sync::Mutex::new(None);

/// Test helper. `None` forces "frontmost is not a USER browser". Clear after the test.
pub fn set_frontmost_user_browser_override(name: Option<&str>) {
    *FRONTMOST_OVERRIDE.lock().unwrap() = Some(name.map(|s| s.to_string()));
}

pub fn clear_frontmost_user_browser_override() {
    *FRONTMOST_OVERRIDE.lock().unwrap() = None;
}

/// macOS frontmost app if it is USER Chrome/Edge. Never sets frontmost.
pub fn frontmost_user_browser_name() -> Option<String> {
    if let Ok(guard) = FRONTMOST_OVERRIDE.lock() {
        if let Some(over) = guard.as_ref() {
            return over.clone();
        }
    }
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to get name of first application process whose frontmost is true"#,
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if name.is_empty() {
            return None;
        }
        let l = name.to_ascii_lowercase();
        if l.contains("edge") {
            return Some("Microsoft Edge".into());
        }
        if l.contains("chrome") && !l.contains("edge") {
            return Some("Chrome".into());
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        None
    }
}

pub fn order_user_browsers_frontmost_first(
    mut users: Vec<BrowserProc>,
    frontmost: Option<&str>,
) -> Vec<BrowserProc> {
    let Some(front) = frontmost.map(str::trim).filter(|s| !s.is_empty()) else {
        return users;
    };
    users.sort_by_key(|u| !browser_name_is_frontmost(&u.name, front));
    users
}

fn http_url(v: &Value) -> Option<String> {
    v.as_str()
        .filter(|u| u.starts_with("http://") || u.starts_with("https://"))
        .map(|s| s.to_string())
}

fn tab_is_active(t: &Value) -> bool {
    t.get("active").and_then(Value::as_bool).unwrap_or(false)
        || t.get("selected").and_then(Value::as_bool).unwrap_or(false)
        || t.get("focused").and_then(Value::as_bool).unwrap_or(false)
}

/// Fill empty AX tabs/url from the extension list. Never writes `source=extension_dom`
/// and never mutates AX `elements` (webpage clicks stay on the DOM lens).
pub fn merge_extension_tabs_into_scene(body: &mut Value, app_id: &str, ext: &Value) {
    let Some(kind) = browser_kind_from_app_id(app_id) else {
        return;
    };
    let Some(all) = ext.get("tabs").and_then(Value::as_array) else {
        return;
    };
    let browsers = ext
        .get("browsers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let only_kind = browsers.len() == 1 && browsers[0].as_str() == Some(kind);
    let filtered: Vec<Value> = all
        .iter()
        .filter(|t| match t
            .get("browser")
            .and_then(Value::as_str)
            .map(|s| s.to_ascii_lowercase())
        {
            Some(b) if !b.is_empty() => b == kind,
            _ => only_kind,
        })
        .cloned()
        .collect();
    if filtered.is_empty() {
        return;
    }
    body["scene_source"] = json!("ax_scene");
    let ax_tabs_empty = body
        .get("tabs")
        .and_then(Value::as_array)
        .map(|a| a.is_empty())
        .unwrap_or(true);
    if ax_tabs_empty {
        body["tabs"] = json!(filtered.clone());
        body["tabs_source"] = json!("extension_tabs");
    } else if body.get("tabs_source").is_none() {
        body["tabs_source"] = json!("ax_scene");
    }
    let ax_url_empty = body
        .get("page_url")
        .and_then(Value::as_str)
        .unwrap_or("")
        .is_empty();
    if ax_url_empty {
        let url = filtered
            .iter()
            .find(|t| tab_is_active(t))
            .and_then(|t| http_url(t.get("url").unwrap_or(&Value::Null)))
            .or_else(|| {
                filtered
                    .iter()
                    .find_map(|t| http_url(t.get("url").unwrap_or(&Value::Null)))
            });
        if let Some(url) = url {
            body["page_url"] = json!(url);
            body["page_url_source"] = json!("extension_tabs");
        }
    }
    if let Some(tab) = focused_http_tab(&filtered) {
        let ax_tab_empty = body
            .get("tab_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty();
        if ax_tab_empty {
            if let Some(id) = tab.get("tab_id").and_then(Value::as_str) {
                body["tab_id"] = json!(id);
                body["tab_id_source"] = json!("extension_tabs");
            }
        }
        let ax_title_empty = body
            .get("page_title")
            .and_then(Value::as_str)
            .unwrap_or("")
            .is_empty();
        if ax_title_empty {
            if let Some(title) = tab
                .get("title")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
            {
                body["page_title"] = json!(title);
                body["page_title_source"] = json!("extension_tabs");
            }
        }
    }
}

fn focused_http_tab(tabs: &[Value]) -> Option<&Value> {
    tabs.iter()
        .find(|t| tab_is_active(t) && http_url(t.get("url").unwrap_or(&Value::Null)).is_some())
        .or_else(|| {
            tabs.iter()
                .find(|t| http_url(t.get("url").unwrap_or(&Value::Null)).is_some())
        })
}

pub fn snapshot_has_png(body: &Value) -> bool {
    body.get("screenshot_path")
        .and_then(Value::as_str)
        .map(|s| !s.is_empty())
        .unwrap_or(false)
        || body
            .pointer("/vision_handoff/must_view")
            .and_then(Value::as_array)
            .map(|a| a.iter().any(|p| p.as_str().map(|s| s.ends_with(".png")).unwrap_or(false)))
            .unwrap_or(false)
}


pub const LAST_OBSERVE_TTL_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct LastObserve {
    pub app_id: String,
    pub tab_id: Option<String>,
    pub at: std::time::Instant,
}

impl LastObserve {
    pub fn fresh(&self, now: std::time::Instant) -> bool {
        now.saturating_duration_since(self.at).as_secs() < LAST_OBSERVE_TTL_SECS
    }
}

/// Explicit tab wins. Else a fresh observe tab. Else the caller keeps implicit last-focused.
pub fn bind_tab_id(
    explicit: Option<&str>,
    last: Option<&LastObserve>,
    now: std::time::Instant,
) -> (Option<String>, &'static str) {
    if let Some(id) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return (Some(id.to_string()), "explicit");
    }
    if let Some(last) = last.filter(|l| l.fresh(now)) {
        if let Some(id) = last
            .tab_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            return (Some(id.to_string()), "last_observe");
        }
    }
    (None, "implicit")
}

pub fn bind_app_id(
    explicit: Option<&str>,
    last: Option<&LastObserve>,
    now: std::time::Instant,
) -> Option<String> {
    if let Some(id) = explicit.map(str::trim).filter(|s| !s.is_empty()) {
        return Some(id.to_string());
    }
    last.filter(|l| l.fresh(now) && !l.app_id.is_empty())
        .map(|l| l.app_id.clone())
}


/// Attach extension tabs onto a desktop.scene JSON without replacing AX targets/dom_refs.
pub fn attach_extension_tabs_as_browser_tabs(body: &mut Value, app_id: &str, ext: &Value) {
    let mut tmp = json!({
        "tabs": [],
        "page_url": body.get("page_url").cloned().unwrap_or(Value::Null),
        "page_title": body.get("page_title").cloned().unwrap_or(Value::Null),
        "elements": [],
    });
    merge_extension_tabs_into_scene(&mut tmp, app_id, ext);
    if tmp.get("tabs_source").and_then(Value::as_str) != Some("extension_tabs") {
        return;
    }
    body["browser_tabs"] = tmp.get("tabs").cloned().unwrap_or(json!([]));
    body["tabs_source"] = json!("extension_tabs");
    if let Some(v) = tmp.get("page_url") {
        body["page_url"] = v.clone();
    }
    if let Some(v) = tmp.get("page_url_source") {
        body["page_url_source"] = v.clone();
    }
    if let Some(v) = tmp.get("tab_id") {
        body["tab_id"] = v.clone();
    }
    if let Some(v) = tmp.get("tab_id_source") {
        body["tab_id_source"] = v.clone();
    }
    body["scene_source"] = json!("ax_scene");
}

#[cfg(test)]
mod tests {
    use super::*;
    use vcu_core::TabInfo;

    fn tab(id: &str, title: &str, profile: Option<&str>, owned: bool) -> TabInfo {
        TabInfo {
            tab_id: id.into(),
            window_id: "desktop-stage".into(),
            title: title.into(),
            url: format!("app://{}", title.replace(' ', "_")),
            agent_owned: owned,
            borrowed_by: None,
            login_state: Some(profile == Some("user")),
            browser_profile: profile.map(|s| s.to_string()),
        }
    }

    #[test]
    fn classifies_user_agent_helper() {
        assert_eq!(
            classify_browser_command(
                "Microsoft Edge",
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"
            ),
            BrowserProfile::User
        );
        assert_eq!(
            classify_browser_command(
                "Microsoft Edge",
                "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge --user-data-dir=/Users/me/.vcu/edge-agent-profile --load-extension=/tmp/ext"
            ),
            BrowserProfile::Agent
        );
        assert_eq!(
            classify_browser_command(
                "Microsoft Edge Helper",
                "Microsoft Edge Helper (Renderer) --type=renderer"
            ),
            BrowserProfile::Helper
        );
        assert_eq!(
            classify_browser_command("Feishu", "/Applications/Feishu.app/Contents/MacOS/Feishu"),
            BrowserProfile::App
        );
    }

    #[test]
    fn pick_prefers_user_edge_over_agent_and_textedit() {
        let tabs = vec![
            tab("proc:TextEdit:1", "TextEdit", None, true),
            tab("proc:Microsoft_Edge:11", "Microsoft Edge", Some("agent"), true),
            tab("proc:Microsoft_Edge:10", "Microsoft Edge", Some("user"), true),
            tab("proc:WeChat:2", "WeChat", None, false),
        ];
        assert_eq!(
            pick_login_tab(&tabs, "edge", None).as_deref(),
            Some("proc:Microsoft_Edge:10")
        );
        assert_eq!(
            pick_login_tab(&tabs, "edge", Some("proc:TextEdit:1")).as_deref(),
            Some("proc:TextEdit:1")
        );
        let no_user = vec![
            tab("proc:TextEdit:1", "TextEdit", None, true),
            tab("proc:Microsoft_Edge:11", "Microsoft Edge", Some("agent"), true),
        ];
        assert_eq!(
            pick_login_tab(&no_user, "auto", None).as_deref(),
            Some("proc:TextEdit:1")
        );
    }

    #[test]
    fn extension_on_agent_profile_is_not_login_state() {
        let user = vec![BrowserProc {
            pid: 10,
            name: "Microsoft Edge".into(),
            profile: "user".into(),
            command_excerpt: "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge".into(),
        }];
        let agent = vec![BrowserProc {
            pid: 11,
            name: "Agent Edge".into(),
            profile: "agent".into(),
            command_excerpt: "Microsoft Edge --user-data-dir=/Users/me/.vcu/edge-agent-profile --load-extension=/Users/me/.local/share/vcu/extension".into(),
        }];
        assert_eq!(classify_extension_profile(&user, &agent), "agent");
        assert_eq!(classify_extension_profile(&user, &[]), "none");
        let user_ext = vec![BrowserProc {
            pid: 10,
            name: "Microsoft Edge".into(),
            profile: "user".into(),
            command_excerpt: "Microsoft Edge --load-extension=/Users/me/.local/share/vcu/extension".into(),
        }];
        assert_eq!(classify_extension_profile(&user_ext, &agent), "user");
    }

    #[test]
    fn debug_ui_detects_infobar_and_allow() {
        assert_eq!(
            classify_debug_ui("Microsoft Edge 正由自动测试软件控制。"),
            (true, false)
        );
        assert_eq!(
            classify_debug_ui("Allow debugging from this computer?"),
            (false, true)
        );
        assert_eq!(classify_debug_ui("bilibili"), (false, false));
    }

    #[test]
    fn next_action_asks_to_load_unpacked_when_copied_but_agent() {
        let a = login_next_action(false, true, "agent", false, "/tmp/lens-extension", false);
        assert!(a.contains("Load unpacked"));
        assert!(a.contains("/tmp/lens-extension"));
        assert!(a.contains("observe"));
        assert!(!a.contains("click Allow debugging"));
        assert!(!a.contains("User: click Allow"));
        let b = login_next_action(false, true, "user", false, "/tmp/lens-extension", false);
        assert!(b.contains("DOM lens") || b.contains("extension_dom"));
        assert!(b.contains("ping"));
        assert!(b.contains("observe"));
        assert!(b.contains("last observe"));
        assert!(b.contains("never click Allow"));
        assert!(!b.contains("click Allow debugging"));
        let stale = login_next_action(false, true, "user", false, "/tmp/lens-extension", true);
        assert!(stale.contains("Reload"));
        assert!(stale.contains("/tmp/lens-extension"));
        assert!(!stale.contains("click Allow debugging"));
        assert!(extension_error_is_stale("ping", Some("{\"error\":\"unknown method ping\"}")));
        assert!(!extension_error_is_stale("ok", None));
    }

    #[test]
    fn next_action_never_asks_to_click_allow() {
        for profile in ["user", "agent", "none"] {
            let msg = login_next_action(false, true, profile, true, "/tmp/lens-extension", false);
            assert!(!msg.contains("click Allow debugging"), "{msg}");
            assert!(!msg.contains("User: click Allow"), "{msg}");
            assert!(!msg.contains("Allow ONCE"), "{msg}");
            assert!(msg.contains("never click Allow") || msg.contains("Load unpacked") || msg.contains("install-lens"), "{msg}");
        }
        let no_browser = login_next_action(true, true, "user", true, "/tmp/lens-extension", false);
        assert!(!no_browser.contains("Allow debugging"));
        assert!(!no_browser.contains("User: click Allow"));
    }

    #[test]
    fn feishu_like_ax_names_do_not_expose_send() {
        let live_like = [
            "飞书",
            "搜索（⌘＋K）",
            "消息",
            "MultiWebView - messenger",
            "messenger-chat",
            "ProfileButton",
            "创建",
        ];
        assert!(!ax_exposes_send_control(live_like));
        assert!(ax_exposes_send_control(["发送"]));
        assert!(ax_exposes_send_control(["Send"]));
        assert!(!ax_exposes_send_control(["messenger"]));
    }

    #[test]
    fn plan_login_key_gates_return() {
        assert_eq!(
            plan_login_key("return", false, true).unwrap_err().code(),
            ErrorCode::FocusPolicyViolation
        );
        assert_eq!(
            plan_login_key("enter", true, false).unwrap_err().code(),
            ErrorCode::FocusPolicyViolation
        );
        assert_eq!(
            plan_login_key("escape", false, false).unwrap_err().code(),
            ErrorCode::FocusPolicyViolation
        );
        assert_eq!(plan_login_key("return", true, true).unwrap(), "ax_press_send");
    }

    #[test]
    fn inspect_always_denies_allow_cursor_wechat() {
        let r = inspect_login_browsers();
        assert!(r.never_click_allow);
        assert!(r.never_os_cursor);
        assert!(r.never_wechat);
    }


    #[test]
    fn browser_kind_from_app_id_maps_chrome_and_edge() {
        assert_eq!(browser_kind_from_app_id("proc:Chrome:51370"), Some("chrome"));
        assert_eq!(browser_kind_from_app_id("proc:Google_Chrome:1"), Some("chrome"));
        assert_eq!(browser_kind_from_app_id("proc:Microsoft_Edge:10"), Some("edge"));
        assert_eq!(browser_kind_from_app_id("proc:TextEdit:1"), None);
        assert_eq!(browser_kind_from_app_id("proc:WeChat:2"), None);
    }

    fn proc(name: &str, pid: i32) -> BrowserProc {
        BrowserProc {
            pid,
            name: name.into(),
            profile: "user".into(),
            command_excerpt: name.into(),
        }
    }

    #[test]
    fn orders_edge_first_when_frontmost() {
        let users = vec![proc("Chrome", 1), proc("Microsoft Edge", 2)];
        let ordered = order_user_browsers_frontmost_first(users, Some("Microsoft Edge"));
        assert_eq!(ordered[0].name, "Microsoft Edge");
        assert_eq!(ordered[0].pid, 2);
        assert_eq!(ordered[1].name, "Chrome");
    }

    #[test]
    fn orders_chrome_first_when_frontmost() {
        let users = vec![proc("Microsoft Edge", 2), proc("Chrome", 1)];
        let ordered = order_user_browsers_frontmost_first(users, Some("Google Chrome"));
        assert_eq!(ordered[0].name, "Chrome");
    }

    #[test]
    fn merge_extension_tabs_fills_empty_ax_chrome_scene() {
        let ext = json!({
            "tabs": [
                {"tab_id":"1","url":"https://chrome.example/","title":"c","active":true,"browser":"chrome"},
                {"tab_id":"2","url":"https://edge.example/","title":"e","active":true,"browser":"edge"}
            ],
            "browsers": ["chrome","edge"]
        });
        let mut body = json!({"tabs":[], "page_url": null, "elements":[{"ref":"e_web"}]});
        merge_extension_tabs_into_scene(&mut body, "proc:Chrome:9", &ext);
        assert_eq!(body["tabs"].as_array().unwrap().len(), 1);
        assert_eq!(body["tabs"][0]["tab_id"], "1");
        assert_eq!(body["tabs_source"], "extension_tabs");
        assert_eq!(body["page_url"], "https://chrome.example/");
        assert_eq!(body["page_url_source"], "extension_tabs");
        assert_eq!(body["scene_source"], "ax_scene");
        assert_eq!(body["tab_id"], "1");
        assert_eq!(body["tab_id_source"], "extension_tabs");
        assert_eq!(body["page_title"], "c");
        assert_eq!(body["page_title_source"], "extension_tabs");
        assert_eq!(body["elements"].as_array().unwrap().len(), 1);
        assert!(body.get("source").is_none());
        assert!(snapshot_has_png(&json!({"screenshot_path":"/tmp/a.png"})));
        assert!(!snapshot_has_png(&json!({"ok":true})));
    }

    #[test]
    fn merge_extension_tabs_does_not_overwrite_ax_or_cross_browser() {
        let ext = json!({
            "tabs": [
                {"tab_id":"1","url":"https://chrome.example/","active":true,"browser":"chrome"}
            ],
            "browsers": ["chrome"]
        });
        let mut edge = json!({"tabs":[], "page_url": Value::Null});
        merge_extension_tabs_into_scene(&mut edge, "proc:Microsoft_Edge:10", &ext);
        assert!(edge.get("tabs").and_then(|t| t.as_array()).map(|a| a.is_empty()).unwrap_or(true));
        assert!(edge.get("tabs_source").is_none());

        let mut kept = json!({"tabs":[{"name":"AX tab","selected":true}], "page_url":"https://from-ax.example/"});
        merge_extension_tabs_into_scene(&mut kept, "proc:Chrome:1", &ext);
        assert_eq!(kept["tabs"][0]["name"], "AX tab");
        assert_eq!(kept["tabs_source"], "ax_scene");
        assert_eq!(kept["page_url"], "https://from-ax.example/");
        assert!(kept.get("page_url_source").is_none());
    }

    #[test]
    fn bind_tab_id_prefers_explicit_then_fresh_observe() {
        let now = std::time::Instant::now();
        let last = LastObserve {
            app_id: "proc:Chrome:1".into(),
            tab_id: Some("42".into()),
            at: now,
        };
        assert_eq!(bind_tab_id(Some("7"), Some(&last), now), (Some("7".into()), "explicit"));
        assert_eq!(bind_tab_id(None, Some(&last), now), (Some("42".into()), "last_observe"));
        assert_eq!(bind_tab_id(None, None, now), (None, "implicit"));
        let stale = LastObserve {
            app_id: "proc:Chrome:1".into(),
            tab_id: Some("42".into()),
            at: now - std::time::Duration::from_secs(LAST_OBSERVE_TTL_SECS + 1),
        };
        assert_eq!(bind_tab_id(None, Some(&stale), now), (None, "implicit"));
        assert_eq!(bind_app_id(None, Some(&last), now).as_deref(), Some("proc:Chrome:1"));
        assert!(bind_app_id(None, Some(&stale), now).is_none());
    }

    #[test]
    fn attach_browser_tabs_does_not_replace_ax_targets() {
        let ext = json!({
            "tabs": [{"tab_id":"42","url":"https://edge.example/","title":"e","active":true,"browser":"edge"}],
            "browsers": ["edge"]
        });
        let mut body = json!({
            "kind": "desktop.scene",
            "source": "ax_scene",
            "targets": [{"tab_id":"proc:Microsoft_Edge:10"}],
            "dom_refs": [{"ref":"e_web"}]
        });
        attach_extension_tabs_as_browser_tabs(&mut body, "proc:Microsoft_Edge:10", &ext);
        assert_eq!(body["source"], "ax_scene");
        assert_eq!(body["targets"][0]["tab_id"], "proc:Microsoft_Edge:10");
        assert_eq!(body["dom_refs"][0]["ref"], "e_web");
        assert_eq!(body["browser_tabs"][0]["tab_id"], "42");
        assert_eq!(body["tabs_source"], "extension_tabs");
        assert_eq!(body["page_url"], "https://edge.example/");
        assert_eq!(body["tab_id"], "42");
        assert_ne!(body.get("source").and_then(|v| v.as_str()), Some("extension_dom"));
    }

    #[test]
    fn windows_edge_main_is_user_helper_skipped_agent_backslash() {
        assert_eq!(
            classify_browser_command(
                "msedge.exe",
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"
            ),
            BrowserProfile::User
        );
        assert_eq!(
            classify_browser_command(
                "msedge.exe",
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe --type=renderer"
            ),
            BrowserProfile::Helper
        );
        assert_eq!(
            classify_browser_command(
                "msedge.exe",
                r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe --user-data-dir=C:\Users\me\.vcu\edge-agent-profile"
            ),
            BrowserProfile::Agent
        );
    }

    #[test]
    fn require_desktop_window_refuses_missing_id() {
        let tabs = vec![tab("win:cmd:1", "cmd", None, true)];
        let err = require_desktop_window(&tabs, Some("win:notepad:9")).unwrap_err();
        assert_eq!(err.code(), ErrorCode::TabNotFound);
        assert!(err.message().contains("refusing to bind another app"));
        assert_eq!(
            require_desktop_window(&tabs, Some("win:cmd:1")).unwrap().as_deref(),
            Some("win:cmd:1")
        );
        let noted = vec![tab("win:Notepad:9", "Notepad", None, true)];
        assert_eq!(
            require_desktop_window(&noted, Some("win:notepad:9")).unwrap().as_deref(),
            Some("win:Notepad:9")
        );
        assert!(require_desktop_window(&tabs, None).unwrap().is_none());
        let noted = vec![
            tab("win:Notepad:20", "无标题 - Notepad", None, true),
            tab("win:cmd:3", "cmd", None, true),
        ];
        assert_eq!(
            require_desktop_window(&noted, Some("win:notepad:9")).unwrap().as_deref(),
            Some("win:Notepad:20")
        );
        let two = vec![
            tab("win:Notepad:20", "a", None, true),
            tab("win:Notepad:21", "b", None, true),
        ];
        assert!(require_desktop_window(&two, Some("win:notepad:9")).is_err());
    }

    #[test]
    fn windows_scan_line_keeps_user_edge_drops_helper() {
        let mut user = Vec::new();
        let mut agent = Vec::new();
        ingest_scanned_browser(
            16520,
            "msedge.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
            &mut user,
            &mut agent,
        );
        ingest_scanned_browser(
            1520,
            "msedge.exe",
            r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe --type=gpu-process",
            &mut user,
            &mut agent,
        );
        assert_eq!(user.len(), 1);
        assert_eq!(user[0].pid, 16520);
        assert_eq!(user[0].name, "Microsoft Edge");
        assert_eq!(user[0].profile, "user");
        assert!(agent.is_empty());
        let parsed = parse_windows_browser_line("16520\tmsedge.exe\tC:\\Edge\\msedge.exe").unwrap();
        assert_eq!(parsed.0, 16520);
        assert_eq!(parsed.1, "msedge.exe");
    }

}
