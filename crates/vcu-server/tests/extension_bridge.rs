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
    for method in [
        "click",
        "type",
        "scroll",
        "open_tab",
        "group_tabs",
        "select_tab",
        "update_group",
        "ungroup_tabs",
    ] {
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
        .call_timeout(
            "extract",
            json!({"tab_id": "chrome-tab", "selector": "#result"}),
            3,
        )
        .await
        .unwrap();
    assert_eq!(result["ok"], true);
    worker.await.unwrap();
}

#[tokio::test]
async fn list_tabs_merged_from_two_clients() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge
        .mark_hello_client(true, Some("edge".into()), Some("edge".into()))
        .await;
    bridge
        .mark_hello_client(true, Some("chrome".into()), Some("chrome".into()))
        .await;
    let edge = bridge.clone();
    let chrome = bridge.clone();
    let edge_w = tokio::spawn(async move {
        let cmd = edge
            .poll_for(2000, Some("edge".into()), Some("edge".into()))
            .await
            .expect("edge list");
        assert_eq!(cmd.method, "list_tabs");
        edge.submit_result(
            &cmd.id,
            json!({"ok": true, "tabs": [{"tab_id": "e1", "window_id": "ew", "title": "Edge", "url": "https://e.example/", "agent_owned": false, "borrowed_by": null}], "groups": []}),
        )
        .await;
    });
    let chrome_w = tokio::spawn(async move {
        let cmd = chrome
            .poll_for(2000, Some("chrome".into()), Some("chrome".into()))
            .await
            .expect("chrome list");
        assert_eq!(cmd.method, "list_tabs");
        chrome
            .submit_result(
                &cmd.id,
                json!({"ok": true, "tabs": [{"tab_id": "c1", "window_id": "cw", "title": "Chrome", "url": "https://c.example/", "agent_owned": false, "borrowed_by": null}], "groups": []}),
            )
            .await;
    });
    let merged = bridge.list_tabs_merged().await.unwrap();
    let tabs = merged["tabs"].as_array().unwrap();
    assert_eq!(tabs.len(), 2);
    let ids: Vec<&str> = tabs.iter().map(|t| t["tab_id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"e1"));
    assert!(ids.contains(&"c1"));
    assert_eq!(
        tabs.iter().find(|t| t["tab_id"] == "e1").unwrap()["browser"],
        "edge"
    );
    assert_eq!(
        tabs.iter().find(|t| t["tab_id"] == "c1").unwrap()["browser"],
        "chrome"
    );
    assert_eq!(merged["browser_count"], 2);
    let names = merged["browsers"].as_array().unwrap();
    assert!(names.iter().any(|v| v == "edge"));
    assert!(names.iter().any(|v| v == "chrome"));
    edge_w.await.unwrap();
    chrome_w.await.unwrap();
}

#[tokio::test]
async fn list_tabs_single_client_still_reports_browser_count() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge
        .mark_hello_client(true, Some("edge".into()), Some("edge".into()))
        .await;
    let edge = bridge.clone();
    let worker = tokio::spawn(async move {
        let cmd = edge.poll_for(2000, Some("edge".into()), Some("edge".into())).await.expect("list");
        assert_eq!(cmd.method, "list_tabs");
        edge.submit_result(
            &cmd.id,
            json!({"ok": true, "tabs": [{"tab_id": "e1", "window_id": "ew", "title": "Edge", "url": "https://e.example/", "agent_owned": false, "borrowed_by": null}], "groups": []}),
        )
        .await;
    });
    let merged = bridge.list_tabs_merged().await.unwrap();
    assert_eq!(merged["browser_count"], 1);
    assert_eq!(merged["browsers"][0], "edge");
    assert_eq!(merged["tabs"].as_array().unwrap().len(), 1);
    worker.await.unwrap();
}

