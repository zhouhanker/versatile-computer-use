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
use base64::Engine;
use serde_json::{json, Value};
use std::collections::BTreeSet;
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
                    "instructions": "VCU browser computer-use on USER Chrome/Edge (dual-browser). Host vision is enough. First vcu_browser_ping, then vcu_browser_observe (look at the PNG). Observe stamps tab_id; for 60s, selector click/type/hover/scroll/extract, viewport screenshot, and open (existing USER window) bind that last observe and its Chrome/Edge lens. Explicit tab_id still wins. Webpage pixels: vcu_browser_screenshot, view PNG this turn, then vcu_browser_click(space=viewport,capture_id,pixel_x,pixel_y). Captures expire in 60s and one real action consumes them; on stale/timeout observe again, do not replay. DOM events are synthetic (trusted=false); verify page results. Never mix observe window/webview pixels with viewport images. Desktop session snapshots of Chrome/Edge may include browser_tabs but source stays ax_scene — HTML clicks still use vcu_browser_*. Never CDP Allow, OS cursor warp, WeChat, or blind Return."
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
                    Ok(value) => json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": mcp_vision_content(&value),
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
        tool("vcu_health", "Daemon health including extension_polling / last_poll_age_ms", json!({"type":"object","properties":{}})),
        tool("vcu_doctor", "Diagnose VCU install and daemon", json!({"type":"object","properties":{}})),
        tool("vcu_browser_login_state", "Classify user vs empty Agent browser. Login-state is the USER window, not ~/.vcu/edge-agent-profile.", json!({"type":"object","properties":{}})),
        tool("vcu_browser_next", "Next login-state action only (load unpacked / observe). Never clicks Allow.", json!({"type":"object","properties":{}})),
        tool("vcu_browser_install_lens", "Copy VCU extension to ~/.vcu/lens-extension. Optional reload hot-restarts every connected Edge/Chrome SW. Does not click UI or Allow.", json!({"type":"object","properties":{"reload":{"type":"boolean","default":false}}})),
        tool("vcu_browser_observe", "Observe USER Chrome/Edge without Stage HUD. Without tab_id, frontmost must be USER Chrome/Edge; otherwise pass tab_id. Optional tab_id captures that tab (activates in-window, does not steal OS frontmost). Returns PNG plus JSON (vision_handoff.must_view). Look at the image before clicking.", json!({
            "type":"object",
            "properties":{
                "pixels":{"type":"boolean","default":true},
                "selector":{"type":"string","description":"Scene extract filter; default *"},
                "budget":{"type":"integer","default":2500},
                "tab_id":{"type":"string","description":"Exact USER tab to capture; does not steal OS frontmost"},
                "browser":{"type":"string","description":"chrome or edge; required when tab_id collides; without tab_id observes that browser's focused tab"}
            }
        })),
        tool("vcu_browser_screenshot", "Capture a USER tab viewport as PNG. Without tab_id, uses last observe tab for 60s. View image before click with space=viewport and capture_id. Expires in 60s; real click consumes capture.", json!({"type":"object","properties":{"tab_id":{"type":"string"},"browser":{"type":"string"}}})),
        tool("vcu_browser_click", "Click a USER webpage by unique selector or bound viewport screenshot pixels (capture_id required). Without tab_id, selector clicks bind last observe for 60s. window/webview is AX, not DOM. dry_run does not press. Never OS cursor or Allow.", json!({
            "type":"object",
            "properties":{
                "capture_id":{"type":"string","description":"Required for space=viewport; use browser_screenshot capture_id"},
                "pixel_x":{"type":"number"},
                "pixel_y":{"type":"number"},
                "selector":{"type":"string","description":"CSS selector; DOM click via USER extension"},
                "tab_id":{"type":"string","description":"USER tab id; default last observe"},
                "browser":{"type":"string","description":"chrome or edge when tab_id collides"},
                "space":{"type":"string","description":"viewport (bound browser screenshot), window or webview (AX)","default":"window"},
                "dry_run":{"type":"boolean","default":false},
                "guide":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_browser_type", "Type on USER browser without HUD. Default AX address bar. selector uses USER extension DOM (source=extension_dom); a native <select> is set by option value or label (input_path=dom_select).", json!({
            "type":"object","properties":{
                "text":{"type":"string"},
                "ref":{"type":"string"},
                "selector":{"type":"string","description":"CSS selector; DOM type via USER extension"},
                "tab_id":{"type":"string","description":"Exact tab ID; no fallback if missing"},
                "browser":{"type":"string","description":"chrome or edge when tab_id collides"},
                "dry_run":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_browser_hover", "Hover a unique CSS selector on a USER tab via extension DOM. Synthetic mouseover; trusted=false; dry_run does not dispatch.", json!({
            "type":"object","required":["selector"],
            "properties":{
                "selector":{"type":"string"},
                "tab_id":{"type":"string"},
                "browser":{"type":"string"},
                "dry_run":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_browser_scroll", "Scroll USER tab via extension DOM. Explicit tab never falls back to AX. dry_run does not scroll.", json!({
            "type":"object","properties":{
                "dy":{"type":"integer","default":600},
                "tab_id":{"type":"string","description":"Exact tab ID; no fallback if missing"},
                "browser":{"type":"string"},
                "dry_run":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_browser_key", "Policy-gated key on USER browser. Never HID. Return requires confirm_send + Send ref. dry_run reports the gate.", json!({
            "type":"object","required":["key"],
            "properties":{
                "key":{"type":"string"},
                "dry_run":{"type":"boolean","default":true},
                "confirm_send":{"type":"boolean","default":false},
                "ref":{"type":"string"}
            }
        })),
        tool("vcu_browser_wait", "Wait on USER browser without HUD. selector polls DOM (last observe for 60s). Optional text substring. name/role/ref still Scene AX. Else sleep ms.", json!({
            "type":"object","properties":{
                "ms":{"type":"integer","default":200},
                "ref":{"type":"string"},
                "name":{"type":"string"},
                "role":{"type":"string"},
                "selector":{"type":"string","description":"CSS selector; DOM wait via USER extension"},
                "text":{"type":"string","description":"Optional extract text/value substring"},
                "tab_id":{"type":"string"},
                "browser":{"type":"string"}
            }
        })),
        tool("vcu_browser_tabs", "List USER browser tabs and native groups, including focus and collapsed state.", json!({"type":"object","properties":{}})),
        tool("vcu_browser_select", "Select an exact tab, expand its group and focus its browser window. Pass browser=chrome|edge when tab_id collides.", json!({"type":"object","required":["tab_id"],"properties":{"tab_id":{"type":"string"},"browser":{"type":"string"}}})),
        tool("vcu_browser_close", "Close exactly the explicitly named tab. Pass browser=chrome|edge when tab_id collides. Never substitutes another tab.", json!({"type":"object","required":["tab_id"],"properties":{"tab_id":{"type":"string"},"browser":{"type":"string"}}})),
        tool("vcu_browser_open", "Open a background tab in the VCU tab group. Does not replace the user page or focus the window. browser=chrome|edge overrides last observe. new_window=true is opt-in.", json!({"type":"object","required":["url"],"properties":{
            "url":{"type":"string"},"session_name":{"type":"string"},"group_id":{"type":"string"},"active":{"type":"boolean","default":false},"new_window":{"type":"boolean","default":false},"browser":{"type":"string","description":"chrome or edge"}
        }})),
        tool("vcu_browser_group", "Group explicit same-window tab IDs in one browser. Rejects Chrome+Edge mixes. Pass browser when tab_id collides.", json!({"type":"object","required":["tab_ids","title"],"properties":{
            "tab_ids":{"type":"array","minItems":1,"items":{"type":"string"}},"title":{"type":"string"},"color":{"type":"string","default":"purple"},"collapsed":{"type":"boolean","default":false},"browser":{"type":"string"}
        }})),
        tool("vcu_browser_group_update", "Rename, recolor, expand or collapse a native browser group. Pass browser when group_id collides. Does not close tabs.", json!({"type":"object","required":["group_id"],"properties":{
            "group_id":{"type":"string"},"title":{"type":"string"},"color":{"type":"string"},"collapsed":{"type":"boolean"},"browser":{"type":"string"}
        }})),
        tool("vcu_browser_ungroup", "Ungroup explicit tab IDs without closing their pages. Pass browser when tab_id collides.", json!({"type":"object","required":["tab_ids"],"properties":{"tab_ids":{"type":"array","minItems":1,"items":{"type":"string"}},"browser":{"type":"string"}}})),
        tool("vcu_browser_ping", "Ping USER Edge/Chrome extension SW. Must pong. reload=true copies nothing but chrome.runtime.reload on every connected lens. Never click Allow.", json!({"type":"object","properties":{"reload":{"type":"boolean","default":false}}})),
        tool("vcu_browser_extract", "DOM extract via USER Edge extension. source must be extension_dom. No HUD, no Agent Edge, no AX chrome fake-green.", json!({
            "type":"object","properties":{
                "selector":{"type":"string","default":"a"},
                "tab_id":{"type":"string"},
                "browser":{"type":"string","description":"chrome or edge when tab_id collides"}
            }
        })),
        tool("vcu_session_start", "Start a VCU session. surface=desktop is the real-window Steward path; browser_agent is the isolated profile bypass.", json!({
            "type":"object",
            "properties":{
                "surface":{"type":"string","description":"desktop|browser_agent"},
                "backend":{"type":"string","description":"mock|cdp|extension|desktop"},
                "browser":{"type":"string","description":"chrome|edge|auto|mock","default":"auto"},
                "vision_policy":{"type":"string"}
            }
        })),
        tool("vcu_session_list", "List active VCU sessions", json!({"type":"object","properties":{}})),
        tool("vcu_session_stop", "Stop a session and release borrows. session=all stops every session.", json!({
            "type":"object","required":["session"],
            "properties":{"session":{"type":"string"}}
        })),
        tool("vcu_session_abort", "Abort a desktop session the same way as Stage Escape. Tears down HUD. Not session stop.", json!({
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
        tool("vcu_snapshot", "Observe Scene or page (a11y/dom/text/full). Desktop Chrome/Edge may include browser_tabs; source stays ax_scene — HTML actions still use vcu_browser_*. mode=full attaches webview_screenshot_ref for messenger panes.", json!({
            "type":"object","required":["session"],
            "properties":{
                "session":{"type":"string"},
                "tab_id":{"type":"string"},
                "mode":{"type":"string","default":"a11y"},
                "budget":{"type":"integer","default":4000},
                "force_vision":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_hover", "Move Guide overlay to a Scene ref. Never OS cursor / HID. Desktop only.", json!({
            "type":"object","required":["session","ref"],
            "properties":{
                "session":{"type":"string"},
                "ref":{"type":"string"},
                "tab_id":{"type":"string"}
            }
        })),
        tool("vcu_click", "Click by Scene ref, or screenshot pixels mapped to AX points (never OS cursor)", json!({
            "type":"object","required":["session"],
            "properties":{
                "session":{"type":"string"},
                "ref":{"type":"string"},
                "tab_id":{"type":"string"},
                "pixel_x":{"type":"number"},
                "pixel_y":{"type":"number"},
                "space":{"type":"string","description":"window or webview screenshot space"}
            }
        })),
        tool("vcu_type", "Type text into element", json!({
            "type":"object","required":["session","text"],
            "properties":{"session":{"type":"string"},"text":{"type":"string"},"ref":{"type":"string"},"tab_id":{"type":"string"}}
        })),
        tool("vcu_extract", "Extract DOM matches by selector", json!({
            "type":"object","required":["session","selector"],
            "properties":{"session":{"type":"string"},"selector":{"type":"string"},"tab_id":{"type":"string"}}
        })),
        tool("vcu_scroll", "Scroll page by dy pixels", json!({
            "type":"object","required":["session"],
            "properties":{"session":{"type":"string"},"tab_id":{"type":"string"},"dy":{"type":"integer","default":600}}
        })),
        tool("vcu_screenshot", "Capture session screenshot (desktop Scene or browser tab). Desktop Feishu/Electron also attaches webview crop when present.", json!({
            "type":"object","required":["session"],
            "properties":{
                "session":{"type":"string"},
                "tab_id":{"type":"string"},
                "full_page":{"type":"boolean","default":false}
            }
        })),
        tool("vcu_wait", "Wait ms, or until a Scene ref/name/role/value appears (desktop)", json!({
            "type":"object","required":["session"],
            "properties":{
                "session":{"type":"string"},
                "tab_id":{"type":"string"},
                "ms":{"type":"integer","default":200},
                "ref":{"type":"string"},
                "name":{"type":"string"},
                "role":{"type":"string"},
                "value":{"type":"string","description":"Match Scene name or value (CU-D-200)"}
            }
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
        tool("vcu_app_snapshot", "Snapshot app UI (AX/UIA). pixels=true attaches a window PNG when Screen Recording is already granted. selector filters Scene without raising Stage HUD.", json!({
            "type":"object","required":["id"],
            "properties":{"id":{"type":"string"},"budget":{"type":"integer","default":4000},"pixels":{"type":"boolean","default":false},"selector":{"type":"string"}}
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

fn vision_png_paths(v: &Value) -> Vec<PathBuf> {
    let mut found = Vec::new();
    fn walk(v: &Value, found: &mut Vec<PathBuf>) {
        match v {
            Value::Object(map) => {
                if let Some(arr) = map.get("must_view").and_then(|x| x.as_array()) {
                    for p in arr {
                        if let Some(s) = p.as_str() {
                            found.push(PathBuf::from(s));
                        }
                    }
                }
                for key in [
                    "screenshot_path",
                    "webview_screenshot_path",
                    "login_latest_png",
                    "feishu_latest_png",
                    "path",
                    "webview_path",
                ] {
                    if let Some(s) = map.get(key).and_then(|x| x.as_str()) {
                        if s.ends_with(".png") {
                            found.push(PathBuf::from(s));
                        }
                    }
                }
                for child in map.values() {
                    walk(child, found);
                }
            }
            Value::Array(items) => {
                for child in items {
                    walk(child, found);
                }
            }
            _ => {}
        }
    }
    walk(v, &mut found);
    let mut seen = BTreeSet::new();
    found
        .into_iter()
        .filter(|p| seen.insert(p.clone()) && p.is_file())
        .take(2)
        .collect()
}

fn png_to_mcp_image(path: &std::path::Path) -> Option<Value> {
    const MAX: u64 = 8_000_000;
    let meta = std::fs::metadata(path).ok()?;
    if meta.len() == 0 || meta.len() > MAX {
        return None;
    }
    let bytes = std::fs::read(path).ok()?;
    Some(json!({
        "type": "image",
        "mimeType": "image/png",
        "data": base64::engine::general_purpose::STANDARD.encode(bytes)
    }))
}

fn mcp_vision_content(v: &Value) -> Vec<Value> {
    let text = serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string());
    let mut content = vec![json!({"type": "text", "text": text})];
    for path in vision_png_paths(v) {
        if let Some(img) = png_to_mcp_image(&path) {
            content.push(img);
        }
    }
    content
}

async fn handle_tool(paths: &VcuPaths, name: &str, args: Value) -> VcuResult<Value> {
    let client = ApiClient::connect(paths).await?;
    let body = match name {
        "vcu_health" => client.get("/v1/health").await?,
        "vcu_doctor" => client.get("/v1/doctor").await?,
        "vcu_browser_login_state" => client.get("/v1/browser/login-state").await?,
        "vcu_browser_next" => {
            let v = client.get("/v1/browser/login-state").await?;
            let action = v
                .pointer("/data/next_action")
                .and_then(|x| x.as_str())
                .unwrap_or("vcu browser observe")
                .to_string();
            json!({"ok": true, "data": {
                "next_action": action,
                "lens_dir": v.pointer("/data/lens_dir").and_then(|x| x.as_str()).unwrap_or(""),
                "lens_copied": v.pointer("/data/lens_copied").and_then(|x| x.as_bool()).unwrap_or(false),
                "extension_profile": v.pointer("/data/extension_profile").and_then(|x| x.as_str()).unwrap_or(""),
                "observe": "vcu browser observe --json",
                "install_lens": "vcu browser install-lens",
            }})
        }
        "vcu_browser_install_lens" => {
            let copied = client.post("/v1/browser/install-lens", json!({})).await?;
            if args.get("reload").and_then(|v| v.as_bool()).unwrap_or(false) {
                let reloaded = client.post("/v1/browser/ping", json!({"reload": true})).await?;
                json!({
                    "ok": true,
                    "data": {
                        "copied": copied.get("data").cloned().unwrap_or(copied.clone()),
                        "reload": reloaded.get("data").cloned().unwrap_or(reloaded),
                    }
                })
            } else {
                copied
            }
        }
        "vcu_browser_observe" => client.post("/v1/browser/observe", args).await?,
        "vcu_browser_screenshot" => client.post("/v1/browser/screenshot", args).await?,
        "vcu_browser_click" => {
            let mut body = json!({
                "space": args.get("space").and_then(|v| v.as_str()).unwrap_or("window"),
                "dry_run": args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false),
                "guide": args.get("guide").and_then(|v| v.as_bool()).unwrap_or(false)
            });
            if let Some(id) = args.get("capture_id") { body["capture_id"] = id.clone(); }
            if let Some(px) = args.get("pixel_x") { body["pixel_x"] = px.clone(); }
            if let Some(py) = args.get("pixel_y") { body["pixel_y"] = py.clone(); }
            if let Some(sel) = args.get("selector") { body["selector"] = sel.clone(); }
            if let Some(t) = args.get("tab_id") { body["tab_id"] = t.clone(); }
            if let Some(b) = args.get("browser") { body["browser"] = b.clone(); }
            client.post("/v1/browser/click", body).await?
        }
        "vcu_browser_type" => {
            let mut body = json!({
                "dry_run": args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false)
            });
            if let Some(t) = args.get("text") { body["text"] = t.clone(); }
            if let Some(r) = args.get("ref") { body["ref"] = r.clone(); }
            if let Some(sel) = args.get("selector") { body["selector"] = sel.clone(); }
            if let Some(t) = args.get("tab_id") { body["tab_id"] = t.clone(); }
            if let Some(b) = args.get("browser") { body["browser"] = b.clone(); }
            client.post("/v1/browser/type", body).await?
        }
        "vcu_browser_hover" => {
            let mut body = json!({
                "selector": req_str(&args, "selector")?,
                "dry_run": args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false)
            });
            if let Some(t) = args.get("tab_id") { body["tab_id"] = t.clone(); }
            if let Some(b) = args.get("browser") { body["browser"] = b.clone(); }
            client.post("/v1/browser/hover", body).await?
        }
        "vcu_browser_scroll" => {
            let mut body = json!({
                "tab_id": args.get("tab_id"),
                "dy": args.get("dy").and_then(|v| v.as_i64()).unwrap_or(600),
                "dry_run": args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(false)
            });
            if let Some(b) = args.get("browser") { body["browser"] = b.clone(); }
            client.post("/v1/browser/scroll", body).await?
        }
        "vcu_browser_key" => {
            let mut body = json!({
                "key": req_str(&args, "key")?,
                "dry_run": args.get("dry_run").and_then(|v| v.as_bool()).unwrap_or(true),
                "confirm_send": args.get("confirm_send").and_then(|v| v.as_bool()).unwrap_or(false)
            });
            if let Some(r) = args.get("ref") { body["ref"] = r.clone(); }
            client.post("/v1/browser/key", body).await?
        }
        "vcu_browser_wait" => {
            let mut body = json!({
                "ms": args.get("ms").and_then(|v| v.as_u64()).unwrap_or(200)
            });
            if let Some(r) = args.get("ref") { body["ref"] = r.clone(); }
            if let Some(n) = args.get("name") { body["name"] = n.clone(); }
            if let Some(r) = args.get("role") { body["role"] = r.clone(); }
            if let Some(s) = args.get("selector") { body["selector"] = s.clone(); }
            if let Some(t) = args.get("text") { body["text"] = t.clone(); }
            if let Some(t) = args.get("tab_id") { body["tab_id"] = t.clone(); }
            if let Some(b) = args.get("browser") { body["browser"] = b.clone(); }
            client.post("/v1/browser/wait", body).await?
        }
        "vcu_browser_tabs" => client.get("/v1/browser/tabs").await?,
        "vcu_browser_select" => client.post("/v1/browser/select", args).await?,
        "vcu_browser_close" => client.post("/v1/browser/close", args).await?,
        "vcu_browser_open" => client.post("/v1/browser/open", args).await?,
        "vcu_browser_group" => client.post("/v1/browser/group", args).await?,
        "vcu_browser_group_update" => client.post("/v1/browser/group/update", args).await?,
        "vcu_browser_ungroup" => client.post("/v1/browser/ungroup", args).await?,
        "vcu_browser_ping" => client.post("/v1/browser/ping", json!({
            "reload": args.get("reload").and_then(|v| v.as_bool()).unwrap_or(false)
        })).await?,
        "vcu_browser_extract" => {
            let mut body = json!({
                "selector": args.get("selector").and_then(|v| v.as_str()).unwrap_or("a")
            });
            if let Some(t) = args.get("tab_id").and_then(|v| v.as_str()) {
                body["tab_id"] = json!(t);
            }
            if let Some(b) = args.get("browser").and_then(|v| v.as_str()) {
                body["browser"] = json!(b);
            }
            client.post("/v1/browser/extract", body).await?
        }
        "vcu_session_start" => {
            let browser = args.get("browser").and_then(|v| v.as_str()).unwrap_or("auto");
            let mut body = json!({"browser": browser});
            if let Some(surface) = args.get("surface") {
                body["surface"] = surface.clone();
            } else if args.get("backend").is_none() {
                body["surface"] = json!("desktop");
            }
            if let Some(backend) = args.get("backend") {
                body["backend"] = backend.clone();
            }
            if let Some(vp) = args.get("vision_policy") {
                body["vision_policy"] = vp.clone();
            }
            if let Some(id) = args.get("app_id") {
                body["app_id"] = id.clone();
            }
            client.post("/v1/session/start", body).await?
        }
        "vcu_session_list" => client.get("/v1/session/list").await?,
        "vcu_session_abort" => {
            let sid = req_str(&args, "session")?;
            client
                .post(&format!("/v1/session/{sid}/abort"), json!({}))
                .await?
        }
        "vcu_session_stop" => {
            let sid = req_str(&args, "session")?;
            if sid == "all" {
                let listed = client.get("/v1/session/list").await?;
                let mut stopped = Vec::new();
                if let Some(arr) = listed.get("data").and_then(|v| v.as_array()) {
                    for s in arr {
                        if let Some(id) = s.get("session_id").and_then(|v| v.as_str()) {
                            let v = client
                                .post(&format!("/v1/session/{id}/stop"), json!({}))
                                .await?;
                            stopped.push(json!({"session_id": id, "ok": v.get("ok")}));
                        }
                    }
                }
                json!({"ok": true, "data": {"count": stopped.len(), "stopped": stopped}})
            } else {
                client.post(&format!("/v1/session/{sid}/stop"), json!({})).await?
            }
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
            let mut body = json!({"mode": mode, "budget": budget, "force_vision": force_vision});
            if let Some(t) = args.get("tab_id") {
                body["tab_id"] = t.clone();
            }
            client
                .post(
                    &format!("/v1/session/{sid}/snapshot"),
                    body,
                )
                .await?
        }
        "vcu_hover" => {
            let sid = req_str(&args, "session")?;
            let r = req_str(&args, "ref")?;
            let mut target = json!({"ref": r});
            let mut act_args = json!({});
            if let Some(t) = args.get("tab_id") {
                target["tab_id"] = t.clone();
                act_args["tab_id"] = t.clone();
            }
            client
                .post(
                    &format!("/v1/session/{sid}/act"),
                    json!({"type":"hover","target": target, "args": act_args}),
                )
                .await?
        }
        "vcu_click" => {
            let sid = req_str(&args, "session")?;
            if args.get("pixel_x").is_some() {
                let mut target = json!({});
                let mut act_args = json!({});
                if let Some(r) = args.get("ref") {
                    target["ref"] = r.clone();
                }
                act_args["pixel_x"] = args.get("pixel_x").cloned().unwrap();
                if let Some(y) = args.get("pixel_y") {
                    act_args["pixel_y"] = y.clone();
                }
                if let Some(sp) = args.get("space") {
                    act_args["space"] = sp.clone();
                }
                if let Some(t) = args.get("tab_id") {
                    target["tab_id"] = t.clone();
                    act_args["tab_id"] = t.clone();
                }
                client
                    .post(
                        &format!("/v1/session/{sid}/act"),
                        json!({"type":"click","target": target, "args": act_args}),
                    )
                    .await?
            } else {
                let r = req_str(&args, "ref")?;
                let mut body = json!({"ref": r});
                if let Some(t) = args.get("tab_id") {
                    body["tab_id"] = t.clone();
                }
                client
                    .post(&format!("/v1/session/{sid}/click"), body)
                    .await?
            }
        }
        "vcu_type" => {
            let sid = req_str(&args, "session")?;
            let text = req_str(&args, "text")?;
            let mut body = json!({"text": text});
            if let Some(r) = args.get("ref") {
                body["ref"] = r.clone();
            }
            if let Some(t) = args.get("tab_id") {
                body["tab_id"] = t.clone();
            }
            client.post(&format!("/v1/session/{sid}/type"), body).await?
        }
        "vcu_extract" => {
            let sid = req_str(&args, "session")?;
            let selector = req_str(&args, "selector")?;
            let mut body = json!({"selector": selector});
            if let Some(t) = args.get("tab_id") {
                body["tab_id"] = t.clone();
            }
            client
                .post(
                    &format!("/v1/session/{sid}/extract"),
                    body,
                )
                .await?
        }
        "vcu_scroll" => {
            let sid = req_str(&args, "session")?;
            let dy = args.get("dy").and_then(|v| v.as_i64()).unwrap_or(600);
            let mut args_body = json!({"dy": dy});
            let mut target = json!({});
            if let Some(t) = args.get("tab_id") {
                args_body["tab_id"] = t.clone();
                target["tab_id"] = t.clone();
            }
            client
                .post(
                    &format!("/v1/session/{sid}/act"),
                    json!({"type":"scroll","target": target,"args": args_body}),
                )
                .await?
        }
        "vcu_screenshot" => {
            let sid = req_str(&args, "session")?;
            let full_page = args.get("full_page").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut body = json!({"full_page": full_page});
            if let Some(t) = args.get("tab_id") {
                body["tab_id"] = t.clone();
            }
            client
                .post(
                    &format!("/v1/session/{sid}/screenshot"),
                    body,
                )
                .await?
        }
        "vcu_wait" => {
            let sid = req_str(&args, "session")?;
            let ms = args.get("ms").and_then(|v| v.as_u64()).unwrap_or(200);
            let mut target = json!({});
            if let Some(r) = args.get("ref") {
                target["ref"] = r.clone();
            }
            let mut a = json!({"ms": ms});
            if let Some(n) = args.get("name") {
                a["name"] = n.clone();
            }
            if let Some(r) = args.get("role") {
                a["role"] = r.clone();
            }
            if let Some(v) = args.get("value") {
                a["value"] = v.clone();
            }
            if let Some(t) = args.get("tab_id") {
                a["tab_id"] = t.clone();
                target["tab_id"] = t.clone();
            }
            client
                .post(
                    &format!("/v1/session/{sid}/act"),
                    json!({"type":"wait","target": target, "args": a}),
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
            let pixels = args.get("pixels").and_then(|v| v.as_bool()).unwrap_or(false);
            let mut body = json!({"id": id, "budget": budget, "pixels": pixels});
            if let Some(sel) = args.get("selector") {
                body["selector"] = sel.clone();
            }
            client
                .post("/v1/app/snapshot", body)
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
    Ok(body)
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

#[cfg(test)]
mod vision_handoff_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn mcp_vision_content_attaches_png() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("scene.png");
        std::fs::write(&p, b"\x89PNG\r\n").unwrap();
        let v = json!({"ok": true, "data": {"screenshot_path": p.to_string_lossy()}});
        let c = mcp_vision_content(&v);
        assert_eq!(c[0]["type"], "text");
        assert_eq!(c.len(), 2);
        assert_eq!(c[1]["type"], "image");
        assert_eq!(c[1]["mimeType"], "image/png");
        assert!(c[1]["data"].as_str().unwrap().len() > 4);
    }

    #[test]
    fn mcp_vision_content_without_png_is_text_only() {
        let v = json!({"ok": true, "data": {"hello": "world"}});
        let c = mcp_vision_content(&v);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0]["type"], "text");
    }
}

#[cfg(test)]
mod desktop_host_tools_tests {
    use super::tool_defs;

    #[test]
    fn mcp_lists_desktop_hover_wait_value_and_abort() {
        let tools = tool_defs();
        let names: Vec<String> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()))
            .collect();
        assert!(names.contains(&"vcu_hover".into()), "{names:?}");
        assert!(names.contains(&"vcu_session_abort".into()), "{names:?}");
        assert!(names.contains(&"vcu_wait".into()), "{names:?}");
        let wait = tools.iter().find(|t| t["name"] == "vcu_wait").unwrap();
        let props = &wait["inputSchema"]["properties"];
        assert!(props.get("value").is_some(), "{wait}");
        let hover = tools.iter().find(|t| t["name"] == "vcu_hover").unwrap();
        assert!(hover["description"].as_str().unwrap().contains("Never OS cursor"));
    }

    #[test]
    fn mcp_tools_list_frame_uses_content_length() {
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": { "tools": super::tool_defs() }
        })
        .to_string();
        let mut buf = Vec::new();
        super::write_message(&mut buf, &body).unwrap();
        let framed = String::from_utf8(buf).unwrap();
        assert!(
            framed.starts_with("Content-Length: "),
            "{}",
            &framed[..framed.len().min(80)]
        );
        assert!(framed.contains("\r\n\r\n"), "missing header break");
        assert!(framed.contains("vcu_hover"));
        assert!(framed.contains("vcu_session_abort"));
        assert!(framed.contains("vcu_wait"));
    }
}
