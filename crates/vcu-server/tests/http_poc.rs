use std::time::Duration;

use serde_json::json;
use vcu_core::{UserConfig, VcuPaths};

#[tokio::test]
async fn end_to_end_mock_session_boundaries() {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths.clone(), cfg).await.unwrap();
    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();

    let auth = |req: reqwest::RequestBuilder| req.header("X-Vcu-Token", &token);

    // bad auth
    let resp = client
        .post(format!("{base}/v1/session/start"))
        .json(&json!({"backend":"mock"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    let resp = auth(client.post(format!("{base}/v1/session/start")))
        .json(&json!({"backend":"mock","browser":"mock"}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], true);
    assert_eq!(body["data"]["policy"]["os_cursor"], "deny");
    let sid = body["data"]["session_id"].as_str().unwrap().to_string();

    let resp = auth(client.post(format!("{base}/v1/session/{sid}/navigate")))
        .json(&json!({"url":"https://example.com/"}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    let resp = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"full","budget":2000}))
        .send()
        .await
        .unwrap();
    let snap: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(snap["ok"], true);
    assert!(snap["data"]["dom_refs"].as_array().unwrap().len() >= 3);

    let resp = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&json!({"type":"os_click","target":{},"args":{}}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);
    let err: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(err["error"]["code"], "OsCursorDenied");

    let tabs: serde_json::Value = auth(client.get(format!("{base}/v1/session/{sid}/tabs")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let user_tab = tabs["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["agent_owned"] == false)
        .unwrap()["tab_id"]
        .as_str()
        .unwrap()
        .to_string();

    let resp = auth(client.post(format!("{base}/v1/session/{sid}/navigate")))
        .json(&json!({"url":"https://x","tab_id": user_tab}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 409);

    let resp = auth(client.post(format!("{base}/v1/session/{sid}/tabs/borrow")))
        .json(&json!({"tab_id": user_tab}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    let resp = auth(client.post(format!("{base}/v1/session/{sid}/navigate")))
        .json(&json!({"url":"https://mail.example.com/y","tab_id": user_tab}))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());

    // idempotency
    let act = json!({"type":"click","target":{"ref":"e1"},"idempotency_key":"k1"});
    // set active back by navigating agent tab first
    let agent_tab = tabs["data"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["agent_owned"] == true)
        .unwrap()["tab_id"]
        .as_str()
        .unwrap()
        .to_string();
    let _ = auth(client.post(format!("{base}/v1/session/{sid}/navigate")))
        .json(&json!({"url":"https://example.com/","tab_id": agent_tab}))
        .send()
        .await
        .unwrap();

    let r1: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&act)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let r2: serde_json::Value = auth(client.post(format!("{base}/v1/session/{sid}/act")))
        .json(&act)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r1["data"]["action_id"], r2["data"]["action_id"]);

    // budget truncation extreme
    let resp = auth(client.post(format!("{base}/v1/session/{sid}/snapshot")))
        .json(&json!({"mode":"full","budget":1}))
        .send()
        .await
        .unwrap();
    let snap: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(snap["ok"], true);
    assert_eq!(snap["data"]["truncated"], true);

    let _ = auth(client.post(format!("{base}/v1/session/{sid}/stop")))
        .json(&json!({}))
        .send()
        .await
        .unwrap();

    // drop handle by aborting join - process end is fine
    handle.join.abort();
    tokio::time::sleep(Duration::from_millis(50)).await;
}