#[tokio::test]
async fn action_with_known_tab_id_targets_owning_client() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge
        .mark_hello_client(true, Some("edge".into()), Some("edge".into()))
        .await;
    bridge
        .mark_hello_client(true, Some("chrome".into()), Some("chrome".into()))
        .await;
    let edge = bridge.clone();
    let chrome = bridge.clone();
    let edge_list = tokio::spawn(async move {
        let cmd = edge.poll_for(2000, Some("edge".into()), Some("edge".into())).await.expect("edge list");
        edge.submit_result(
            &cmd.id,
            json!({"ok": true, "tabs": [{"tab_id": "e1", "window_id": "ew", "title": "Edge", "url": "https://e.example/", "agent_owned": false, "borrowed_by": null}], "groups": []}),
        )
        .await;
    });
    let chrome_list = tokio::spawn(async move {
        let cmd = chrome
            .poll_for(2000, Some("chrome".into()), Some("chrome".into()))
            .await
            .expect("chrome list");
        chrome
            .submit_result(
                &cmd.id,
                json!({"ok": true, "tabs": [{"tab_id": "c1", "window_id": "cw", "title": "Chrome", "url": "https://c.example/", "agent_owned": false, "borrowed_by": null}], "groups": []}),
            )
            .await;
    });
    let merged = bridge.list_tabs_merged().await.unwrap();
    assert_eq!(merged["tabs"].as_array().unwrap().len(), 2);
    edge_list.await.unwrap();
    chrome_list.await.unwrap();

    let edge = bridge.clone();
    let chrome = bridge.clone();
    let chrome_w = tokio::spawn(async move {
        let cmd = chrome
            .poll_for(2000, Some("chrome".into()), Some("chrome".into()))
            .await
            .expect("chrome extract");
        assert_eq!(cmd.method, "extract");
        chrome
            .submit_result(&cmd.id, json!({"ok": true, "source": "extension_dom", "owner": "chrome"}))
            .await;
    });
    let edge_w = tokio::spawn(async move {
        let stolen = tokio::time::timeout(
            std::time::Duration::from_millis(300),
            edge.poll_for(200, Some("edge".into()), Some("edge".into())),
        )
        .await
        .ok()
        .flatten();
        assert!(stolen.is_none(), "edge must not receive a chrome-owned tab command");
    });
    let result = bridge
        .call_timeout("extract", json!({"tab_id": "c1", "selector": "#result"}), 3)
        .await
        .unwrap();
    assert_eq!(result["owner"], "chrome");
    chrome_w.await.unwrap();
    edge_w.await.unwrap();
}

#[tokio::test]
async fn reload_all_clients_targets_each_connected_browser() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge
        .mark_hello_client(true, Some("edge".into()), Some("edge".into()))
        .await;
    bridge
        .mark_hello_client(true, Some("chrome".into()), Some("chrome".into()))
        .await;
    let edge = bridge.clone();
    let chrome = bridge.clone();
    let edge_w = tokio::spawn(async move {
        let cmd = edge.poll_for(2000, Some("edge".into()), Some("edge".into())).await.expect("edge reload");
        assert_eq!(cmd.method, "reload_self");
        edge.submit_result(&cmd.id, json!({"ok": true, "reloading": true, "browser": "edge"}))
            .await;
    });
    let chrome_w = tokio::spawn(async move {
        let cmd = chrome
            .poll_for(2000, Some("chrome".into()), Some("chrome".into()))
            .await
            .expect("chrome reload");
        assert_eq!(cmd.method, "reload_self");
        chrome
            .submit_result(&cmd.id, json!({"ok": true, "reloading": true, "browser": "chrome"}))
            .await;
    });
    let result = bridge.reload_all_clients().await.unwrap();
    assert_eq!(result["reloaded"], 2);
    edge_w.await.unwrap();
    chrome_w.await.unwrap();
}

#[tokio::test]
async fn reload_without_client_ids_enqueues_two_untargeted_commands() {
    let bridge = ExtensionBridge::with_lease_ms(80);
    bridge.mark_hello(true).await;
    let a = bridge.clone();
    let b = bridge.clone();
    let wa = tokio::spawn(async move {
        let cmd = a.poll(2000).await.expect("first");
        assert_eq!(cmd.method, "reload_self");
        a.submit_result(&cmd.id, json!({"ok": true, "reloading": true, "n": 1}))
            .await;
    });
    let wb = tokio::spawn(async move {
        let cmd = b.poll(2000).await.expect("second");
        assert_eq!(cmd.method, "reload_self");
        b.submit_result(&cmd.id, json!({"ok": true, "reloading": true, "n": 2}))
            .await;
    });
    let result = bridge.reload_all_clients().await.unwrap();
    assert_eq!(result["reloaded"], 2);
    wa.await.unwrap();
    wb.await.unwrap();
}

#[tokio::test]
async fn same_runtime_id_two_browsers_are_two_clients() {
    let bridge = ExtensionBridge::new();
    let id = "cedlbclnijpladccmmfpihhgkeeldfhc";
    bridge
        .mark_hello_client(true, Some(id.into()), Some("edge".into()))
        .await;
    bridge
        .mark_hello_client(true, Some(id.into()), Some("chrome".into()))
        .await;
    let mut names = bridge.active_browsers().await;
    names.sort();
    assert_eq!(names, vec!["chrome".to_string(), "edge".to_string()]);
    assert_eq!(bridge.active_client_ids().await.len(), 2);
}

#[tokio::test]
async fn poll_without_browser_does_not_register_placeholder() {
    let bridge = ExtensionBridge::new();
    let id = "cedlbclnijpladccmmfpihhgkeeldfhc";
    bridge
        .mark_hello_client(true, Some(id.into()), Some("edge".into()))
        .await;
    bridge
        .mark_hello_client(true, Some(id.into()), Some("chrome".into()))
        .await;
    let _ = bridge.poll_for(20, Some(id.into()), None).await;
    let mut names = bridge.active_browsers().await;
    names.sort();
    assert_eq!(names, vec!["chrome".to_string(), "edge".to_string()]);
    assert_eq!(bridge.active_client_ids().await.len(), 2);
    assert!(!names.iter().any(|n| n == "browser"));
}
