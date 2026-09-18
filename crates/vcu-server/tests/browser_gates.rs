//! Browser-version gates (TC-B-013..028). No Feishu/App product path.
use std::time::Duration;

use serde_json::json;
use vcu_core::{UserConfig, VcuPaths};

async fn boot_async() -> (vcu_server::DaemonHandle, String, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths, cfg).await.unwrap();
    let base = format!("http://{}", handle.addr);
    (handle, base, token, dir)
}

#[tokio::test]
async fn login_state_policy() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let v: serde_json::Value = client
        .get(format!("{base}/v1/browser/login-state"))
        .header("X-Vcu-Token", &token)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["ok"], true);
    let d = &v["data"];
    assert_eq!(d["never_click_allow"], true);
    assert_eq!(d["never_os_cursor"], true);
    assert_eq!(d["never_wechat"], true);
    let next = d["next_action"].as_str().unwrap_or("");
    assert!(!next.contains("click Allow debugging"), "{next}");
    assert!(!next.contains("User: click Allow"), "{next}");
    let note = d["cdp_note"].as_str().unwrap_or("");
    assert!(note.to_ascii_lowercase().contains("abandon") || note.contains("Do not click Allow"), "{note}");
    handle.join.abort();
}

#[tokio::test]
async fn key_return_dry_run_blocked() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let v: serde_json::Value = client
        .post(format!("{base}/v1/browser/key"))
        .header("X-Vcu-Token", &token)
        .json(&json!({"key":"return","dry_run":true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["ok"], true);
    assert_eq!(v["data"]["blocked"], true);
    assert_eq!(v["data"]["pressed"], false);
    assert_eq!(v["data"]["hid_injected"], false);
    handle.join.abort();
}

#[tokio::test]
async fn key_return_live_rejected() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/browser/key"))
        .header("X-Vcu-Token", &token)
        .json(&json!({"key":"return","dry_run":false}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["ok"], false);
    assert_eq!(v["error"]["code"], "FocusPolicyViolation");
    handle.join.abort();
}

#[tokio::test]
async fn mock_session_core() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);

    let started: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"backend":"mock","browser":"mock"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(started["ok"], true);
    assert_eq!(started["data"]["policy"]["os_cursor"], "deny");
    let sid = started["data"]["session_id"].as_str().unwrap();

    assert!(auth(client.post(format!("{base}/v1/session/{sid}/navigate")))
        .json(&json!({"url":"https://example.com/"}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());

    let snap: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"full","budget":2000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["ok"], true);
    assert!(snap["data"]["dom_refs"].as_array().unwrap().len() >= 1);

    let click: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/click")))
        .json(&json!({"ref":"e3"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(click["ok"], true);
    assert_eq!(click["data"]["detail"]["os_cursor_used"], false);

    let typed: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/type")))
        .json(&json!({"ref":"e4","text":"hello"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(typed["ok"], true);

    let extracted: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/extract")))
        .json(&json!({"selector":"button"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(extracted["data"]["count"].as_u64().unwrap() >= 1);

    let scrolled: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"scroll","target":{},"args":{"dy":400}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(scrolled["ok"], true);
    assert_eq!(scrolled["data"]["detail"]["os_cursor_used"], false);

    let waited: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"wait","target":{},"args":{"ms":10}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(waited["ok"], true);

    let denied = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"os_click","target":{},"args":{}}))
        .send()
        .await
        .unwrap();
    assert_eq!(denied.status(), 409);

    let _ = auth(client.post(format!("{base}/v1/session/{sid}/stop")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    handle.join.abort();
    tokio::time::sleep(Duration::from_millis(30)).await;
}

#[tokio::test]
async fn extract_requires_user_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/browser/extract"))
        .header("X-Vcu-Token", &token)
        .json(&json!({"selector":"a"}))
        .send()
        .await
        .unwrap();
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["ok"], false);
    let code = v["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "ExtensionDisconnected" || code == "ActionFailed",
        "{code} {v}"
    );
    handle.join.abort();
}

#[tokio::test]
async fn extract_with_fake_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["bilibili.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());

    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..80 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=200"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = match method {
                "list_tabs" => json!({
                    "ok": true,
                    "tabs": [{
                        "tab_id": "42",
                        "window_id": "1",
                        "title": "Example",
                        "url": "https://example.com/",
                        "agent_owned": false,
                        "borrowed_by": null
                    }]
                }),
                "extract" => json!({
                    "ok": true,
                    "tab_id": "42",
                    "matches": [{"tag":"A","text":"hi","href":"https://example.com/","value":null}],
                    "count": 1
                }),
                _ => json!({"ok": false, "error": method}),
            };
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "extract" {
                break;
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(400)).await;
    let extracted: serde_json::Value = auth(client.post(format!("{base}/v1/browser/extract")))
        .json(&json!({"selector": "a"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(extracted["ok"], true, "{extracted}");
    assert_eq!(extracted["data"]["login_state"], true);
    assert_eq!(extracted["data"]["hud"], false);
    assert_eq!(extracted["data"]["count"], 1);
    assert_eq!(extracted["data"]["tab_id"], "42");
    assert_eq!(extracted["data"]["source"], "extension_dom");
    let _ = poller.await;
    handle.join.abort();
}

#[tokio::test]
async fn ping_with_fake_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());

    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..40 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = match method {
                "ping" => json!({"ok": true, "pong": true, "version": "0.1.5"}),
                _ => json!({"ok": false, "error": method}),
            };
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "ping" {
                break;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let pinged: serde_json::Value = auth(client.post(format!("{base}/v1/browser/ping")))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(pinged["ok"], true, "{pinged}");
    assert_eq!(pinged["data"]["pong"], true);
    let _ = poller.await;
    handle.join.abort();
}

#[tokio::test]
async fn ping_unknown_method_is_stale() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..40 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = json!({"ok": false, "error": "unknown method ping"});
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "ping" {
                break;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let pinged: serde_json::Value = auth(client.post(format!("{base}/v1/browser/ping")))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(pinged["ok"], false, "{pinged}");
    let msg = pinged["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("stale") || msg.contains("Reload"), "{pinged}");
    assert!(!msg.contains("click Allow debugging"), "{msg}");
    let _ = poller.await;
    handle.join.abort();
}

#[tokio::test]
async fn extract_timeout_is_error_not_ax_ok() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..80 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=200"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            if method == "extract" {
                // leave the waiter hanging until daemon timeout
                continue;
            }
            let result = match method {
                "list_tabs" => json!({
                    "ok": true,
                    "tabs": [{
                        "tab_id": "7",
                        "window_id": "1",
                        "title": "Example",
                        "url": "https://example.com/",
                        "agent_owned": false,
                        "borrowed_by": null
                    }]
                }),
                _ => json!({"ok": false, "error": method}),
            };
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let extracted: serde_json::Value = auth(client.post(format!("{base}/v1/browser/extract")))
        .json(&json!({"selector": "a"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(extracted["ok"], false, "{extracted}");
    let blob = extracted.to_string();
    assert!(!blob.contains("ax_scene_fallback"), "{blob}");
    assert!(!blob.contains("click Allow debugging"), "{blob}");
    let msg = extracted["error"]["message"].as_str().unwrap_or("");
    assert!(msg.contains("timed out") || msg.contains("failed") || msg.contains("Reload"), "{extracted}");
    poller.abort();
    handle.join.abort();
}

#[tokio::test]
async fn login_state_marks_stale_sw() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..40 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = json!({"ok": false, "error": "unknown method ping"});
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "ping" {
                break;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let v: serde_json::Value = auth(client.get(format!("{base}/v1/browser/login-state")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["data"]["extension_sw_stale"], true, "{v}");
    let next = v["data"]["next_action"].as_str().unwrap_or("");
    assert!(next.contains("Reload"), "{next}");
    assert!(!next.contains("click Allow debugging"), "{next}");
    assert_eq!(v["data"]["never_click_allow"], true);
    let _ = poller.await;
    handle.join.abort();
}

#[tokio::test]
async fn type_selector_requires_user_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let v: serde_json::Value = client
        .post(format!("{base}/v1/browser/type"))
        .header("X-Vcu-Token", &token)
        .json(&json!({"selector":"input","text":"x","dry_run":true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["ok"], false, "{v}");
    let code = v["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "ExtensionDisconnected" || code == "ActionFailed",
        "{code} {v}"
    );
    handle.join.abort();
}

#[tokio::test]
async fn type_selector_with_fake_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..40 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = match method {
                "type" => json!({"ok": true, "typed": true, "source": "extension_dom", "tab_id": "9"}),
                _ => json!({"ok": false, "error": method}),
            };
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "type" {
                break;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let typed: serde_json::Value = auth(client.post(format!("{base}/v1/browser/type")))
        .json(&json!({"selector": "input[name=q]", "text": "hello", "dry_run": false}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(typed["ok"], true, "{typed}");
    assert_eq!(typed["data"]["source"], "extension_dom");
    assert_eq!(typed["data"]["hud"], false);
    assert_eq!(typed["data"]["os_cursor_used"], false);
    let _ = poller.await;
    handle.join.abort();
}

#[tokio::test]
async fn click_selector_requires_user_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let v: serde_json::Value = client
        .post(format!("{base}/v1/browser/click"))
        .header("X-Vcu-Token", &token)
        .json(&json!({"selector":"a","dry_run":true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(v["ok"], false, "{v}");
    let code = v["error"]["code"].as_str().unwrap_or("");
    assert!(
        code == "ExtensionDisconnected" || code == "ActionFailed",
        "{code} {v}"
    );
    handle.join.abort();
}

#[tokio::test]
async fn click_selector_with_fake_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..40 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = match method {
                "click" => json!({"ok": true, "pressed": false, "dry_run": true, "source": "extension_dom", "tab_id": "99", "page_url": "https://example.com/", "focused": true}),
                _ => json!({"ok": false, "error": method}),
            };
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "click" {
                break;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let clicked: serde_json::Value = auth(client.post(format!("{base}/v1/browser/click")))
        .json(&json!({"selector": "a.more", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(clicked["ok"], true, "{clicked}");
    assert_eq!(clicked["data"]["source"], "extension_dom");
    assert_eq!(clicked["data"]["hud"], false);
    assert_eq!(clicked["data"]["os_cursor_used"], false);
    assert_eq!(clicked["data"]["tab_id"], "99");
    assert_eq!(clicked["data"]["page_url"], "https://example.com/");
    assert_eq!(clicked["data"]["focused"], true);
    let _ = poller.await;
    handle.join.abort();
}

#[tokio::test]
async fn scroll_with_fake_extension_uses_dom() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = reqwest::Client::new();
    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);
    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({"likely_user_profile": true, "hosts": ["example.com"]}))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let poller_base = base.clone();
    let poller_token = token.clone();
    let poller = tokio::spawn(async move {
        let c = reqwest::Client::new();
        for _ in 0..40 {
            let body: serde_json::Value = c
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150"))
                .header("X-Vcu-Token", &poller_token)
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            let data = body.get("data").cloned().unwrap_or(json!({}));
            let id = data.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let method = data.get("method").and_then(|v| v.as_str()).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let result = match method {
                "scroll" => json!({"ok": true, "scrolled": true, "dy": 600, "source": "extension_dom"}),
                _ => json!({"ok": false, "error": method}),
            };
            let _ = c
                .post(format!("{poller_base}/v1/extension/result"))
                .header("X-Vcu-Token", &poller_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
            if method == "scroll" {
                break;
            }
        }
    });
    tokio::time::sleep(Duration::from_millis(250)).await;
    let scrolled: serde_json::Value = auth(client.post(format!("{base}/v1/browser/scroll")))
        .json(&json!({"dy": 600, "dry_run": false}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(scrolled["ok"], true, "{scrolled}");
    assert_eq!(scrolled["data"]["source"], "extension_dom");
    assert_eq!(scrolled["data"]["scrolled"], true);
    assert_eq!(scrolled["data"]["os_cursor_used"], false);
    let _ = poller.await;
    handle.join.abort();
}
