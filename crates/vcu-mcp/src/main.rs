//! VCU MCP server (stdio, JSON-RPC 2.0, Content-Length framing).
//!
//! Agent hosts (Codex, Claude Code, Cursor, …) speak **MCP** to this process.
//! This server translates tool calls into authenticated HTTP calls against the
//! local `vcu-daemon`. Shell fallback is not used for tool execution.
//!
//! Users who prefer not to use MCP can still drive VCU purely via the `vcu` CLI.

use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use serde_json::{json, Value};
use vcu_core::{VcuPaths, VcuResult, ErrorCode, VcuError};

#[derive(Parser, Debug)]
#[command(name = "vcu-mcp", version)]
struct Args {
    #[arg(long, env = "VCU_DIR")]
    user_dir: Option<PathBuf>,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .with_target(false)
        .compact()
        .init();

    let args = Args::parse();
    let paths = match args.user_dir {
        Some(p) => VcuPaths::from_root(p),
        None => VcuPaths::default_user().expect("home dir"),
    };
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    if let Err(e) = rt.block_on(serve(paths)) {
        eprintln!("vcu-mcp error: {e}");
        std::process::exit(1);
    }
}

async fn serve(paths: VcuPaths) -> VcuResult<()> {
    let mut stdin = io::stdin().lock();
    let mut stdout = io::stdout();
    loop {
        let msg = match read_message(&mut stdin)? {
            Some(m) => m,
            None => break,
        };
        let req: Value = serde_json::from_str(&msg).map_err(|e| {
            VcuError::with_detail(ErrorCode::InvalidInput, "mcp json", e.to_string())
        })?;
        // notifications have no id
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");
        if id.is_none() {
            // notification
            if method == "notifications/initialized" || method == "initialized" {
                continue;
            }
            continue;
        }
        let id = id.unwrap();
        let resp = match method {
            "initialize" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": {
                        "name": "vcu-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "instructions": "VCU computer-use tools. Prefer backend=mock for tests; cdp/extension for real browsers. Start daemon first: vcu daemon start."
                }
            }),
            "ping" => json!({"jsonrpc":"2.0","id": id, "result": {}}),
            "tools/list" => json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tool_defs() }
            }),
            "tools/call" => {
                let name = req.pointer("/params/name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = req.pointer("/params/arguments").cloned().unwrap_or(json!({}));
                match handle_tool(&paths, name, arguments).await {
                    Ok(text) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{"type": "text", "text": text}],
                            "isError": false
                        }
                    }),
                    Err(e) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{"type": "text", "text": format!(
                                "{{\"ok\":false,\"error\":{{\"code\":\"{:?}\",\"message\":\"{}\",\"repair_hint\":\"{}\"}}}}",
                                e.code(), e.message().replace('"', "'"), e.repair_hint().replace('"', "'")
                            )}],
                            "isError": true
                        }
                    }),
                }
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {"code": -32601, "message": format!("method not found: {method}")}
            }),
        };
        write_message(&mut stdout, &resp.to_string())?;
    }
    Ok(())
}

fn tool_defs() -> Vec<Value> {
    vec![
        tool("vcu_doctor", "Diagnose VCU install and daemon", json!({"type":"object","properties":{}})),
        tool("vcu_session_start", "Start a browser computer-use session", json!({
            "type":"object",
            "properties":{
                "backend":{"type":"string","description":"mock|cdp|extension","default":"mock"},
                "browser":{"type":"string","description":"chrome|edge|auto|mock","default":"auto"},
                "vision_policy":{"type":"string"}
            }
        })),
        tool("vcu_session_stop", "Stop a session and release borrows", json!({
            "type":"object","required":["session"],
            "properties":{"session":{"type":"string"}}
        })),
        tool("vcu_tabs_list", "List tabs for a session", json!({
            "type":"object","required":["session"],
            "properties":{"session":{"type":"string"}}
        })),
        tool("vcu_tabs_borrow", "Borrow a user tab for write actions", json!({
            "type":"object","required":["session","tab_id"],
            "properties":{"session":{"type":"string"},"tab_id":{"type":"string"}}
        })),
        tool("vcu_navigate", "Navigate active/borrowed tab", json!({
            "type":"object","required":["session","url"],
            "properties":{"session":{"type":"string"},"url":{"type":"string"},"tab_id":{"type":"string"}}
        })),
        tool("vcu_snapshot", "Observe page (a11y/dom/text/full)", json!({
            "type":"object","required":["session"],
            "properties":{
                "session":{"type":"string"},
                "mode":{"type":"string","default":"a11y"},
                "budget":{"type":"integer","default":4000},
                "force_vision":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_click", "Click element by ref (page DOM, never OS cursor)", json!({
            "type":"object","required":["session","ref"],
            "properties":{"session":{"type":"string"},"ref":{"type":"string"},"tab_id":{"type":"string"}}
        })),
        tool("vcu_type", "Type text into element", json!({
            "type":"object","required":["session","text"],
            "properties":{"session":{"type":"string"},"text":{"type":"string"},"ref":{"type":"string"}}
        })),
        tool("vcu_extract", "Extract DOM matches by selector", json!({
            "type":"object","required":["session","selector"],
            "properties":{"session":{"type":"string"},"selector":{"type":"string"}}
        })),
        tool("vcu_scroll", "Scroll page by dy pixels", json!({
            "type":"object","required":["session"],
            "properties":{"session":{"type":"string"},"dy":{"type":"integer","default":600}}
        })),
        tool("vcu_act", "Generic action JSON (type/target/args)", json!({
            "type":"object","required":["session","type"],
            "properties":{
                "session":{"type":"string"},
                "type":{"type":"string"},
                "target":{"type":"object"},
                "args":{"type":"object"}
            }
        })),
        tool("vcu_app_windows", "List desktop app windows (macOS/Windows adapter)", json!({"type":"object","properties":{}})),
        tool("vcu_app_snapshot", "Snapshot app UI (best-effort AX/UIA)", json!({
            "type":"object","required":["id"],
            "properties":{"id":{"type":"string"},"budget":{"type":"integer","default":4000}}
        })),
        tool("vcu_blackboard", "Read session blackboard for main/sub-agent sharing", json!({
            "type":"object","required":["session"],
            "properties":{"session":{"type":"string"}}
        })),
    ]
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema
    })
}

