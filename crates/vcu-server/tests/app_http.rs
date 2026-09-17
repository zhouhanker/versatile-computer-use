use serde_json::json;
use vcu_core::{UserConfig, VcuPaths};
use vcu_server::app::mock_app::MockAppBackend;

#[tokio::test]
async fn app_http_windows_snapshot_policy() {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 19200 + (std::process::id() % 400) as u16;
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
    assert_eq!(inv["ok"], false);
    assert_eq!(inv["error"]["code"], "OsCursorDenied");

    handle.join.abort();
}
