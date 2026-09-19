//! MCP parity gates for USER-extension browser tab/group and DOM operations.
//!
//! The daemon side is backed by a fake extension poll worker.  No real browser
//! is opened and no OS input or screenshot path is exercised.

use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Duration;

use reqwest::Client;
use serde_json::{json, Value};
use tokio::sync::Mutex;
use vcu_core::{UserConfig, VcuPaths};

#[derive(Debug, Clone, PartialEq)]
struct CommandRecord {
    method: String,
    params: Value,
}

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

async fn fake_extension(
    base: &str,
    token: &str,
) -> (Arc<Mutex<Vec<CommandRecord>>>, tokio::task::JoinHandle<()>) {
    let client = Client::new();
    let hello = client
        .post(format!("{base}/v1/extension/hello"))
        .header("X-Vcu-Token", token)
        .json(&json!({
            "likely_user_profile": true,
            "hosts": ["example.com"]
        }))
        .send()
        .await
        .unwrap();
    assert!(
        hello.status().is_success(),
        "extension hello failed: {}",
        hello.status()
    );

    let log = Arc::new(Mutex::new(Vec::new()));
    let worker_log = Arc::clone(&log);
    let worker_base = base.to_string();
    let worker_token = token.to_string();
    let worker = tokio::spawn(async move {
        let c = Client::new();
        loop {
            let Ok(response) = c
                .get(format!("{worker_base}/v1/extension/poll?wait_ms=100"))
                .header("X-Vcu-Token", &worker_token)
                .send()
                .await
            else {
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
            let result = match method {
                "list_tabs" => json!({
                    "ok": true,
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
                    "meta": {"focused_tab_id": "42"}
                }),
                "group_tabs" => json!({
                    "ok": true,
                    "group": {
                        "group_id": "10",
                        "window_id": "7",
                        "title": "Sprint",
                        "color": "purple",
                        "collapsed": true
                    },
                    "meta": {"same_window": true}
                }),
                "type" => json!({
                    "ok": true,
                    "typed": true,
                    "tab_id": "42",
                    "page_url": "https://example.com/",
                    "focused": true
                }),
                "scroll" => json!({
                    "ok": true,
                    "scrolled": true,
                    "tab_id": "42",
                    "page_url": "https://example.com/",
                    "focused": true
                }),
                _ => json!({"ok": true}),
            };
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

fn write_msg(stdin: &mut impl Write, body: &str) {
    let bytes = body.as_bytes();
    write!(stdin, "Content-Length: {}\r\n\r\n", bytes.len()).unwrap();
    stdin.write_all(bytes).unwrap();
    stdin.flush().unwrap();
}

fn read_msg(stdout: &mut impl Read) -> Value {
    let mut headers = String::new();
    let mut one = [0u8; 1];
    let mut line = Vec::new();
    loop {
        line.clear();
        loop {
            let n = stdout.read(&mut one).unwrap();
            assert!(n > 0, "MCP stdout ended while reading headers");
            line.push(one[0]);
            if one[0] == b'\n' {
                break;
            }
        }
        let text = String::from_utf8_lossy(&line)
            .trim_end_matches(['\r', '\n'])
            .to_string();
        if text.is_empty() {
            break;
        }
        headers.push_str(&text);
        headers.push('\n');
    }
    let len = headers
        .lines()
        .find_map(|line| {
            line.to_ascii_lowercase()
                .strip_prefix("content-length:")
                .and_then(|rest| rest.trim().parse::<usize>().ok())
        })
        .expect("MCP response missing Content-Length");
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

fn call_tool(
    stdin: &mut impl Write,
    stdout: &mut impl Read,
    id: u64,
    name: &str,
    args: Value,
) -> Value {
    write_msg(
        stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "tools/call",
            "params": {"name": name, "arguments": args}
        })
        .to_string(),
    );
    let response = read_msg(stdout);
    assert_eq!(response["id"], id, "{response}");
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_else(|| panic!("MCP tool response has no text: {response}"));
    serde_json::from_str(text).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_browser_parity_tools_and_argument_forwarding() {
    let (handle, base, token, dir) = boot_async().await;
    let (log, worker) = fake_extension(&base, &token).await;
    let bin = env!("CARGO_BIN_EXE_vcu-mcp");
    let mut child = Command::new(bin)
        .arg("--user-dir")
        .arg(dir.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = child.stdout.take().unwrap();

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {"name": "browser-parity", "version": "0"}
            }
        })
        .to_string(),
    );
    let init = read_msg(&mut stdout);
    assert_eq!(init["id"], 1, "{init}");

    write_msg(
        &mut stdin,
        &json!({"jsonrpc": "2.0", "id": 2, "method": "tools/list"}).to_string(),
    );
    let listed = read_msg(&mut stdout);
    let tools = listed["result"]["tools"].as_array().unwrap();
    for name in [
        "vcu_browser_tabs",
        "vcu_browser_select",
        "vcu_browser_group",
        "vcu_browser_group_update",
        "vcu_browser_ungroup",
        "vcu_browser_open",
    ] {
        assert!(
            tools.iter().any(|tool| tool["name"] == name),
            "{name} missing"
        );
    }
    for name in ["vcu_browser_type", "vcu_browser_scroll"] {
        let tool = tools.iter().find(|tool| tool["name"] == name).unwrap();
        assert!(
            tool["inputSchema"]["properties"].get("tab_id").is_some(),
            "{name} must expose optional tab_id"
        );
    }

    let tabs = call_tool(&mut stdin, &mut stdout, 3, "vcu_browser_tabs", json!({}));
    assert_eq!(tabs["ok"], true, "{tabs}");
    assert_eq!(tabs["data"]["source"], "extension_tabs");
    assert_eq!(tabs["data"]["groups"][0]["group_id"], "9");
    assert_eq!(tabs["data"]["meta"]["focused_tab_id"], "42");

    let grouped = call_tool(
        &mut stdin,
        &mut stdout,
        4,
        "vcu_browser_group",
        json!({
            "tab_ids": ["42", "43"],
            "title": "Sprint",
            "color": "purple",
            "collapsed": true
        }),
    );
    assert_eq!(grouped["ok"], true, "{grouped}");
    assert_eq!(grouped["data"]["source"], "extension_tabs");
    assert_eq!(grouped["data"]["group"]["group_id"], "10");
    assert_eq!(grouped["data"]["meta"]["same_window"], true);

    let typed = call_tool(
        &mut stdin,
        &mut stdout,
        5,
        "vcu_browser_type",
        json!({
            "selector": "input[name=q]",
            "text": "hello",
            "tab_id": "42",
            "dry_run": true
        }),
    );
    assert_eq!(typed["ok"], true, "{typed}");
    assert_eq!(typed["data"]["source"], "extension_dom");
    assert_eq!(typed["data"]["tab_id"], "42");

    let scrolled = call_tool(
        &mut stdin,
        &mut stdout,
        6,
        "vcu_browser_scroll",
        json!({"dy": -320, "tab_id": "42", "dry_run": true}),
    );
    assert_eq!(scrolled["ok"], true, "{scrolled}");
    assert_eq!(scrolled["data"]["source"], "extension_dom");
    assert_eq!(scrolled["data"]["tab_id"], "42");

    let commands = log.lock().await.clone();
    assert!(commands
        .iter()
        .any(|command| { command.method == "list_tabs" && command.params == json!({}) }));
    assert!(commands.iter().any(|command| {
        command.method == "group_tabs"
            && command.params
                == json!({
                    "tab_ids": ["42", "43"],
                    "title": "Sprint",
                    "color": "purple",
                    "collapsed": true
                })
    }));
    assert!(commands.iter().any(|command| {
        command.method == "type"
            && command.params
                == json!({
                    "selector": "input[name=q]",
                    "text": "hello",
                    "dry_run": true,
                    "tab_id": "42"
                })
    }));
    assert!(commands.iter().any(|command| {
        command.method == "scroll"
            && command.params == json!({"dy": -320, "dry_run": true, "tab_id": "42"})
    }));

    child.kill().unwrap();
    let _ = child.wait();
    worker.abort();
    handle.join.abort();
}