async fn handle_tool(paths: &VcuPaths, name: &str, args: Value) -> VcuResult<String> {
    let client = ApiClient::connect(paths).await?;
    let body = match name {
        "vcu_doctor" => client.get("/v1/doctor").await?,
        "vcu_session_start" => {
            let backend = args.get("backend").and_then(|v| v.as_str()).unwrap_or("mock");
            let browser = args.get("browser").and_then(|v| v.as_str()).unwrap_or("auto");
            let mut body = json!({"backend": backend, "browser": browser});
            if let Some(vp) = args.get("vision_policy") {
                body["vision_policy"] = vp.clone();
            }
            client.post("/v1/session/start", body).await?
        }
        "vcu_session_stop" => {
            let sid = req_str(&args, "session")?;
            client.post(&format!("/v1/session/{sid}/stop"), json!({})).await?
        }
        "vcu_tabs_list" => {
            let sid = req_str(&args, "session")?;
            client.get(&format!("/v1/session/{sid}/tabs")).await?
        }
        "vcu_tabs_borrow" => {
            let sid = req_str(&args, "session")?;
            let tab = req_str(&args, "tab_id")?;
            client
                .post(
                    &format!("/v1/session/{sid}/tabs/borrow"),
                    json!({"tab_id": tab}),
                )
                .await?
        }
        "vcu_navigate" => {
            let sid = req_str(&args, "session")?;
            let url = req_str(&args, "url")?;
            let mut body = json!({"url": url});
            if let Some(t) = args.get("tab_id") {
                body["tab_id"] = t.clone();
            }
            client
                .post(&format!("/v1/session/{sid}/navigate"), body)
                .await?
        }
        "vcu_snapshot" => {
            let sid = req_str(&args, "session")?;
            let mode = args.get("mode").and_then(|v| v.as_str()).unwrap_or("a11y");
            let budget = args.get("budget").and_then(|v| v.as_u64()).unwrap_or(4000);
            let force_vision = args
                .get("force_vision")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            client
                .post(
                    &format!("/v1/session/{sid}/snapshot"),
                    json!({"mode": mode, "budget": budget, "force_vision": force_vision}),
                )
                .await?
        }
        "vcu_click" => {
            let sid = req_str(&args, "session")?;
            let r = req_str(&args, "ref")?;
            let mut body = json!({"ref": r});
            if let Some(t) = args.get("tab_id") {
                body["tab_id"] = t.clone();
            }
            client
                .post(&format!("/v1/session/{sid}/click"), body)
                .await?
        }
        "vcu_type" => {
            let sid = req_str(&args, "session")?;
            let text = req_str(&args, "text")?;
            let mut body = json!({"text": text});
            if let Some(r) = args.get("ref") {
                body["ref"] = r.clone();
            }
            client.post(&format!("/v1/session/{sid}/type"), body).await?
        }
        "vcu_extract" => {
            let sid = req_str(&args, "session")?;
            let selector = req_str(&args, "selector")?;
            client
                .post(
                    &format!("/v1/session/{sid}/extract"),
                    json!({"selector": selector}),
                )
                .await?
        }
        "vcu_scroll" => {
            let sid = req_str(&args, "session")?;
            let dy = args.get("dy").and_then(|v| v.as_i64()).unwrap_or(600);
            client
                .post(
                    &format!("/v1/session/{sid}/act"),
                    json!({"type":"scroll","target":{},"args":{"dy": dy}}),
                )
                .await?
        }
        "vcu_act" => {
            let sid = req_str(&args, "session")?;
            let ty = req_str(&args, "type")?;
            let target = args.get("target").cloned().unwrap_or(json!({}));
            let a = args.get("args").cloned().unwrap_or(json!({}));
            client
                .post(
                    &format!("/v1/session/{sid}/act"),
                    json!({"type": ty, "target": target, "args": a}),
                )
                .await?
        }
        "vcu_app_windows" => client.get("/v1/app/windows").await?,
        "vcu_app_snapshot" => {
            let id = req_str(&args, "id")?;
            let budget = args.get("budget").and_then(|v| v.as_u64()).unwrap_or(4000);
            client
                .post("/v1/app/snapshot", json!({"id": id, "budget": budget}))
                .await?
        }
        "vcu_blackboard" => {
            let sid = req_str(&args, "session")?;
            client.get(&format!("/v1/session/{sid}/blackboard")).await?
        }
        other => {
            return Err(VcuError::coded(
                ErrorCode::InvalidInput,
                format!("unknown tool: {other}"),
            ))
        }
    };
    Ok(serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string()))
}

