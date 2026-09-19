//! MCP Content-Length framing + tools/call against live daemon.
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{json, Value};
use vcu_core::{UserConfig, VcuPaths};

fn write_msg(stdin: &mut impl Write, body: &str) {
    let b = body.as_bytes();
    write!(stdin, "Content-Length: {}\r\n\r\n", b.len()).unwrap();
    stdin.write_all(b).unwrap();
    stdin.flush().unwrap();
}

fn read_msg(stdout: &mut impl Read) -> Value {
    let mut headers = String::new();
    let mut buf = [0u8; 1];
    // read until blank line
    let mut line = Vec::new();
    loop {
        line.clear();
        loop {
            let n = stdout.read(&mut buf).unwrap();
            assert!(n > 0, "eof headers");
            line.push(buf[0]);
            if buf[0] == b'\n' {
                break;
            }
        }
        let s = String::from_utf8_lossy(&line).trim_end_matches(['\r','\n']).to_string();
        if s.is_empty() {
            break;
        }
        headers.push_str(&s);
        headers.push('\n');
    }
    let mut len = None;
    for h in headers.lines() {
        if let Some(rest) = h.to_ascii_lowercase().strip_prefix("content-length:") {
            len = Some(rest.trim().parse::<usize>().unwrap());
        }
    }
    let len = len.expect("content-length");
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).unwrap();
    serde_json::from_slice(&body).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn mcp_initialize_and_session_tools() {
    let dir = tempfile::tempdir().unwrap();
    let paths = VcuPaths::from_root(dir.path());
    let mut cfg = UserConfig::default();
    cfg.daemon_port = 0;
    paths.save_config(&cfg).unwrap();
    let handle = vcu_server::start_daemon(paths.clone(), cfg).await.unwrap();

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
            "jsonrpc":"2.0","id":1,"method":"initialize",
            "params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0"}}
        })
        .to_string(),
    );
    let init = read_msg(&mut stdout);
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["serverInfo"]["name"], "vcu-mcp");

    write_msg(
        &mut stdin,
        &json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}).to_string(),
    );
    let tools = read_msg(&mut stdout);
    let arr = tools["result"]["tools"].as_array().unwrap();
    assert!(arr.iter().any(|t| t["name"] == "vcu_session_start"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_browser_observe"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_browser_next"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_browser_login_state"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_browser_ping"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_browser_extract"));
    for name in ["vcu_browser_tabs", "vcu_browser_select", "vcu_browser_open", "vcu_browser_group", "vcu_browser_group_update", "vcu_browser_ungroup"] {
        assert!(arr.iter().any(|t| t["name"] == name), "missing {name}");
    }
    let ext_tool = arr.iter().find(|t| t["name"] == "vcu_browser_extract").unwrap();
    assert!(ext_tool["inputSchema"]["properties"].get("selector").is_some());
    assert!(arr.iter().any(|t| t["name"] == "vcu_snapshot"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_screenshot"));
    assert!(arr.iter().any(|t| t["name"] == "vcu_session_list"));
    let snap_tool = arr.iter().find(|t| t["name"] == "vcu_snapshot").unwrap();
    assert!(snap_tool["inputSchema"]["properties"].get("tab_id").is_some());
    let shot_tool = arr.iter().find(|t| t["name"] == "vcu_screenshot").unwrap();
    assert!(shot_tool["inputSchema"]["properties"].get("tab_id").is_some());
    for name in ["vcu_type", "vcu_scroll", "vcu_wait", "vcu_extract"] {
        let tool = arr.iter().find(|t| t["name"] == name).unwrap();
        assert!(
            tool["inputSchema"]["properties"].get("tab_id").is_some(),
            "{name} missing tab_id"
        );
    }

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":3,"method":"tools/call",
            "params":{"name":"vcu_session_start","arguments":{"backend":"mock","browser":"mock"}}
        })
        .to_string(),
    );
    let start = read_msg(&mut stdout);
    let text = start["result"]["content"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["ok"], true);
    let sid = body["data"]["session_id"].as_str().unwrap();

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":4,"method":"tools/call",
            "params":{"name":"vcu_navigate","arguments":{"session":sid,"url":"https://example.com/"}}
        })
        .to_string(),
    );
    let nav = read_msg(&mut stdout);
    let text = nav["result"]["content"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["ok"], true);

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":5,"method":"tools/call",
            "params":{"name":"vcu_snapshot","arguments":{"session":sid,"mode":"a11y"}}
        })
        .to_string(),
    );
    let snap = read_msg(&mut stdout);
    let text = snap["result"]["content"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["data"]["a11y_summary"].as_str().unwrap().contains("Example Domain"));

    // os cursor denied via act tool
    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":6,"method":"tools/call",
            "params":{"name":"vcu_act","arguments":{"session":sid,"type":"os_click","target":{},"args":{}}}
        })
        .to_string(),
    );
    let denied = read_msg(&mut stdout);
    let text = denied["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("OsCursorDenied") || text.contains("os_cursor"));

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":7,"method":"tools/call",
            "params":{"name":"vcu_screenshot","arguments":{"session":sid}}
        })
        .to_string(),
    );
    let shot = read_msg(&mut stdout);
    let text = shot["result"]["content"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["ok"], true);
    assert!(body["data"]["sha256"].as_str().unwrap().len() >= 16);

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":8,"method":"tools/call",
            "params":{"name":"vcu_session_list","arguments":{}}
        })
        .to_string(),
    );
    let listed = read_msg(&mut stdout);
    let text = listed["result"]["content"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["ok"], true);

    write_msg(
        &mut stdin,
        &json!({
            "jsonrpc":"2.0","id":9,"method":"tools/call",
            "params":{"name":"vcu_session_stop","arguments":{"session":"all"}}
        })
        .to_string(),
    );
    let stopped = read_msg(&mut stdout);
    let text = stopped["result"]["content"][0]["text"].as_str().unwrap();
    let body: Value = serde_json::from_str(text).unwrap();
    assert_eq!(body["ok"], true);

    drop(stdin);
    let _ = child.wait();
    handle.join.abort();
    let _ = Duration::from_millis(10);
}
