use std::time::Duration;

use serde_json::json;
use tempfile::tempdir;
use vcu_core::{ErrorCode, UserConfig, VcuPaths};

#[tokio::test]
async fn extension_bootstrap_returns_token_without_auth() {
    let dir = tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths.clone(), cfg).await.unwrap();
    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();

    let resp = client
        .post(format!("{base}/v1/extension/bootstrap"))
        .json(&json!({"client":"vcu-extension","version":"0.1.0"}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success(), "status={}", resp.status());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["data"]["token"], token);
    assert!(body["data"]["endpoint"].as_str().unwrap().starts_with("http://127.0.0.1:"));
    assert_eq!(body["data"]["already_connected"], false);

    // hello still requires the pairing token
    let bad = client
        .post(format!("{base}/v1/extension/hello"))
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert_eq!(bad.status(), 401);

    let hello = client
        .post(format!("{base}/v1/extension/hello"))
        .header("X-Vcu-Token", &token)
        .json(&json!({}))
        .send()
        .await
        .unwrap();
    assert!(hello.status().is_success());
    let health: serde_json::Value = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(health["data"]["extension_connected"], true);
    assert_eq!(health["data"]["extension_polling"], false);
    assert!(health["data"]["login_next_action"].as_str().unwrap().len() > 8);
    assert!(health["data"]["host_vision"].as_str().unwrap().contains("Grok") || health["data"]["host_vision"].as_str().unwrap().contains("vision"));

    handle.join.abort();
    tokio::time::sleep(Duration::from_millis(50)).await;
}

#[tokio::test]
async fn daemon_single_instance_lock_rejects_second_start() {
    let dir = tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let handle = vcu_server::start_daemon(paths.clone(), cfg.clone())
        .await
        .unwrap();

    let second = vcu_server::start_daemon(paths.clone(), cfg).await;
    let err = match second {
        Err(e) => e,
        Ok(_) => panic!("second start_daemon should fail with DaemonAlreadyRunning"),
    };
    assert_eq!(err.code(), ErrorCode::DaemonAlreadyRunning);

    handle.join.abort();
    drop(handle);
    tokio::time::sleep(Duration::from_millis(80)).await;
}

#[tokio::test]
async fn extension_bootstrap_rejects_non_extension_origin() {
    let dir = tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let handle = vcu_server::start_daemon(paths.clone(), cfg).await.unwrap();
    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();

    let evil = client
        .post(format!("{base}/v1/extension/bootstrap"))
        .header("Origin", "https://evil.example")
        .json(&json!({"client":"web"}))
        .send()
        .await
        .unwrap();
    assert_eq!(evil.status(), 401);

    let ext = client
        .post(format!("{base}/v1/extension/bootstrap"))
        .header("Origin", "chrome-extension://abcdefghijklmnopqrstuvwxyz")
        .json(&json!({"client":"vcu-extension"}))
        .send()
        .await
        .unwrap();
    assert!(ext.status().is_success(), "status={}", ext.status());
    let body: serde_json::Value = ext.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["data"]["token"].as_str().unwrap().len() > 8);

    let health: serde_json::Value = client
        .get(format!("{base}/v1/health"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(health["data"]["pid"].as_u64().unwrap() > 0);
    assert!(health["data"].get("lens_dir").is_some());
    assert!(health["data"].get("lens_copied").is_some());
    assert!(health["data"]["last_poll_age_ms"].is_null());

    handle.join.abort();
    tokio::time::sleep(Duration::from_millis(50)).await;
}
