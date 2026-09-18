use serde_json::json;
use vcu_core::{ModelConfig, UserConfig, VcuPaths, VisionPolicy};

#[tokio::test]
async fn snapshot_can_use_mock_vision_provider() {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    cfg.vision_policy = VisionPolicy::VisionAlways;
    cfg.models.insert(
        "vision".into(),
        ModelConfig {
            name: "vision".into(),
            provider: "mock".into(),
            base_url: "http://mock.local".into(),
            model: "mock-vl".into(),
            api_key_env: "VCU_UNUSED".into(),
            kind: "vision".into(),
        },
    );
    cfg.default_vision_model = Some("vision".into());
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths, cfg).await.unwrap();
    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();
    let auth = |r: reqwest::RequestBuilder| r.header("X-Vcu-Token", &token);

    let body: serde_json::Value = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"backend":"mock","browser":"mock","vision_policy":"vision_always"}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(body["ok"], true);
    let sid = body["data"]["session_id"].as_str().unwrap().to_string();

    let _ = auth(client.post(format!("{base}/v1/session/{sid}/navigate")))
        .json(&json!({"url":"https://example.com/"}))
        .send()
        .await
        .unwrap();

    let snap: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"full","budget":2000}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(snap["ok"], true, "{snap}");
    assert_eq!(snap["data"]["vision"]["used"], true);
    let summary = snap["data"]["vision"]["summary"].as_str().unwrap();
    assert!(summary.contains("mock-vision"), "{summary}");

    handle.join.abort();
}
