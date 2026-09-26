use std::fs;

use vcu_core::{DaemonStatus, DoctorCheck, DoctorReport, UserConfig, VcuPaths};

use crate::state::AppState;

pub async fn build_report(paths: &VcuPaths, live: Option<&AppState>) -> DoctorReport {
    let mut checks = Vec::new();
    let mut ok = true;

    // user dir
    if paths.root.exists() {
        checks.push(DoctorCheck {
            name: "user_dir".into(),
            status: "pass".into(),
            detail: format!("{}", paths.root.display()),
            hint: None,
        });
    } else {
        ok = false;
        checks.push(DoctorCheck {
            name: "user_dir".into(),
            status: "fail".into(),
            detail: format!("{} missing", paths.root.display()),
            hint: Some("Run `vcu init`".into()),
        });
    }

    let cfg = paths.load_config().ok();
    match &cfg {
        Some(c) => {
            checks.push(DoctorCheck {
                name: "config".into(),
                status: "pass".into(),
                detail: format!(
                    "port={} vision_policy={:?} models={}",
                    c.daemon_port,
                    c.vision_policy,
                    c.models.len()
                ),
                hint: None,
            });
            if c.models.is_empty() {
                checks.push(DoctorCheck {
                    name: "vision_model".into(),
                    status: "warn".into(),
                    detail: "no vision model configured; DOM-only mode still works".into(),
                    hint: Some("Optional: `vcu init model` only if the host agent has no vision. Grok/GPT-4o-class hosts can read Scene screenshots directly.".into()),
                });
            } else {
                checks.push(DoctorCheck {
                    name: "vision_model".into(),
                    status: "pass".into(),
                    detail: format!("default={:?}", c.default_vision_model),
                    hint: None,
                });
            }
        }
        None => {
            ok = false;
            checks.push(DoctorCheck {
                name: "config".into(),
                status: "fail".into(),
                detail: "config.json unreadable".into(),
                hint: Some("Run `vcu init`".into()),
            });
        }
    }

    let daemon = if let Some(state) = live {
        checks.push(DoctorCheck {
            name: "daemon".into(),
            status: "pass".into(),
            detail: format!("in-process version {}", state.version),
            hint: None,
        });
        let ext_ok = state.extension_bridge.is_connected().await;
        let ext_poll = state.extension_bridge.is_polling().await;
        let mut ext_profile = crate::login_state::inspect_login_browsers().extension_profile;
        if state.extension_bridge.likely_user_profile().await {
            ext_profile = "user";
        }
        let ext_browsers = state.extension_bridge.active_browsers().await;
        let (ext_status, ext_detail, ext_hint) = if ext_poll && ext_profile == "user" {
            (
                "pass",
                format!(
                    "VCU extension polling in the USER browser (login-state DOM lens); browsers={}",
                    if ext_browsers.is_empty() { "unknown".into() } else { ext_browsers.join(",") }
                ),
                None,
            )
        } else if ext_poll && ext_profile == "agent" {
            (
                "warn",
                "extension polling on empty Agent Edge — not login-state".into(),
                Some("Load ~/.local/share/vcu/extension unpacked in your USER Edge (edge://extensions) for DOM lens with cookies. Do not treat Agent Edge as logged-in.".into()),
            )
        } else if ext_poll {
            (
                "warn",
                "extension polling but profile unknown".into(),
                Some("Prefer loading the extension in the USER Edge for login-state.".into()),
            )
        } else if ext_ok {
            (
                "warn",
                "extension hello seen but poll loop idle (MV3 SW asleep)".into(),
                None,
            )
        } else {
            (
                "warn",
                "no extension hello yet — unpacked extension auto-pairs via POST /v1/extension/bootstrap".into(),
                Some("For login-state, load the unpacked extension in USER Edge, not the empty Agent profile.".into()),
            )
        };
        checks.push(DoctorCheck {
            name: "extension_bridge".into(),
            status: ext_status.into(),
            detail: ext_detail,
            hint: ext_hint,
        });
        {
            let chrome_app = std::path::Path::new("/Applications/Google Chrome.app").exists()
                || std::path::Path::new(r"C:\Program Files\Google\Chrome\Application\chrome.exe").exists();
            let edge_app = std::path::Path::new("/Applications/Microsoft Edge.app").exists()
                || std::path::Path::new(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe").exists();
            checks.push(dual_browser_lens_check(chrome_app, edge_app, &ext_browsers));
        }
        let cfg = state.config.read().await;
        DaemonStatus {
            running: true,
            endpoint: Some(VcuPaths::endpoint_url(&cfg)),
            pid: Some(std::process::id()),
            version: Some(state.version.clone()),
        }
    } else {
        match probe_daemon(paths, cfg.as_ref()).await {
            Ok(status) => {
                checks.push(DoctorCheck {
                    name: "daemon".into(),
                    status: if status.running { "pass" } else { "fail" }.into(),
                    detail: status
                        .endpoint
                        .clone()
                        .unwrap_or_else(|| "not running".into()),
                    hint: if status.running {
                        None
                    } else {
                        Some("Run `vcu daemon start`".into())
                    },
                });
                if !status.running {
                    ok = false;
                }
                status
            }
            Err(e) => {
                ok = false;
                checks.push(DoctorCheck {
                    name: "daemon".into(),
                    status: "fail".into(),
                    detail: e,
                    hint: Some("Run `vcu daemon start`".into()),
                });
                DaemonStatus {
                    running: false,
                    endpoint: None,
                    pid: None,
                    version: None,
                }
            }
        }
    };


    // CDP abandoned for login-state. Probe only as optional leftover, never as a required next step.
    checks.push(DoctorCheck {
        name: "cdp_endpoint".into(),
        status: "pass".into(),
        detail: "CDP abandoned for login-state; use USER Edge extension extract/observe".into(),
        hint: Some("Do not click Allow. Do not vcu config set-cdp for cookies/DOM.".into()),
    });
    if let Some(c) = &cfg {
        if let Some(url) = &c.cdp_url {
            checks.push(DoctorCheck {
                name: "cdp_url_legacy".into(),
                status: "warn".into(),
                detail: format!("{url} is configured but CDP is not the login-state path"),
                hint: Some("Ignore leftover cdp_url. Prefer extension extract on the USER browser. Never click Allow.".into()),
            });
        }
    }

    // macOS accessibility probe
    #[cfg(target_os = "macos")]
    {
        let ax = std::process::Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first process"])
            .output();
        let ax_ok = ax.map(|o| o.status.success()).unwrap_or(false);
        checks.push(DoctorCheck {
            name: "macos_accessibility".into(),
            status: if ax_ok { "pass" } else { "fail" }.into(),
            detail: if ax_ok {
                "System Events reachable — desktop surface can build Scene".into()
            } else {
                "Accessibility missing — desktop surface cannot observe real windows".into()
            },
            hint: if ax_ok {
                None
            } else {
                Some("系统设置 → 隐私与安全 → 辅助功能（一次授权给终端和 vcu-daemon；不要去点 Edge Allow debugging）".into())
            },
        });
        let rec = crate::app::macos::screen_capture_enabled();
        checks.push(scene_webview_crop_check(ax_ok, rec));
        checks.push(DoctorCheck {
            name: "macos_screen_recording".into(),
            status: if rec { "pass" } else { "fail" }.into(),
            detail: if rec {
                "Screen Recording already granted — desktop Scene can attach window pixels".into()
            } else {
                "Screen Recording not granted to this process — Scene stays AX-only (no TCC prompt will be shown)".into()
            },
            hint: if rec {
                None
            } else {
                Some("系统设置 → 隐私与安全 → 屏幕录制，勾选 vcu-daemon 和你的终端；授权后执行 `vcu daemon stop && vcu daemon start`".into())
            },
        });
        let chrome = std::path::Path::new("/Applications/Google Chrome.app").exists();
        let edge = std::path::Path::new("/Applications/Microsoft Edge.app").exists();
        checks.push(DoctorCheck {
            name: "browsers_installed".into(),
            status: if chrome || edge { "pass" } else { "warn" }.into(),
            detail: format!("chrome={chrome} edge={edge}"),
            hint: if chrome || edge {
                None
            } else {
                Some("Install Chrome or Edge for CDP/extension backends".into())
            },
        });
    }

    {
        #[cfg(target_os = "macos")]
        let found = crate::stage::resolve_stage_bin();
        #[cfg(not(target_os = "macos"))]
        let found: Option<std::path::PathBuf> = None;
        checks.push(stage_helper_check(found.as_deref(), std::env::consts::OS));
    }

    checks.push(app_backend_check(std::env::consts::OS));
    if let Some(scope) = windows_desktop_scope_check(std::env::consts::OS) {
        checks.push(scope);
    }

    // OS cursor policy invariant documentation check
    checks.push(DoctorCheck {
        name: "os_cursor_policy".into(),
        status: "pass".into(),
        detail: "browser sessions enforce os_cursor=deny".into(),
        hint: None,
    });

    let mut login_report = crate::login_state::inspect_login_browsers();
    if let Some(state) = live {
        if state.extension_bridge.likely_user_profile().await {
            login_report.extension_profile = "user";
        }
        login_report.next_action = crate::login_state::login_next_action(
            login_report.user_browsers.is_empty() && login_report.extension_profile != "user",
            login_report.lens_copied,
            login_report.extension_profile,
            false,
            &login_report.lens_dir,
            false,
        );
    }
    checks.push(login_browser_check(&login_report));

    DoctorReport {
        ok,
        checks,
        user_dir: paths.root.display().to_string(),
        daemon,
    }
}

async fn probe_daemon(paths: &VcuPaths, cfg: Option<&UserConfig>) -> Result<DaemonStatus, String> {
    let endpoint = if let Some(c) = cfg {
        VcuPaths::endpoint_url(c)
    } else if paths.endpoint_path().exists() {
        fs::read_to_string(paths.endpoint_path()).map_err(|e| e.to_string())?
    } else {
        return Ok(DaemonStatus {
            running: false,
            endpoint: None,
            pid: read_pid(paths),
            version: None,
        });
    };
    let endpoint = endpoint.trim().to_string();
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(800))
        .build()
        .map_err(|e| e.to_string())?;
    let url = format!("{}/v1/health", endpoint.trim_end_matches('/'));
    match client.get(url).send().await {
        Ok(resp) if resp.status().is_success() => {
            let v: serde_json::Value = resp.json().await.unwrap_or_default();
            Ok(DaemonStatus {
                running: true,
                endpoint: Some(endpoint),
                pid: read_pid(paths),
                version: v
                    .pointer("/data/version")
                    .and_then(|x| x.as_str())
                    .map(|s| s.to_string()),
            })
        }
        _ => Ok(DaemonStatus {
            running: false,
            endpoint: Some(endpoint),
            pid: read_pid(paths),
            version: None,
        }),
    }
}

