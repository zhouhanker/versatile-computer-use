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
        || (cmd.contains("--user-data-dir=") && cmd.contains("/.vcu/"))
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
    "Login-state DOM lens is on the USER browser. `vcu browser ping` must pong; `vcu browser extract` source must be extension_dom. Do not use Agent Edge. CDP is abandoned; never click Allow.".into()
}

fn lens_status() -> (bool, String) {
    let home = std::env::var("HOME").unwrap_or_default();
    let dir = std::path::PathBuf::from(home).join(".vcu/lens-extension");
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

/// Scan running Chrome/Edge main processes. Helpers are ignored.
pub fn inspect_login_browsers() -> LoginBrowserReport {
    let mut user_browsers = Vec::new();
    let mut agent_browsers = Vec::new();
    if let Ok(out) = std::process::Command::new("ps")
        .args(["-ax", "-o", "pid=,command="])
        .output()
    {
        if out.status.success() {
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
                let cmd = cmd.trim();
                let kind = classify_browser_command("", cmd);
                match kind {
                    BrowserProfile::User => user_browsers.push(BrowserProc {
                        pid,
                        name: if cmd.to_ascii_lowercase().contains("edge") {
                            "Microsoft Edge".into()
                        } else {
                            "Chrome".into()
                        },
                        profile: kind.as_str().to_string(),
                        command_excerpt: excerpt(cmd),
                    }),
                    BrowserProfile::Agent => agent_browsers.push(BrowserProc {
                        pid,
                        name: "Agent Edge".into(),
                        profile: kind.as_str().to_string(),
                        command_excerpt: excerpt(cmd),
                    }),
                    _ => {}
                }
            }
        }
    }
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
        assert_eq!(body["elements"].as_array().unwrap().len(), 1);
        assert!(body.get("source").is_none());
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

}
