use std::time::Duration;
use serde_json::json;
use vcu_core::{UserConfig, VcuPaths};
use vcu_server::app::mock_app::MockAppBackend;

#[tokio::test]
async fn app_http_windows_snapshot_policy() {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths, cfg).await.unwrap();
    // Replace platform backend with mock for deterministic CI.
    {
        let mut b = handle.state.app_backend.write().await;
        *b = Box::new(MockAppBackend::default());
    }
    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();
    let auth = |r: reqwest::RequestBuilder| r.header("X-Vcu-Token", &token);

    let wins: serde_json::Value = auth(client.get(format!("{base}/v1/app/windows")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(wins["ok"], true);
    assert_eq!(wins["data"]["platform"], "mock-app");
    let id = wins["data"]["windows"][0]["id"].as_str().unwrap().to_string();

    let snap: serde_json::Value = auth(client.post(format!("{base}/v1/app/snapshot")))
        .json(&json!({"id": id, "budget": 1000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["ok"], true);
    assert!(snap["data"]["elements"].as_array().unwrap().len() >= 2);
    assert!(snap["data"].get("screenshot_path").is_none());

    let pix: serde_json::Value = auth(client.post(format!("{base}/v1/app/snapshot")))
        .json(&json!({"id": id, "budget": 1000, "pixels": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(pix["ok"], true);
    let path = pix["data"]["screenshot_path"].as_str().unwrap();
    assert!(std::path::Path::new(path).is_file());
    assert!(pix["data"]["screenshot_sha256"].as_str().unwrap().len() >= 16);
    assert!(pix["data"]["screenshot_bytes"].as_u64().unwrap() > 8);

    let focus: serde_json::Value = auth(client.post(format!("{base}/v1/app/focus")))
        .json(&json!({"id": id, "allow_focus_steal": false}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(focus["ok"], false);
    assert_eq!(focus["error"]["code"], "FocusPolicyViolation");

    let inv: serde_json::Value = auth(client.post(format!("{base}/v1/app/invoke")))
        .json(&json!({"id": id, "ref": "e1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(inv["ok"], true);
    assert_eq!(inv["data"]["os_cursor_used"], false);
    assert_eq!(inv["data"]["input_path"], "ax_press");

    let wechat = wins["data"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["title"] == "WeChat")
        .unwrap();
    let denied: serde_json::Value = auth(client.post(format!("{base}/v1/app/invoke")))
        .json(&json!({"id": wechat["id"], "ref": "e1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(denied["ok"], false);
    assert_eq!(denied["error"]["code"], "AppDenied");

    let feishu = wins["data"]["windows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["title"] == "Feishu")
        .unwrap();
    let fs_pix: serde_json::Value = auth(client.post(format!("{base}/v1/app/snapshot")))
        .json(&json!({"id": feishu["id"], "budget": 1000, "pixels": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fs_pix["ok"], true);
    assert_eq!(fs_pix["data"]["webview"], true);
    assert_eq!(fs_pix["data"]["webview_ref"], "e15");
    let wv = fs_pix["data"]["webview_screenshot_path"].as_str().unwrap();
    assert!(std::path::Path::new(wv).is_file());
    assert!(fs_pix["data"]["webview_screenshot_bytes"].as_u64().unwrap() > 8);
    assert_eq!(
        fs_pix["data"]["webview_screenshot_frame"],
        json!([2218.0, 36.0, 1424.0, 1038.0])
    );
    assert_eq!(fs_pix["data"]["webview_screenshot_scale"], 1.0);
    assert_eq!(fs_pix["data"]["webview_screenshot_width"], 1424);
    assert_eq!(fs_pix["data"]["webview_screenshot_height"], 1038);

    handle.join.abort();
}


#[tokio::test]
async fn snapshot_merges_extension_tabs_for_empty_ax_edge() {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths, cfg).await.unwrap();
    {
        let mut b = handle.state.app_backend.write().await;
        *b = Box::new(MockAppBackend::default());
    }
    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();
    let auth = |r: reqwest::RequestBuilder| r.header("X-Vcu-Token", &token);

    assert!(auth(client.post(format!("{base}/v1/extension/hello")))
        .json(&json!({
            "likely_user_profile": true,
            "browser": "edge",
            "client_id": "edge-test"
        }))
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
                .get(format!("{poller_base}/v1/extension/poll?wait_ms=150&browser=edge&client_id=edge-test"))
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
                        "title": "edge-live",
                        "url": "https://edge.example/live",
                        "active": true,
                        "focused": true,
                        "browser": "edge"
                    }, {
                        "tab_id": "99",
                        "title": "edge-other",
                        "url": "https://edge.example/other",
                        "active": false,
                        "focused": false,
                        "browser": "edge"
                    }, {
                        "tab_id": "42",
                        "title": "chrome-live",
                        "url": "https://chrome.example/live",
                        "active": true,
                        "focused": true,
                        "browser": "chrome"
                    }],
                    "groups": [],
                    "source": "extension_tabs"
                }),
                "select_tab" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("99"));
                    json!({"ok": true, "tab_id": tab, "source": "extension_tabs", "focused": false})
                }
                "close_tab" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    json!({"ok": true, "closed": true, "tab_id": tab, "source": "extension_tabs"})
                }
                "open_tab" => json!({"ok": true, "tab_id": "1001", "url": "https://edge.example/new", "source": "extension_tabs"}),
                "group_tabs" => json!({"ok": true, "group_id": "g1", "source": "extension_tabs"}),
                "ungroup_tabs" => json!({"ok": true, "source": "extension_tabs"}),
                "click" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!(null));
                    json!({"ok": true, "pressed": false, "dry_run": true, "source": "extension_dom", "tab_id": tab, "page_url": "https://edge.example/live", "focused": true})
                }
                "extract" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    let sel = data.get("params").and_then(|p| p.get("selector")).and_then(|v| v.as_str()).unwrap_or("");
                    if sel.contains("missing") {
                        json!({"ok": true, "source": "extension_dom", "tab_id": tab, "count": 0, "matches": []})
                    } else {
                        json!({"ok": true, "source": "extension_dom", "tab_id": tab, "count": 1, "matches": [{"text": "edge-live"}]})
                    }
                }
                "hover" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    json!({"ok": true, "hovered": false, "dry_run": true, "source": "extension_dom", "tab_id": tab})
                }
                "click_point" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    json!({"ok": true, "pressed": false, "dry_run": true, "source": "extension_dom", "tab_id": tab})
                }
                "type" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    json!({"ok": true, "typed": false, "dry_run": true, "source": "extension_dom", "tab_id": tab})
                }
                "scroll" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    json!({"ok": true, "scrolled": false, "dry_run": true, "source": "extension_dom", "tab_id": tab})
                }
                "capture_tab" => {
                    let tab = data.get("params").and_then(|p| p.get("tab_id")).cloned().unwrap_or(json!("42"));
                    json!({
                        "ok": true,
                        "tab_id": tab,
                        "png_base64": "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==",
                        "viewport": {"document_id": "doc-1", "width": 100.0, "height": 80.0}
                    })
                }
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

    let snap: serde_json::Value = auth(client.post(format!("{base}/v1/app/snapshot")))
        .json(&json!({"id": "proc:Microsoft_Edge:10", "budget": 1000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["ok"], true, "{snap}");
    assert_eq!(snap["data"]["tabs_source"], "extension_tabs");
    assert_eq!(snap["data"]["page_url"], "https://edge.example/live");
    assert_eq!(snap["data"]["page_url_source"], "extension_tabs");
    assert_eq!(snap["data"]["scene_source"], "ax_scene");
    assert_eq!(snap["data"]["tabs"][0]["tab_id"], "42");
    assert_eq!(snap["data"]["tab_id"], "42");
    assert_eq!(snap["data"]["tab_id_source"], "extension_tabs");
    assert!(snap["data"]["elements"].as_array().unwrap().iter().any(|e| e["ref"] == "e_web"));
    assert_ne!(snap["data"].get("source").and_then(|v| v.as_str()), Some("extension_dom"));

    let observed: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"id": "proc:Microsoft_Edge:10", "pixels": false, "budget": 1000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(observed["ok"], true, "{observed}");
    assert_eq!(observed["data"]["hud"], false);
    assert_eq!(observed["data"]["login_state"], true);
    assert_eq!(observed["data"]["tab_id"], "42");
    assert_eq!(observed["data"]["tabs_source"], "extension_tabs");
    assert_eq!(observed["data"]["snapshot"]["tab_id"], "42");

    vcu_server::login_state::set_frontmost_user_browser_override(None);
    let missing_front: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"pixels": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(missing_front["ok"], false, "{missing_front}");
    assert_eq!(missing_front["error"]["code"], "InvalidInput", "{missing_front}");
    let miss_msg = missing_front["error"]["message"].as_str().unwrap_or("");
    assert!(miss_msg.contains("frontmost is not USER Chrome/Edge"), "{missing_front}");
    assert!(miss_msg.contains("observe --tab"), "{missing_front}");

    vcu_server::login_state::set_frontmost_user_browser_override(Some("Microsoft Edge"));
    let lens_obs: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"pixels": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(lens_obs["ok"], true, "{lens_obs}");
    assert_eq!(lens_obs["data"]["tab_id"], "42");
    assert_eq!(lens_obs["data"]["tabs_source"], "extension_tabs");
    assert_eq!(lens_obs["data"]["tab_id_source"], "extension_tabs");
    let capture = lens_obs["data"]["capture_id"].as_str().unwrap();
    assert_eq!(capture.len(), 26, "{lens_obs}");
    let point: serde_json::Value = auth(client.post(format!("{base}/v1/browser/click")))
        .json(&json!({"space":"viewport","capture_id":capture,"pixel_x":0.0,"pixel_y":0.0,"dry_run":true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(point["ok"], true, "{point}");
    assert_eq!(point["data"]["source"], "extension_dom");
    assert_eq!(point["data"]["tab_id"], "42");

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
    assert_eq!(clicked["data"]["tab_id"], "42");
    assert_eq!(clicked["data"]["tab_id_source"], "last_observe");

    let shot: serde_json::Value = auth(client.post(format!("{base}/v1/browser/screenshot")))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(shot["ok"], true, "{shot}");
    assert_eq!(shot["data"]["source"], "extension_viewport");
    assert_eq!(shot["data"]["tab_id"], "42");
    assert_eq!(shot["data"]["tab_id_source"], "last_observe");

    let extracted: serde_json::Value = auth(client.post(format!("{base}/v1/browser/extract")))
        .json(&json!({"selector": "title"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(extracted["ok"], true, "{extracted}");
    assert_eq!(extracted["data"]["source"], "extension_dom");
    assert_eq!(extracted["data"]["tab_id"], "42");
    assert_eq!(extracted["data"]["tab_id_source"], "last_observe");

    let typed: serde_json::Value = auth(client.post(format!("{base}/v1/browser/type")))
        .json(&json!({"selector": "input", "text": "hello", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(typed["ok"], true, "{typed}");
    assert_eq!(typed["data"]["source"], "extension_dom");
    assert_eq!(typed["data"]["tab_id"], "42");
    assert_eq!(typed["data"]["tab_id_source"], "last_observe");
    assert_eq!(typed["data"]["typed"], false);
    assert_eq!(typed["data"]["os_cursor_used"], false);

    let scrolled: serde_json::Value = auth(client.post(format!("{base}/v1/browser/scroll")))
        .json(&json!({"dy": 80, "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(scrolled["ok"], true, "{scrolled}");
    assert_eq!(scrolled["data"]["source"], "extension_dom");
    assert_eq!(scrolled["data"]["tab_id"], "42");
    assert_eq!(scrolled["data"]["tab_id_source"], "last_observe");
    assert_eq!(scrolled["data"]["scrolled"], false);
    assert_eq!(scrolled["data"]["os_cursor_used"], false);

    let hovered: serde_json::Value = auth(client.post(format!("{base}/v1/browser/hover")))
        .json(&json!({"selector": "#pad", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hovered["ok"], true, "{hovered}");
    assert_eq!(hovered["data"]["source"], "extension_dom");
    assert_eq!(hovered["data"]["tab_id"], "42");
    assert_eq!(hovered["data"]["tab_id_source"], "last_observe");
    assert_eq!(hovered["data"]["hovered"], false);

    let waited: serde_json::Value = auth(client.post(format!("{base}/v1/browser/wait")))
        .json(&json!({"selector": "title", "text": "edge-live", "ms": 500}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(waited["ok"], true, "{waited}");
    assert_eq!(waited["data"]["source"], "extension_dom");
    assert_eq!(waited["data"]["tab_id"], "42");
    assert_eq!(waited["data"]["tab_id_source"], "last_observe");
    assert_eq!(waited["data"]["found"], true);

    let wait_miss: serde_json::Value = auth(client.post(format!("{base}/v1/browser/wait")))
        .json(&json!({"selector": "#missing", "ms": 120}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(wait_miss["ok"], false, "{wait_miss}");
    assert_eq!(wait_miss["error"]["code"], "ActionFailed", "{wait_miss}");

    let ambiguous: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"pixels": false, "tab_id": "42"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ambiguous["ok"], false, "{ambiguous}");
    assert_eq!(ambiguous["error"]["code"], "InvalidInput", "{ambiguous}");
    let amsg = ambiguous["error"]["message"].as_str().unwrap_or("");
    assert!(amsg.contains("ambiguous"), "{ambiguous}");
    assert!(amsg.contains("--browser"), "{ambiguous}");

    let chrome_obs: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"pixels": false, "tab_id": "42", "browser": "chrome"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(chrome_obs["ok"], true, "{chrome_obs}");
    assert_eq!(chrome_obs["data"]["tab_id"], "42");
    assert_eq!(chrome_obs["data"]["browser"], "chrome");
    assert_eq!(chrome_obs["data"]["page_url"], "https://chrome.example/live");

    let edge_obs: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"pixels": false, "tab_id": "42", "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(edge_obs["ok"], true, "{edge_obs}");
    assert_eq!(edge_obs["data"]["browser"], "edge");
    assert_eq!(edge_obs["data"]["page_url"], "https://edge.example/live");

    let close_amb: serde_json::Value = auth(client.post(format!("{base}/v1/browser/close")))
        .json(&json!({"tab_id": "42"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(close_amb["ok"], false, "{close_amb}");
    assert_eq!(close_amb["error"]["code"], "InvalidInput", "{close_amb}");
    assert!(close_amb["error"]["message"].as_str().unwrap_or("").contains("ambiguous"), "{close_amb}");

    let closed: serde_json::Value = auth(client.post(format!("{base}/v1/browser/close")))
        .json(&json!({"tab_id": "42", "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(closed["ok"], true, "{closed}");
    assert_eq!(closed["data"]["closed"], true);
    assert_eq!(closed["data"]["browser"], "edge");

    let selected: serde_json::Value = auth(client.post(format!("{base}/v1/browser/select")))
        .json(&json!({"tab_id": "99", "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(selected["ok"], true, "{selected}");
    assert_eq!(selected["data"]["tab_id"], "99");
    assert_eq!(selected["data"]["browser"], "edge");

    let extract_amb: serde_json::Value = auth(client.post(format!("{base}/v1/browser/extract")))
        .json(&json!({"selector": "#who", "tab_id": "42"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(extract_amb["ok"], false, "{extract_amb}");
    assert!(extract_amb["error"]["message"].as_str().unwrap_or("").contains("ambiguous"), "{extract_amb}");

    let extracted_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/extract")))
        .json(&json!({"selector": "#who", "tab_id": "42", "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(extracted_edge["ok"], true, "{extracted_edge}");
    assert_eq!(extracted_edge["data"]["browser"], "edge");
    assert_eq!(extracted_edge["data"]["tab_id"], "42");

    let typed_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/type")))
        .json(&json!({"selector": "input", "text": "hi", "tab_id": "42", "browser": "edge", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(typed_edge["ok"], true, "{typed_edge}");
    assert_eq!(typed_edge["data"]["browser"], "edge");

    let clicked_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/click")))
        .json(&json!({"selector": "body", "tab_id": "42", "browser": "edge", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(clicked_edge["ok"], true, "{clicked_edge}");
    assert_eq!(clicked_edge["data"]["browser"], "edge");

    let hover_amb: serde_json::Value = auth(client.post(format!("{base}/v1/browser/hover")))
        .json(&json!({"selector": "#who", "tab_id": "42"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hover_amb["ok"], false, "{hover_amb}");
    assert!(hover_amb["error"]["message"].as_str().unwrap_or("").contains("ambiguous"), "{hover_amb}");

    let hovered_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/hover")))
        .json(&json!({"selector": "#who", "tab_id": "42", "browser": "edge", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hovered_edge["ok"], true, "{hovered_edge}");
    assert_eq!(hovered_edge["data"]["browser"], "edge");

    let scrolled_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/scroll")))
        .json(&json!({"dy": 80, "tab_id": "42", "browser": "edge", "dry_run": true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(scrolled_edge["ok"], true, "{scrolled_edge}");
    assert_eq!(scrolled_edge["data"]["browser"], "edge");

    let waited_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/wait")))
        .json(&json!({"selector": "title", "text": "edge-live", "tab_id": "42", "browser": "edge", "ms": 500}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(waited_edge["ok"], true, "{waited_edge}");
    assert_eq!(waited_edge["data"]["browser"], "edge");

    let shot_edge: serde_json::Value = auth(client.post(format!("{base}/v1/browser/screenshot")))
        .json(&json!({"tab_id": "42", "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(shot_edge["ok"], true, "{shot_edge}");
    assert_eq!(shot_edge["data"]["browser"], "edge");
    assert_eq!(shot_edge["data"]["source"], "extension_viewport");

    let group_amb: serde_json::Value = auth(client.post(format!("{base}/v1/browser/group")))
        .json(&json!({"tab_ids": ["42"], "title": "x"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(group_amb["ok"], false, "{group_amb}");
    assert!(group_amb["error"]["message"].as_str().unwrap_or("").contains("ambiguous"), "{group_amb}");

    let grouped: serde_json::Value = auth(client.post(format!("{base}/v1/browser/group")))
        .json(&json!({"tab_ids": ["99"], "title": "x", "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(grouped["ok"], true, "{grouped}");
    assert_eq!(grouped["data"]["browser"], "edge");

    let opened: serde_json::Value = auth(client.post(format!("{base}/v1/browser/open")))
        .json(&json!({"url": "https://example.com/", "active": false, "browser": "edge"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(opened["ok"], true, "{opened}");
    assert_eq!(opened["data"]["browser"], "edge");

    let targeted: serde_json::Value = auth(client.post(format!("{base}/v1/browser/observe")))
        .json(&json!({"pixels": true, "tab_id": "99"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(targeted["ok"], true, "{targeted}");
    assert_eq!(targeted["data"]["tab_id"], "99");
    assert_eq!(targeted["data"]["tabs_source"], "extension_tabs");
    let targeted_capture = targeted["data"]["capture_id"].as_str().unwrap();
    assert_eq!(targeted_capture.len(), 26, "{targeted}");
    let targeted_click: serde_json::Value = auth(client.post(format!("{base}/v1/browser/click")))
        .json(&json!({"space":"viewport","capture_id":targeted_capture,"pixel_x":0.0,"pixel_y":0.0,"dry_run":true}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(targeted_click["ok"], true, "{targeted_click}");
    assert_eq!(targeted_click["data"]["source"], "extension_dom");
    assert_eq!(targeted_click["data"]["tab_id"], "99");
    vcu_server::login_state::clear_frontmost_user_browser_override();
    poller.abort();

    let textedit: serde_json::Value = auth(client.post(format!("{base}/v1/app/snapshot")))
        .json(&json!({"id": "proc:TextEdit:1", "budget": 1000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(textedit["ok"], true, "{textedit}");
    assert!(textedit["data"].get("tabs_source").is_none());
    handle.join.abort();
}
