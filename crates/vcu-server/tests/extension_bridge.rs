use serde_json::json;
use vcu_core::{ActionRequest, ErrorCode};
use vcu_server::browser::extension::{ExtensionBackend, ExtensionBridge};
use vcu_server::browser::BrowserBackend;

#[tokio::test]
async fn extension_bridge_roundtrip() {
    let bridge = ExtensionBridge::new();
    bridge.mark_hello(false).await;
    let bridge2 = bridge.clone();
    let worker = tokio::spawn(async move {
        let cmd = bridge2.poll(5000).await.expect("cmd");
        assert_eq!(cmd.method, "list_tabs");
        bridge2
            .submit_result(
                &cmd.id,
                json!({
                    "ok": true,
                    "tabs": [{
                        "tab_id": "1",
                        "window_id": "w",
                        "title": "t",
                        "url": "https://example.com",
                        "agent_owned": true,
                        "borrowed_by": null
                    }]
                }),
            )
            .await;
    });
    let mut backend = ExtensionBackend::with_bridge(bridge);
    let tabs = backend.list_tabs().await.unwrap();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].tab_id, "1");
    worker.await.unwrap();

    let err = backend
        .act(
            "1",
            &ActionRequest {
                r#type: "os_cursor_move".into(),
                target: json!({}),
                args: json!({}),
                idempotency_key: None,
            },
        )
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::OsCursorDenied);
}

#[tokio::test]
async fn likely_user_profile_sticky() {
    let bridge = ExtensionBridge::new();
    assert!(!bridge.likely_user_profile().await);
    bridge.mark_hello(false).await;
    assert!(!bridge.likely_user_profile().await);
    bridge.mark_hello(true).await;
    assert!(bridge.likely_user_profile().await);
    bridge.mark_hello(false).await;
    assert!(bridge.likely_user_profile().await);
}

#[tokio::test]
async fn leased_command_is_retried_until_result() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge.mark_hello(true).await;
    let b2 = bridge.clone();
    let worker = tokio::spawn(async move {
        let first = b2.poll(1000).await.expect("first lease");
        assert_eq!(first.method, "list_tabs");
        let second = b2.poll(1000).await.expect("re-lease after expiry");
        assert_eq!(second.id, first.id);
        b2.submit_result(&second.id, json!({"ok": true, "tabs": []}))
            .await;
    });
    let backend = ExtensionBackend::with_bridge(bridge);
    let tabs = backend.list_tabs().await.unwrap();
    assert!(tabs.is_empty());
    worker.await.unwrap();
}


#[tokio::test]
async fn call_timeout_without_result_is_error() {
    let bridge = ExtensionBridge::with_lease_ms(50);
    bridge.mark_hello(true).await;
    let err = bridge
        .call_timeout("extract", json!({"selector": "a"}), 1)
        .await
        .unwrap_err();
    assert_eq!(err.code(), ErrorCode::ActionFailed);
    assert!(err.message().contains("timeout"), "{}", err.message());
}

#[tokio::test]
async fn leased_mutation_is_never_replayed() {
    for method in ["click", "type", "scroll", "open_tab", "group_tabs", "select_tab", "update_group", "ungroup_tabs"] {
        let bridge = ExtensionBridge::with_lease_ms(10);
        bridge.mark_hello(true).await;
        let caller = bridge.clone();
        let task = tokio::spawn(async move { caller.call_timeout(method, json!({}), 2).await });
        let first = bridge.poll(500).await.expect("first delivery");
        assert!(bridge.poll(100).await.is_none(), "replayed {method}");
        bridge.submit_result(&first.id, json!({"ok":true})).await;
        assert!(task.await.unwrap().is_ok());
    }
}

#[tokio::test]
async fn malformed_extension_reply_is_not_success() {
    let bridge = ExtensionBridge::new();
    bridge.mark_hello(true).await;
    let caller = bridge.clone();
    let task = tokio::spawn(async move { caller.call_timeout("click", json!({}), 2).await });
    let cmd = bridge.poll(500).await.unwrap();
    bridge.submit_result(&cmd.id, json!({"pressed":true})).await;
    assert!(task.await.unwrap().is_err());
}

#[tokio::test]
async fn retryable_wrong_browser_is_handed_to_next_poller() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge.mark_hello(true).await;
    let b2 = bridge.clone();
    let worker = tokio::spawn(async move {
        let first = b2.poll(1000).await.expect("edge poller");
        b2.submit_result(
            &first.id,
            json!({"ok": false, "error": "wrong_extension_browser", "retryable": true}),
        )
        .await;
        let second = b2.poll(1000).await.expect("chrome poller");
        assert_eq!(second.id, first.id);
        b2.submit_result(&second.id, json!({"ok": true, "source": "extension_dom"}))
            .await;
    });
    let result = bridge
        .call_timeout("extract", json!({"tab_id": "chrome-tab", "selector": "#result"}), 3)
        .await
        .unwrap();
    assert_eq!(result["ok"], true);
    worker.await.unwrap();
}
