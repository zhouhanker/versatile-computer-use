use std::time::Duration;
use serde_json::json;
use vcu_core::{UserConfig, VcuPaths};
use vcu_server::app::mock_app::MockAppBackend;

#[tokio::test]
async fn desktop_surface_scene_actuator_and_wechat_denied() {
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

    let started: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"surface":"desktop","app_id":"proc:TextEdit:1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(started["ok"], true);
    assert_eq!(started["data"]["surface"], "desktop");
    assert_eq!(started["data"]["adapter"], "desktop");
    assert_eq!(started["data"]["backend"], "desktop");
    assert_eq!(started["data"]["policy"]["os_cursor"], "deny");
    assert_eq!(started["data"]["stage_hud"], true, "{started}");
    assert_eq!(started["data"]["stage_presenter"], "noop");
    let sid = started["data"]["session_id"].as_str().unwrap().to_string();
    let active = started["data"]["active_app_id"].as_str().unwrap().to_string();
    assert!(active.contains("TextEdit"));

    let tabs: serde_json::Value = auth(client.get(format!("{base}/v1/session/{sid}/tabs")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let wechat = tabs["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["title"] == "WeChat")
        .unwrap();
    assert_eq!(wechat["agent_owned"], false);
    let user_edge = tabs["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["browser_profile"] == "user")
        .unwrap();
    assert_eq!(user_edge["login_state"], true);
    assert!(user_edge["tab_id"].as_str().unwrap().contains("Microsoft_Edge"));

    let snap: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"a11y","budget":2000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["ok"], true);
    assert_eq!(snap["data"]["kind"], "desktop.scene");
    assert_eq!(snap["data"]["surface"], "desktop");
    assert_eq!(snap["data"]["source"], "ax_scene");
    assert_ne!(snap["data"]["source"], "extension_dom");
    assert!(snap["data"]["dom_refs"].as_array().unwrap().len() >= 2);
    assert_eq!(snap["data"]["dom_refs"][0]["frame"], json!([20.0, 20.0, 80.0, 24.0]));

    let click: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/click")))
        .json(&json!({"ref":"e1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(click["ok"], true);
    assert_eq!(click["data"]["detail"]["os_cursor_used"], false);
    assert_eq!(click["data"]["detail"]["guide"]["overlay"], true);
    assert_eq!(click["data"]["detail"]["guide"]["x"], 60.0);
    assert_eq!(click["data"]["detail"]["guide"]["os_cursor_used"], false);

    let full: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"full","budget":2000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(full["ok"], true);
    assert!(full["data"]["screenshot_ref"].as_str().unwrap().starts_with("cas://sha256/"));

    let shot: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/screenshot")))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(shot["ok"], true);

    let typed: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/type")))
        .json(&json!({"text":"hello","ref":"e2"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(typed["ok"], true);
    assert_eq!(typed["data"]["detail"]["input_path"], "ax_set_value");

    let scrolled: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"scroll","target":{"ref":"e1"},"args":{"dy":600}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(scrolled["ok"], true);
    assert_eq!(scrolled["data"]["detail"]["input_path"], "ax_scroll");
    assert_eq!(scrolled["data"]["detail"]["os_cursor_used"], false);
    assert_eq!(scrolled["data"]["detail"]["dy"], 600);

    let gated: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"key","target":{"ref":"e1"},"args":{"key":"return"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(gated["ok"], false);
    assert_eq!(gated["error"]["code"], "FocusPolicyViolation");

    let blind: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"key","target":{},"args":{"key":"enter","confirm_send":true}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(blind["ok"], false);
    assert_eq!(blind["error"]["code"], "FocusPolicyViolation");

    let sent: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"key","target":{"ref":"e1"},"args":{"key":"return","confirm_send":true}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(sent["ok"], true);
    assert_eq!(sent["data"]["detail"]["input_path"], "ax_press_send");
    assert_eq!(sent["data"]["detail"]["hid_injected"], false);
    assert_eq!(sent["data"]["detail"]["os_cursor_used"], false);

    let esc: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"key","target":{},"args":{"key":"escape"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(esc["ok"], false);
    assert_eq!(esc["error"]["code"], "FocusPolicyViolation");

    let waited: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"wait","target":{"ref":"e1"},"args":{"ms":200,"name":"OK"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(waited["ok"], true);
    assert_eq!(waited["data"]["detail"]["found_ref"], "e1");
    assert_eq!(waited["data"]["detail"]["os_cursor_used"], false);

    let timed: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"wait","target":{},"args":{"ms":50,"name":"SendToNobody"}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(timed["ok"], false);
    assert_eq!(timed["error"]["code"], "ActionFailed");

    let hovered: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"hover","target":{"ref":"e1"},"args":{}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hovered["ok"], true);
    assert_eq!(hovered["data"]["detail"]["input_path"], "guide_hover");
    assert_eq!(hovered["data"]["detail"]["guide"]["overlay"], true);
    assert_eq!(hovered["data"]["detail"]["os_cursor_used"], false);

    let denied: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"tab_id": wechat["tab_id"], "mode":"a11y"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(denied["ok"], false);
    assert_eq!(denied["error"]["code"], "AppDenied");

    let feishu = tabs["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["title"] == "Feishu")
        .unwrap();
    let fs_id = feishu["tab_id"].as_str().unwrap();
    let fs_snap: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"tab_id": fs_id, "mode":"a11y","budget":2000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fs_snap["ok"], true);
    assert!(fs_snap["data"]["a11y_summary"].as_str().unwrap_or("").contains("webview=true")
        || fs_snap["data"]["kind"] == "desktop.scene");
    let hit: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/click")))
        .json(&json!({"ref":"e15","tab_id": fs_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hit["ok"], true);
    assert_eq!(hit["data"]["detail"]["input_path"], "ax_frame_hit");
    assert_eq!(hit["data"]["detail"]["os_cursor_used"], false);
    assert_eq!(hit["data"]["detail"]["hid_injected"], false);
    assert_eq!(hit["data"]["detail"]["guide"]["overlay"], true);
    assert_eq!(hit["data"]["detail"]["guide"]["x"], 2218.0 + 1424.0 / 2.0);
    assert_eq!(hit["data"]["detail"]["guide"]["y"], 36.0 + 1038.0 / 2.0);

    let hit_act: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"hit","target":{"ref":"e15"},"args":{"tab_id": fs_id}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(hit_act["ok"], true);
    assert_eq!(hit_act["data"]["detail"]["input_path"], "ax_frame_hit");
    assert_eq!(hit_act["data"]["detail"]["os_cursor_used"], false);

    let fs_full: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"tab_id": fs_id, "mode":"full","budget":2000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fs_full["ok"], true);
    assert_eq!(fs_full["data"]["webview"], true);
    assert_eq!(fs_full["data"]["webview_ref"], "e15");
    assert!(fs_full["data"]["webview_screenshot_ref"]
        .as_str()
        .unwrap()
        .starts_with("cas://sha256/"));
    assert_eq!(fs_full["data"]["webview_screenshot_scale"], 1.0);

    let fs_shot: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/screenshot")))
        .json(&json!({"tab_id": fs_id}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(fs_shot["ok"], true);
    assert_eq!(fs_shot["data"]["webview"], true);
    assert_eq!(fs_shot["data"]["webview_ref"], "e15");
    assert_eq!(
        fs_shot["data"]["webview_frame"],
        json!([2218.0, 36.0, 1424.0, 1038.0])
    );
    assert!(fs_shot["data"]["webview_path"].as_str().unwrap().ends_with(".png"));
    assert!(fs_shot["data"]["webview_bytes"].as_u64().unwrap() > 8);
    assert_eq!(fs_shot["data"]["webview_scale"], 1.0);

    let warp: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"os_click","target":{},"args":{}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(warp["ok"], false);
    assert_eq!(warp["error"]["code"], "OsCursorDenied");

    let browser: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"backend":"mock"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(browser["data"]["surface"], "browser_agent");
    assert_eq!(browser["data"]["adapter"], "browser");

    let _ = auth(client.post(format!("{base}/v1/session/{sid}/stop")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    handle.join.abort();
}

#[tokio::test]
async fn session_abort_removes_desktop_session() {
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

    let started: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"surface":"desktop","app_id":"proc:TextEdit:1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(started["ok"], true, "{started}");
    assert_eq!(started["data"]["stage_hud"], true);
    let sid = started["data"]["session_id"].as_str().unwrap().to_string();

    let aborted: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/abort")))
        .json(&json!({}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(aborted["ok"], true, "{aborted}");
    assert_eq!(aborted["data"]["aborted"], true);
    assert_eq!(aborted["data"]["hud"], false);
    assert_eq!(aborted["data"]["stage_signaled"], true, "{aborted}");

    let listed: serde_json::Value = auth(client.get(format!("{base}/v1/session/list")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let arr = listed["data"].as_array().unwrap();
    assert!(arr.iter().all(|s| s["session_id"] != sid), "{listed}");
}

#[tokio::test]
async fn type_without_tab_uses_active_app_not_first_window() {
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

    // First agent-owned mock window is TextEdit; pin Feishu via app_id.
    let started: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"surface":"desktop","app_id":"proc:Feishu:3"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(started["ok"], true, "{started}");
    let sid = started["data"]["session_id"].as_str().unwrap().to_string();
    assert!(
        started["data"]["active_app_id"]
            .as_str()
            .unwrap()
            .contains("Feishu"),
        "{started}"
    );

    let typed: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/type")))
        .json(&json!({"text":"hello","ref":"e1"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(typed["ok"], true, "{typed}");
    assert_eq!(typed["data"]["detail"]["process"], "Feishu", "{typed}");
    assert_eq!(typed["data"]["detail"]["os_cursor_used"], false);
    handle.join.abort();
}

#[tokio::test]
async fn desktop_scene_attaches_extension_tabs_for_edge() {
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
                    }],
                    "groups": [],
                    "source": "extension_tabs"
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

    let started: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"surface":"desktop","app_id":"proc:Microsoft_Edge:10"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(started["ok"], true, "{started}");
    let sid = started["data"]["session_id"].as_str().unwrap().to_string();

    let snap: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"a11y","budget":2000,"tab_id":"proc:Microsoft_Edge:10"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    poller.abort();
    assert_eq!(snap["ok"], true, "{snap}");
    assert_eq!(snap["data"]["kind"], "desktop.scene");
    assert_eq!(snap["data"]["source"], "ax_scene");
    assert_ne!(snap["data"]["source"], "extension_dom");
    assert_eq!(snap["data"]["tabs_source"], "extension_tabs");
    assert_eq!(snap["data"]["browser_tabs"][0]["tab_id"], "42");
    assert_eq!(snap["data"]["page_url"], "https://edge.example/live");
    assert_eq!(snap["data"]["tab_id"], "42");
    assert!(snap["data"]["targets"].as_array().unwrap().iter().any(|t| t["tab_id"] == "proc:Microsoft_Edge:10"));
    handle.join.abort();
}
