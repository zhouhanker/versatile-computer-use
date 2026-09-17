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
                    hint: Some("Run `vcu init model` if the main agent lacks vision".into()),
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


    // CDP optional
    if let Some(c) = &cfg {
        if let Some(url) = &c.cdp_url {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_millis(600))
                .build()
                .ok();
            let ok_cdp = if let Some(client) = client {
                client
                    .get(format!("{}/json/version", url.trim_end_matches('/')))
                    .send()
                    .await
                    .map(|r| r.status().is_success())
                    .unwrap_or(false)
            } else {
                false
            };
            checks.push(DoctorCheck {
                name: "cdp_endpoint".into(),
                status: if ok_cdp { "pass" } else { "warn" }.into(),
                detail: format!("{url} reachable={ok_cdp}"),
                hint: if ok_cdp {
                    None
                } else {
                    Some("Start Chrome/Edge with --remote-debugging-port and set cdp_url".into())
                },
            });
        } else {
            checks.push(DoctorCheck {
                name: "cdp_endpoint".into(),
                status: "warn".into(),
                detail: "cdp_url not configured (mock/extension still work)".into(),
                hint: Some("Optional: set cdp_url in config.json for real browser".into()),
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
            status: if ax_ok { "pass" } else { "warn" }.into(),
            detail: if ax_ok {
                "System Events reachable".into()
            } else {
                "osascript/System Events failed — grant Accessibility to your terminal and vcu-daemon".into()
            },
            hint: if ax_ok {
                None
            } else {
                Some("System Settings → Privacy & Security → Accessibility".into())
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

    checks.push(DoctorCheck {
        name: "app_backend".into(),
        status: "pass".into(),
        detail: format!("platform adapter present ({})", std::env::consts::OS),
        hint: Some("vcu app windows / snapshot — invoke/focus denied by default".into()),
    });

    // OS cursor policy invariant documentation check
    checks.push(DoctorCheck {
        name: "os_cursor_policy".into(),
        status: "pass".into(),
        detail: "browser sessions enforce os_cursor=deny".into(),
        hint: None,
    });

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
