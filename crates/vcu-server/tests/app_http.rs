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
