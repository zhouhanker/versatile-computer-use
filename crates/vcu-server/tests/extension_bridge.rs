use serde_json::json;
use vcu_core::{ActionRequest, ErrorCode};
use vcu_server::browser::extension::{ExtensionBackend, ExtensionBridge};
use vcu_server::browser::BrowserBackend;

#[tokio::test]
async fn extension_bridge_roundtrip() {
    let bridge = ExtensionBridge::new();
    bridge.mark_hello().await;
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
