//! HTTP parity gates for USER-extension browser tab and DOM operations.
//!
//! These tests use only the daemon's extension poll/result bridge.  They never
//! attach to a real browser, take a screenshot, or invoke an OS input path.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use reqwest::{Client, StatusCode};
use serde_json::{json, Value};
use tokio::sync::Mutex;
use vcu_core::{UserConfig, VcuPaths};

#[derive(Debug, Clone, PartialEq)]
struct CommandRecord {
    method: String,
    params: Value,
}

type CommandLog = Arc<Mutex<Vec<CommandRecord>>>;
type Responses = Arc<Mutex<HashMap<String, Value>>>;

async fn boot_async() -> (vcu_server::DaemonHandle, String, String, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let token = cfg.pairing_token.clone();
    let handle = vcu_server::start_daemon(paths, cfg).await.unwrap();
    let base = format!("http://{}", handle.addr);
    (handle, base, token, dir)
}

fn default_extension_result(method: &str) -> Value {
    match method {
        "list_tabs" => json!({"ok": true, "tabs": [], "groups": []}),
        "extract" => json!({"ok": true, "matches": [], "count": 0}),
        "type" => json!({"ok": true, "typed": false}),
        "scroll" => json!({"ok": true, "scrolled": false}),
        _ => json!({"ok": true}),
    }
}