fn read_pid(paths: &VcuPaths) -> Option<u32> {
    fs::read_to_string(paths.pid_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
}


fn scene_webview_crop_check(ax_ok: bool, rec: bool) -> DoctorCheck {
    let ready = ax_ok && rec;
    DoctorCheck {
        name: "scene_webview_crop".into(),
        status: if ready { "pass" } else { "warn" }.into(),
        detail: if ready {
            "AX + Screen Recording — screenshot/snapshot can attach webview crop (messenger pane)".into()
        } else {
            "webview crop needs Accessibility and Screen Recording; AX-only Scene still works".into()
        },
        hint: if ready {
            Some("MCP vcu_screenshot/vcu_snapshot pass tab_id; mode=full or app snapshot --pixels. App snapshot does not raise HUD.".into())
        } else {
            Some("系统设置 → 隐私与安全：辅助功能 + 屏幕录制给 vcu-daemon/终端；不要点 Edge Allow debugging".into())
        },
    }
}

fn login_browser_check(report: &crate::login_state::LoginBrowserReport) -> DoctorCheck {
    let user_n = report.user_browsers.len();
    let agent_n = report.agent_browsers.len();
    let (status, detail) = if user_n > 0 {
        let pids: Vec<String> = report
            .user_browsers
            .iter()
            .map(|b| format!("{}:{}", b.name, b.pid))
            .collect();
        (
            "pass",
            format!(
                "user browser login-state via desktop surface ({}); agent_profile={} cdp_9222={} handshake={} infobar={} allow_dialog={}",
                pids.join(","),
                agent_n,
                report.cdp_listening,
                report.cdp_handshake,
                report.automation_infobar,
                report.allow_dialog_visible
            ),
        )
    } else if agent_n > 0 {
        (
            "warn",
            format!(
                "only empty Agent Edge (no user cookies). Start desktop on the user Edge window. cdp_9222={}",
                report.cdp_listening
            ),
        )
    } else {
        (
            "warn",
            "no Chrome/Edge main process. Login-state needs the user browser window.".into(),
        )
    };
    DoctorCheck {
        name: "login_browser".into(),
        status: status.into(),
        detail,
        hint: Some(report.next_action.clone()),
    }
}

fn dual_browser_lens_check(
    chrome_installed: bool,
    edge_installed: bool,
    polling: &[String],
) -> DoctorCheck {
    let chrome_poll = polling.iter().any(|b| b.eq_ignore_ascii_case("chrome"));
    let edge_poll = polling.iter().any(|b| b.eq_ignore_ascii_case("edge"));
    if chrome_installed && edge_installed && !(chrome_poll && edge_poll) {
        let have = if polling.is_empty() {
            "none".to_string()
        } else {
            polling.join(",")
        };
        return DoctorCheck {
            name: "lens_dual_browser".into(),
            status: "warn".into(),
            detail: format!("Chrome and Edge are installed but lens is polling: {have}"),
            hint: Some(
                "Load unpacked ~/.vcu/lens-extension in the missing USER browser, then Reload. Never click Allow.".into(),
            ),
        };
    }
    let detail = if chrome_poll && edge_poll {
        "lens polling chrome+edge".into()
    } else if polling.is_empty() {
        "lens polling none".into()
    } else {
        format!("lens polling {}", polling.join(","))
    };
    DoctorCheck {
        name: "lens_dual_browser".into(),
        status: "pass".into(),
        detail,
        hint: None,
    }
}

fn app_backend_check(os: &str) -> DoctorCheck {
    if os == "windows" {
        return DoctorCheck {
            name: "app_backend".into(),
            status: "pass".into(),
            detail: "Windows adapter present; honest paths wm_settext / bm_click / clipboard_paste / wm_vscroll / guide_hover".into(),
            hint: Some(
                "CI live slices only. Not product Windows CU. Not complete Codex CU. WeChat denied. No SendInput / OS cursor."
                    .into(),
            ),
        };
    }
    DoctorCheck {
        name: "app_backend".into(),
        status: "pass".into(),
        detail: format!("platform adapter present ({os})"),
        hint: Some(
            "desktop surface uses AXPress/AXSetValue; WeChat is denied; OS cursor still denied"
                .into(),
        ),
    }
}

fn windows_desktop_scope_check(os: &str) -> Option<DoctorCheck> {
    if os != "windows" {
        return None;
    }
    Some(DoctorCheck {
        name: "windows_desktop_scope".into(),
        status: "warn".into(),
        detail: "CI slices: Notepad/Explorer/cmd/PowerShell/Calculator/Settings-observe/Guide/Abort. Not a product Windows Computer Use session.".into(),
        hint: Some("Do not claim complete Codex CU. WeChat denied. OS cursor denied.".into()),
    })
}

fn stage_helper_check(found: Option<&std::path::Path>, os: &str) -> DoctorCheck {
    if os == "windows" {
        return DoctorCheck {
            name: "stage_helper".into(),
            status: "pass".into(),
            detail: "WinForms Stage HUD (not vcu-stage); abort tears down; Guide overlay; no OS cursor".into(),
            hint: None,
        };
    }
    if os != "macos" {
        return DoctorCheck {
            name: "stage_helper".into(),
            status: "pass".into(),
            detail: "vcu-stage is macOS-only; Linux omits the helper".into(),
            hint: None,
        };
    }
    match found {
        Some(path) => DoctorCheck {
            name: "stage_helper".into(),
            status: "pass".into(),
            detail: format!("native Stage helper {}", path.display()),
            hint: None,
        },
        None => DoctorCheck {
            name: "stage_helper".into(),
            status: "warn".into(),
            detail: "vcu-stage not on PATH or next to vcu-daemon; desktop HUD falls back to JXA capsule".into(),
            hint: Some(
                "Install the macOS archive (includes vcu-stage) or run `bash scripts/build-stage.sh && cp target/release/vcu-stage ~/.local/bin/`"
                    .into(),
            ),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        app_backend_check, dual_browser_lens_check, login_browser_check, scene_webview_crop_check,
        stage_helper_check, windows_desktop_scope_check,
    };
    use std::path::Path;

    #[test]
    fn dual_browser_lens_warns_when_only_one_polls() {
        let c = dual_browser_lens_check(true, true, &["edge".into()]);
        assert_eq!(c.name, "lens_dual_browser");
        assert_eq!(c.status, "warn");
        assert!(c.detail.contains("edge"), "{}", c.detail);
        assert!(c.hint.unwrap().contains("Never click Allow"));
        let p = dual_browser_lens_check(true, true, &["chrome".into(), "edge".into()]);
        assert_eq!(p.status, "pass");
        assert!(p.detail.contains("chrome+edge"));
        let one = dual_browser_lens_check(false, true, &["edge".into()]);
        assert_eq!(one.status, "pass");
    }

    #[test]
    fn stage_helper_is_optional_off_macos() {
        let c = stage_helper_check(None, "linux");
        assert_eq!(c.name, "stage_helper");
        assert_eq!(c.status, "pass");
        assert!(c.detail.contains("macOS-only"));
        let w = stage_helper_check(None, "windows");
        assert_eq!(w.status, "pass");
        assert!(w.detail.contains("WinForms"), "{}", w.detail);
        assert!(!w.detail.contains("omit the helper"), "{}", w.detail);
    }

    #[test]
    fn windows_app_backend_names_honest_paths_not_ax() {
        let w = app_backend_check("windows");
        assert_eq!(w.name, "app_backend");
        assert!(w.detail.contains("wm_settext"), "{}", w.detail);
        assert!(w.detail.contains("bm_click"), "{}", w.detail);
        assert!(w.detail.contains("clipboard_paste"), "{}", w.detail);
        assert!(!w.detail.contains("AXPress"), "{}", w.detail);
        let hint = w.hint.unwrap();
        assert!(hint.contains("Not product Windows CU"), "{hint}");
        assert!(hint.contains("No SendInput"), "{hint}");
        let m = app_backend_check("macos");
        assert!(m.hint.unwrap().contains("AXPress"));
    }

    #[test]
    fn windows_desktop_scope_warns_not_product() {
        assert!(windows_desktop_scope_check("macos").is_none());
        let c = windows_desktop_scope_check("windows").unwrap();
        assert_eq!(c.name, "windows_desktop_scope");
        assert_eq!(c.status, "warn");
        assert!(c.detail.contains("Not a product Windows"), "{}", c.detail);
        assert!(!c.detail.contains("AXPress"));
    }

    #[test]
    fn stage_helper_warns_when_missing_on_macos() {
        let c = stage_helper_check(None, "macos");
        assert_eq!(c.status, "warn");
        assert!(c.detail.contains("JXA"));
    }

    #[test]
    fn stage_helper_passes_when_found_on_macos() {
        let p = Path::new("/usr/local/bin/vcu-stage");
        let c = stage_helper_check(Some(p), "macos");
        assert_eq!(c.status, "pass");
        assert!(c.detail.contains("vcu-stage"));
    }

    #[test]
    fn accessibility_repair_is_settings_hint_not_tcc_or_allow() {
        let w = scene_webview_crop_check(false, false);
        let hint = w.hint.unwrap();
        assert!(hint.contains("系统设置"), "{hint}");
        assert!(hint.contains("不要点") || hint.to_ascii_lowercase().contains("do not click"), "{hint}");
        assert!(!hint.contains("tccutil"), "{hint}");
        assert!(!hint.contains("x-apple.systempreferences"), "{hint}");
        let ax = vcu_core::ErrorCode::AccessibilityDenied.default_hint();
        assert!(ax.contains("系统设置"), "{ax}");
        assert!(ax.contains("Do not click Edge Allow debugging"), "{ax}");
        assert!(!ax.contains("tccutil"));
    }

    #[test]
    fn scene_webview_crop_pass_when_ax_and_recording() {
        let c = scene_webview_crop_check(true, true);
        assert_eq!(c.name, "scene_webview_crop");
        assert_eq!(c.status, "pass");
        assert!(c.detail.contains("webview crop"));
        let w = scene_webview_crop_check(true, false);
        assert_eq!(w.status, "warn");
        let w = scene_webview_crop_check(false, true);
        assert_eq!(w.status, "warn");
    }

    #[test]
    fn login_browser_pass_when_user_edge() {
        let report = crate::login_state::LoginBrowserReport {
            preferred_path: "desktop_user_window",
            user_browsers: vec![crate::login_state::BrowserProc {
                pid: 10,
                name: "Microsoft Edge".into(),
                profile: "user".into(),
                command_excerpt: "Microsoft Edge".into(),
            }],
            agent_browsers: vec![],
            cdp_listening: true,
            cdp_note: "x".into(),
            extension_profile: "agent",
            automation_infobar: true,
            allow_dialog_visible: false,
            cdp_handshake: "listening_may_block",
            host_vision: "h",
            never: vec![],
            lens_copied: true,
            lens_dir: "/tmp/lens".into(),
            next_action: "x".into(),
            never_click_allow: true,
            never_os_cursor: true,
            never_wechat: true,
            frontmost_app: None,
        };
        let c = login_browser_check(&report);
        assert_eq!(c.name, "login_browser");
        assert_eq!(c.status, "pass");
        assert!(c.detail.contains("user browser"));
    }
}