fn req_str<'a>(args: &'a Value, key: &str) -> VcuResult<&'a str> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| VcuError::coded(ErrorCode::InvalidInput, format!("missing {key}")))
}

struct ApiClient {
    http: reqwest::Client,
    endpoint: String,
    token: String,
}

impl ApiClient {
    async fn connect(paths: &VcuPaths) -> VcuResult<Self> {
        let cfg = paths.load_config().map_err(|_| {
            VcuError::coded(ErrorCode::InvalidInput, "run `vcu init` before using vcu-mcp")
        })?;
        let endpoint = if paths.endpoint_path().exists() {
            std::fs::read_to_string(paths.endpoint_path())
                .unwrap_or_else(|_| VcuPaths::endpoint_url(&cfg))
                .trim()
                .to_string()
        } else {
            VcuPaths::endpoint_url(&cfg)
        };
        let http = reqwest::Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "http client", e.to_string()))?;
        // health probe
        let url = format!("{}/v1/health", endpoint.trim_end_matches('/'));
        match http.get(&url).send().await {
            Ok(r) if r.status().is_success() => {}
            _ => {
                return Err(VcuError::coded(
                    ErrorCode::DaemonNotRunning,
                    "vcu-daemon is not reachable; run `vcu daemon start`",
                ))
            }
        }
        Ok(Self {
            http,
            endpoint,
            token: cfg.pairing_token,
        })
    }

    async fn get(&self, path: &str) -> VcuResult<Value> {
        let url = format!("{}{}", self.endpoint.trim_end_matches('/'), path);
        let resp = self
            .http
            .get(&url)
            .header("X-Vcu-Token", &self.token)
            .send()
            .await
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::DaemonNotRunning, format!("GET {url}"), e.to_string())
            })?;
        resp.json().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "decode", e.to_string())
        })
    }

    async fn post(&self, path: &str, body: Value) -> VcuResult<Value> {
        let url = format!("{}{}", self.endpoint.trim_end_matches('/'), path);
        let resp = self
            .http
            .post(&url)
            .header("X-Vcu-Token", &self.token)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::DaemonNotRunning, format!("POST {url}"), e.to_string())
            })?;
        resp.json().await.map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "decode", e.to_string())
        })
    }
}

fn read_message(stdin: &mut impl Read) -> VcuResult<Option<String>> {
    let mut headers = Vec::new();
    let mut line_buf = Vec::new();
    loop {
        line_buf.clear();
        let n = read_line(stdin, &mut line_buf)?;
        if n == 0 {
            return Ok(None);
        }
        let line = String::from_utf8_lossy(&line_buf).trim_end_matches(['\r', '\n']).to_string();
        if line.is_empty() {
            break;
        }
        headers.push(line);
    }
    if headers.is_empty() {
        // try newline-delimited JSON fallback: already consumed blank — read one more blob
        return Ok(None);
    }
    let mut content_length = None;
    for h in &headers {
        let lower = h.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("content-length:") {
            content_length = rest.trim().parse::<usize>().ok();
        }
    }
    let len = content_length.ok_or_else(|| {
        VcuError::coded(ErrorCode::InvalidInput, "MCP message missing Content-Length")
    })?;
    let mut body = vec![0u8; len];
    stdin.read_exact(&mut body).map_err(|e| {
        VcuError::with_detail(ErrorCode::InvalidInput, "read body", e.to_string())
    })?;
    Ok(Some(String::from_utf8_lossy(&body).to_string()))
}

fn read_line(r: &mut impl Read, buf: &mut Vec<u8>) -> io::Result<usize> {
    let mut n = 0;
    let mut b = [0u8; 1];
    loop {
        let k = r.read(&mut b)?;
        if k == 0 {
            return Ok(n);
        }
        n += 1;
        buf.push(b[0]);
        if b[0] == b'\n' {
            return Ok(n);
        }
    }
}

fn write_message(stdout: &mut impl Write, body: &str) -> VcuResult<()> {
    let bytes = body.as_bytes();
    write!(stdout, "Content-Length: {}\r\n\r\n", bytes.len()).map_err(VcuError::from)?;
    stdout.write_all(bytes).map_err(VcuError::from)?;
    stdout.flush().map_err(VcuError::from)?;
    Ok(())
}