/// Pair a fake USER extension and keep polling until the daemon sees it as live.
/// The worker records every command before returning the configured response.
async fn fake_extension(
    base: &str,
    token: &str,
    likely_user_profile: bool,
    responses: HashMap<String, Value>,
) -> (CommandLog, tokio::task::JoinHandle<()>) {
    let client = Client::new();
    let hello = client
        .post(format!("{base}/v1/extension/hello"))
        .header("X-Vcu-Token", token)
        .json(&json!({
            "likely_user_profile": likely_user_profile,
            "hosts": ["example.com"]
        }))
        .send()
        .await
        .unwrap();
    assert!(
        hello.status().is_success(),
        "extension hello status={}",
        hello.status()
    );

    let log: CommandLog = Arc::new(Mutex::new(Vec::new()));
    let response_map: Responses = Arc::new(Mutex::new(responses));
    let worker_log = Arc::clone(&log);
    let worker_responses = Arc::clone(&response_map);
    let worker_base = base.to_string();
    let worker_token = token.to_string();
    let worker = tokio::spawn(async move {
        let c = Client::new();
        loop {
            let polled = c
                .get(format!("{worker_base}/v1/extension/poll?wait_ms=100"))
                .header("X-Vcu-Token", &worker_token)
                .send()
                .await;
            let Ok(response) = polled else {
                tokio::time::sleep(Duration::from_millis(5)).await;
                continue;
            };
            let Ok(body) = response.json::<Value>().await else {
                continue;
            };
            let data = body.get("data").cloned().unwrap_or_else(|| json!({}));
            let id = data.get("id").and_then(Value::as_str).unwrap_or("");
            let method = data.get("method").and_then(Value::as_str).unwrap_or("");
            if id.is_empty() || method.is_empty() {
                continue;
            }
            let params = data.get("params").cloned().unwrap_or_else(|| json!({}));
            worker_log.lock().await.push(CommandRecord {
                method: method.to_string(),
                params,
            });
            let result = worker_responses
                .lock()
                .await
                .get(method)
                .cloned()
                .unwrap_or_else(|| default_extension_result(method));
            let _ = c
                .post(format!("{worker_base}/v1/extension/result"))
                .header("X-Vcu-Token", &worker_token)
                .json(&json!({"id": id, "result": result}))
                .send()
                .await;
        }
    });

    for _ in 0..100 {
        let health: Value = client
            .get(format!("{base}/v1/health"))
            .send()
            .await
            .unwrap()
            .json()
            .await
            .unwrap();
        if health["data"]["extension_polling"] == true {
            return (log, worker);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    worker.abort();
    panic!("fake extension did not reach polling state");
}

async fn get_json(client: &Client, url: String, token: Option<&str>) -> (StatusCode, Value) {
    let mut request = client.get(url);
    if let Some(token) = token {
        request = request.header("X-Vcu-Token", token);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    (status, response.json().await.unwrap())
}

async fn post_json(
    client: &Client,
    url: String,
    token: Option<&str>,
    body: Value,
) -> (StatusCode, Value) {
    let mut request = client.post(url).json(&body);
    if let Some(token) = token {
        request = request.header("X-Vcu-Token", token);
    }
    let response = request.send().await.unwrap();
    let status = response.status();
    (status, response.json().await.unwrap())
}

fn assert_error(body: &Value, code: &str) {
    assert_eq!(body["ok"], false, "{body}");
    assert_eq!(body["error"]["code"], code, "{body}");
    assert!(
        body.get("data").is_none() || body["data"].is_null(),
        "{body}"
    );
}

fn assert_extension_metadata(data: &Value, expected: &Value) {
    assert_eq!(data["source"], "extension_tabs", "{data}");
    assert_eq!(data["login_state"], true, "{data}");
    assert_eq!(data["hud"], false, "{data}");
    assert_eq!(data["os_cursor_used"], false, "{data}");
    for key in ["tabs", "groups", "group", "meta", "tab"] {
        if expected.get(key).is_some() {
            assert_eq!(
                data[key], expected[key],
                "metadata field {key} was changed: {data}"
            );
        }
    }
}

#[tokio::test]
async fn browser_tab_management_requires_pairing_token() {
    let (handle, base, _token, _dir) = boot_async().await;
    let client = Client::new();
    let cases = [
        ("GET", "/v1/browser/tabs", json!({})),
        ("POST", "/v1/browser/select", json!({"tab_id": "42"})),
        (
            "POST",
            "/v1/browser/group",
            json!({"tab_ids": ["42", "43"], "title": "Sprint"}),
        ),
        (
            "POST",
            "/v1/browser/group/update",
            json!({"group_id": "9", "title": "Sprint"}),
        ),
        ("POST", "/v1/browser/ungroup", json!({"tab_ids": ["42"]})),
        (
            "POST",
            "/v1/browser/open",
            json!({"url": "https://example.com/"}),
        ),
    ];

    for (method, path, body) in cases {
        let (status, value) = if method == "GET" {
            get_json(&client, format!("{base}{path}"), None).await
        } else {
            post_json(&client, format!("{base}{path}"), None, body).await
        };
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{method} {path}: {value}");
        assert_error(&value, "DaemonAuthFailed");
    }
    handle.join.abort();
}

#[tokio::test]
async fn browser_tab_management_requires_live_user_extension() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = Client::new();
    let cases = [
        ("GET", "/v1/browser/tabs", json!({})),
        ("POST", "/v1/browser/select", json!({"tab_id": "42"})),
        (
            "POST",
            "/v1/browser/group",
            json!({"tab_ids": ["42", "43"], "title": "Sprint"}),
        ),
        (
            "POST",
            "/v1/browser/group/update",
            json!({"group_id": "9", "title": "Sprint"}),
        ),
        ("POST", "/v1/browser/ungroup", json!({"tab_ids": ["42"]})),
        (
            "POST",
            "/v1/browser/open",
            json!({"url": "https://example.com/"}),
        ),
    ];

    for (method, path, body) in cases {
        let (status, value) = if method == "GET" {
            get_json(&client, format!("{base}{path}"), Some(&token)).await
        } else {
            post_json(&client, format!("{base}{path}"), Some(&token), body).await
        };
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {path}: {value}");
        assert_error(&value, "ExtensionDisconnected");
    }
    handle.join.abort();
}

#[tokio::test]
async fn browser_tab_management_rejects_agent_profile() {
    let (handle, base, token, _dir) = boot_async().await;
    let (log, worker) = fake_extension(&base, &token, false, HashMap::new()).await;
    let client = Client::new();
    let cases = [
        ("GET", "/v1/browser/tabs", json!({})),
        ("POST", "/v1/browser/select", json!({"tab_id": "42"})),
        (
            "POST",
            "/v1/browser/group",
            json!({"tab_ids": ["42", "43"], "title": "Sprint"}),
        ),
        (
            "POST",
            "/v1/browser/group/update",
            json!({"group_id": "9", "title": "Sprint"}),
        ),
        ("POST", "/v1/browser/ungroup", json!({"tab_ids": ["42"]})),
        (
            "POST",
            "/v1/browser/open",
            json!({"url": "https://example.com/"}),
        ),
    ];

    for (method, path, body) in cases {
        let (status, value) = if method == "GET" {
            get_json(&client, format!("{base}{path}"), Some(&token)).await
        } else {
            post_json(&client, format!("{base}{path}"), Some(&token), body).await
        };
        assert_eq!(status, StatusCode::BAD_REQUEST, "{method} {path}: {value}");
        assert_error(&value, "ActionFailed");
    }
    assert!(
        log.lock().await.is_empty(),
        "agent profile must not receive commands"
    );
    worker.abort();
    handle.join.abort();
}

#[tokio::test]
async fn browser_tab_management_rejects_invalid_shapes_before_bridge() {
    let (handle, base, token, _dir) = boot_async().await;
    let client = Client::new();
    let cases = [
        ("/v1/browser/select", json!({"tab_id": ""})),
        ("/v1/browser/close", json!({})),
        ("/v1/browser/close", json!({"tab_id": ""})),
        (
            "/v1/browser/group",
            json!({"tab_ids": [], "title": "Sprint"}),
        ),
        (
            "/v1/browser/group",
            json!({"tab_ids": ["42"], "title": "  "}),
        ),
        (
            "/v1/browser/group",
            json!({"tab_ids": ["42"], "title": "Sprint", "color": "lavender"}),
        ),
        ("/v1/browser/group/update", json!({"group_id": ""})),
        (
            "/v1/browser/open",
            json!({
                "url": "https://example.com/",
                "session_name": "Sprint",
                "group_id": "9"
            }),
        ),
        (
            "/v1/browser/open",
            json!({"url": "https://example.com/", "active": "yes"}),
        ),
        (
            "/v1/browser/open",
            json!({"url": "https://example.com/", "new_window": true, "group_id": "9"}),
        ),
        (
            "/v1/browser/open",
            json!({"url": "https://example.com/", "new_window": "yes"}),
        ),
        (
            "/v1/browser/open",
            json!({"url": "file:///tmp/private.html"}),
        ),
    ];

    for (path, body) in cases {
        let (status, value) = post_json(&client, format!("{base}{path}"), Some(&token), body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{path}: {value}");
        assert_error(&value, "InvalidInput");
    }
    handle.join.abort();
}

#[tokio::test]
async fn browser_tab_management_preserves_extension_metadata_and_params() {
    let (handle, base, token, _dir) = boot_async().await;
    let list = json!({
        "ok": true,
        "source": "extension-provided",
        "tabs": [{
            "tab_id": "42",
            "window_id": "7",
            "title": "Example",
            "url": "https://example.com/",
            "active": true,
            "focused": true,
            "group_id": "9"
        }],
        "groups": [{
            "group_id": "9",
            "window_id": "7",
            "title": "Sprint",
            "color": "purple",
            "collapsed": false
        }],
        "meta": {"focused_tab_id": "42", "last_focused_window_id": "7"}
    });
    let select = json!({
        "ok": true,
        "tab": {"tab_id": "42", "window_id": "7", "active": true, "focused": true},
        "group": {"group_id": "9", "title": "Sprint", "collapsed": false},
        "meta": {"expanded": true, "selected": "42"}
    });
    let group = json!({
        "ok": true,
        "tabs": [{"tab_id": "42"}, {"tab_id": "43"}],
        "groups": [{"group_id": "10", "window_id": "7", "title": "Sprint"}],
        "group": {"group_id": "10", "window_id": "7", "title": "Sprint", "color": "purple", "collapsed": true},
        "meta": {"same_window": true}
    });
    let update = json!({
        "ok": true,
        "group": {"group_id": "10", "window_id": "7", "title": "Renamed", "color": "blue", "collapsed": false},
        "meta": {"updated": ["title", "color", "collapsed"]}
    });
    let ungroup = json!({
        "ok": true,
        "tabs": [{"tab_id": "42", "group_id": null}, {"tab_id": "43", "group_id": null}],
        "group": null,
        "meta": {"ungrouped": true}
    });
    let open = json!({
        "ok": true,
        "tab": {"tab_id": "44", "window_id": "7", "url": "https://example.com/", "active": false},
        "group": {"group_id": "10", "title": "Sprint"},
        "meta": {"session_name": "Sprint", "group_created": false}
    });
    let mut responses = HashMap::new();
    responses.insert("list_tabs".into(), list.clone());
    responses.insert("select_tab".into(), select.clone());
    responses.insert("group_tabs".into(), group.clone());
    responses.insert("update_group".into(), update.clone());
    responses.insert("ungroup_tabs".into(), ungroup.clone());
    responses.insert("open_tab".into(), open.clone());
    let (log, worker) = fake_extension(&base, &token, true, responses).await;
    let client = Client::new();

    let (status, value) = get_json(&client, format!("{base}/v1/browser/tabs"), Some(&token)).await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["ok"], true, "{value}");
    assert_extension_metadata(&value["data"], &list);

    let calls = [
        (
            "/v1/browser/select",
            json!({"tab_id": "42"}),
            &select,
            "select_tab",
        ),
        (
            "/v1/browser/group",
            json!({"tab_ids": ["42", "43"], "title": "Sprint", "color": "purple", "collapsed": true}),
            &group,
            "group_tabs",
        ),
        (
            "/v1/browser/group/update",
            json!({"group_id": "10", "title": "Renamed", "color": "blue", "collapsed": false}),
            &update,
            "update_group",
        ),
        (
            "/v1/browser/ungroup",
            json!({"tab_ids": ["42", "43"]}),
            &ungroup,
            "ungroup_tabs",
        ),
        (
            "/v1/browser/open",
            json!({"url": "https://example.com/", "session_name": "Sprint", "active": false}),
            &open,
            "open_tab",
        ),
    ];
    for (path, request_body, expected, method) in calls {
        let (status, value) =
            post_json(&client, format!("{base}{path}"), Some(&token), request_body).await;
        assert_eq!(status, StatusCode::OK, "{method}: {value}");
        assert_eq!(value["ok"], true, "{method}: {value}");
        assert_extension_metadata(&value["data"], expected);
    }

    let commands = log.lock().await.clone();
    assert_eq!(commands.len(), 7, "{commands:?}");
    assert_eq!(commands[0].method, "list_tabs");
    assert_eq!(commands[0].params, json!({}));
    assert_eq!(commands[1].method, "list_tabs");
    assert_eq!(commands[1].params, json!({}));
    assert_eq!(commands[2].method, "select_tab");
    assert_eq!(commands[2].params, json!({"tab_id": "42"}));
    assert_eq!(commands[3].method, "group_tabs");
    assert_eq!(
        commands[3].params,
        json!({"tab_ids": ["42", "43"], "title": "Sprint", "color": "purple", "collapsed": true})
    );
    assert_eq!(commands[4].method, "update_group");
    assert_eq!(
        commands[4].params,
        json!({"group_id": "10", "title": "Renamed", "color": "blue", "collapsed": false})
    );
    assert_eq!(commands[5].method, "ungroup_tabs");
    assert_eq!(commands[5].params, json!({"tab_ids": ["42", "43"]}));
    assert_eq!(commands[6].method, "open_tab");
    assert_eq!(
        commands[6].params,
        json!({"url": "https://example.com/", "session_name": "Sprint", "active": false})
    );

    worker.abort();
    handle.join.abort();
}

#[tokio::test]
async fn browser_tab_management_does_not_forge_success_on_extension_error() {
    let (handle, base, token, _dir) = boot_async().await;
    let mut responses = HashMap::new();
    responses.insert(
        "list_tabs".into(),
        json!({"ok": true, "tabs": [{"tab_id": "42"}], "groups": []}),
    );
    responses.insert(
        "select_tab".into(),
        json!({
            "ok": false,
            "error": "tabs.update rejected",
            "detail": {"reason": "window mismatch"}
        }),
    );
    let (log, worker) = fake_extension(&base, &token, true, responses).await;
    let client = Client::new();
    let (status, value) = post_json(
        &client,
        format!("{base}/v1/browser/select"),
        Some(&token),
        json!({"tab_id": "42"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{value}");
    assert_error(&value, "ActionFailed");
    assert!(value["error"]["detail"]
        .as_str()
        .unwrap_or("")
        .contains("tabs.update rejected"));
    assert!(value["data"].is_null());
    assert_eq!(
        log.lock().await.as_slice(),
        &[
            CommandRecord {
                method: "list_tabs".into(),
                params: json!({}),
            },
            CommandRecord {
                method: "select_tab".into(),
                params: json!({"tab_id": "42"}),
            },
        ]
    );
    worker.abort();
    handle.join.abort();
}

#[tokio::test]
async fn dom_type_and_scroll_forward_explicit_tab_and_dry_run() {
    let (handle, base, token, _dir) = boot_async().await;
    let mut responses = HashMap::new();
    responses.insert(
        "list_tabs".into(),
        json!({"ok": true, "tabs": [{"tab_id": "77"}], "groups": []}),
    );
    responses.insert(
        "type".into(),
        json!({
            "ok": true,
            "typed": true,
            "tab_id": "77",
            "page_url": "https://example.com/form",
            "focused": false
        }),
    );
    responses.insert(
        "scroll".into(),
        json!({
            "ok": true,
            "scrolled": true,
            "tab_id": "77",
            "page_url": "https://example.com/form",
            "focused": false
        }),
    );
    let (log, worker) = fake_extension(&base, &token, true, responses).await;
    let client = Client::new();

    let (status, typed) = post_json(
        &client,
        format!("{base}/v1/browser/type"),
        Some(&token),
        json!({
            "selector": "input[name=q]",
            "text": "hello",
            "tab_id": "77",
            "dry_run": true
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{typed}");
    assert_eq!(typed["ok"], true, "{typed}");
    assert_eq!(typed["data"]["source"], "extension_dom");
    assert_eq!(typed["data"]["typed"], true);
    assert_eq!(typed["data"]["tab_id"], "77");

    let (status, scrolled) = post_json(
        &client,
        format!("{base}/v1/browser/scroll"),
        Some(&token),
        json!({"dy": -320, "tab_id": "77", "dry_run": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{scrolled}");
    assert_eq!(scrolled["ok"], true, "{scrolled}");
    assert_eq!(scrolled["data"]["source"], "extension_dom");
    assert_eq!(scrolled["data"]["scrolled"], true);
    assert_eq!(scrolled["data"]["tab_id"], "77");

    let commands = log.lock().await.clone();
    assert_eq!(commands.len(), 4, "{commands:?}");
    assert_eq!(commands[0].method, "list_tabs");
    assert_eq!(commands[1].method, "type");
    assert_eq!(
        commands[1].params,
        json!({"selector": "input[name=q]", "text": "hello", "dry_run": true, "tab_id": "77"})
    );
    assert_eq!(commands[2].method, "list_tabs");
    assert_eq!(commands[3].method, "scroll");
    assert_eq!(
        commands[3].params,
        json!({"dy": -320, "dry_run": true, "tab_id": "77"})
    );

    worker.abort();
    handle.join.abort();
}

#[tokio::test]
async fn dom_scroll_extension_failure_is_returned_without_ax_fallback() {
    let (handle, base, token, _dir) = boot_async().await;
    let mut responses = HashMap::new();
    responses.insert(
        "list_tabs".into(),
        json!({"ok": true, "tabs": [{"tab_id": "77"}], "groups": []}),
    );
    responses.insert(
        "scroll".into(),
        json!({"ok": false, "error": "content script timeout", "via": "extension_dom"}),
    );
    let (log, worker) = fake_extension(&base, &token, true, responses).await;
    let client = Client::new();
    let (status, value) = post_json(
        &client,
        format!("{base}/v1/browser/scroll"),
        Some(&token),
        json!({"dy": 600, "tab_id": "77", "dry_run": false}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{value}");
    assert_error(&value, "ActionFailed");
    assert!(value["error"]["detail"]
        .as_str()
        .unwrap_or("")
        .contains("content script timeout"));
    assert_eq!(
        log.lock().await.as_slice(),
        &[
            CommandRecord {
                method: "list_tabs".into(),
                params: json!({}),
            },
            CommandRecord {
                method: "scroll".into(),
                params: json!({"dy": 600, "dry_run": false, "tab_id": "77"}),
            },
        ]
    );
    worker.abort();
    handle.join.abort();
}

#[tokio::test]
async fn extract_without_tab_forwards_directly_and_lets_extension_resolve_focus() {
    let (handle, base, token, _dir) = boot_async().await;
    let mut responses = HashMap::new();
    responses.insert(
        "extract".into(),
        json!({
            "ok": true,
            "tab_id": "88",
            "page_url": "https://focused.example/",
            "focused": true,
            "matches": [{"tag": "A", "text": "focused"}],
            "count": 1,
            "truncated": false
        }),
    );
    let (log, worker) = fake_extension(&base, &token, true, responses).await;
    let client = Client::new();
    let (status, value) = post_json(
        &client,
        format!("{base}/v1/browser/extract"),
        Some(&token),
        json!({"selector": "a"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{value}");
    assert_eq!(value["ok"], true, "{value}");
    assert_eq!(value["data"]["source"], "extension_dom");
    assert_eq!(value["data"]["tab_id"], "88");
    assert_eq!(value["data"]["focused"], true);
    let commands = log.lock().await.clone();
    assert_eq!(
        commands.as_slice(),
        &[CommandRecord {
            method: "extract".into(),
            params: json!({"selector": "a"}),
        }]
    );

    worker.abort();
    handle.join.abort();
}

#[tokio::test]
async fn viewport_screenshot_binds_pixels_to_one_tab_and_is_consumed_once() {
    let (handle, base, token, dir) = boot_async().await;
    let viewport = json!({"document_id":"doc-1","url":"https://example.com/","width":100,"height":80,"scroll_x":0,"scroll_y":0,"revision":1});
    let (log, worker) = fake_extension(&base, &token, true, HashMap::from([
        ("list_tabs".into(), json!({"ok":true,"tabs":[{"tab_id":"7"}],"groups":[]})),
        ("capture_tab".into(), json!({"ok":true,"tab_id":"7","viewport":viewport,"png_base64":"iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aD1kAAAAASUVORK5CYII="})),
        ("click_point".into(), json!({"ok":true,"pressed":true,"source":"extension_dom","trusted":false})),
    ])).await;
    let c = Client::new();
    let (_, shot) = post_json(&c, format!("{base}/v1/browser/screenshot"), Some(&token), json!({"tab_id":"7"})).await;
    assert_eq!(shot["ok"], true, "{shot}");
    let capture = shot["data"]["capture_id"].as_str().unwrap();
    assert!(std::path::Path::new(shot["data"]["screenshot_path"].as_str().unwrap()).is_file());
    assert!(shot["data"].get("png_base64").is_none());
    let mut click = json!({"space":"viewport","capture_id":capture,"pixel_x":0.5,"pixel_y":0.25,"tab_id":"8","dry_run":true});
    let (_, wrong) = post_json(&c, format!("{base}/v1/browser/click"), Some(&token), click.clone()).await;
    assert_error(&wrong, "InvalidInput");
    click["tab_id"] = json!("7");
    let (_, dry) = post_json(&c, format!("{base}/v1/browser/click"), Some(&token), click.clone()).await;
    assert_eq!(dry["ok"], true, "{dry}");
    let record = log.lock().await.last().unwrap().clone();
    assert_eq!(record.method, "click_point");
    assert_eq!(record.params["x"], 50.0);
    assert_eq!(record.params["y"], 20.0);
    assert_eq!(record.params["expected_viewport"], viewport);
    click["dry_run"] = json!(false);
    let (_, acted) = post_json(&c, format!("{base}/v1/browser/click"), Some(&token), click.clone()).await;
    assert_eq!(acted["ok"], true, "{acted}");
    let count = log.lock().await.len();
    let (_, replay) = post_json(&c, format!("{base}/v1/browser/click"), Some(&token), click).await;
    assert_error(&replay, "InvalidInput");
    assert_eq!(log.lock().await.len(), count, "consumed capture was replayed");
    // An expired capture fails before the extension sees another action.
    let (_, fresh) = post_json(&c, format!("{base}/v1/browser/screenshot"), Some(&token), json!({"tab_id":"7"})).await;
    let fresh_id = fresh["data"]["capture_id"].as_str().unwrap();
    let path = VcuPaths::from_root(dir.path()).captures_dir().join(format!("viewport-{fresh_id}.json"));
    let mut meta: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    meta["created_at_ms"] = json!(0);
    std::fs::write(path, serde_json::to_vec(&meta).unwrap()).unwrap();
    let (_, expired) = post_json(&c, format!("{base}/v1/browser/click"), Some(&token), json!({"space":"viewport","capture_id":fresh_id,"pixel_x":0,"pixel_y":0})).await;
    assert_error(&expired, "InvalidInput");
    worker.abort(); handle.join.abort();
}

#[tokio::test]
async fn extension_can_return_a_retina_png_larger_than_two_megabytes() {
    let (handle, base, token, _dir) = boot_async().await;
    let c = Client::new();
    let result = json!({"id":"transport-test","result":{"ok":true,"png_base64":"A".repeat(2_500_000)}});
    let response = c.post(format!("{base}/v1/extension/result")).header("X-Vcu-Token",token).json(&result).send().await.unwrap();
    assert!(response.status().is_success(), "large screenshot transport status {}", response.status());
    handle.join.abort();
}
