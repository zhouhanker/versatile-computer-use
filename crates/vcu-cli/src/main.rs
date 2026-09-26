use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use clap::{Parser, Subcommand, ValueEnum};
use serde_json::{json, Value};
use vcu_core::{
    Envelope, ErrorCode, ModelConfig, VcuError, VcuPaths, VisionPolicy,
};

#[derive(Parser, Debug)]
#[command(name = "vcu", version, about = "Versatile Computer Use CLI")]
struct Cli {
    /// User config directory (default: ~/.vcu)
    #[arg(long, global = true, env = "VCU_DIR")]
    user_dir: Option<PathBuf>,

    /// Always print JSON envelopes
    #[arg(long, global = true, default_value_t = false)]
    json: bool,

    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Initialize ~/.vcu layout and pairing token
    Init {
        #[command(subcommand)]
        sub: Option<InitCmd>,
    },
    /// Diagnose local setup
    Doctor,
    /// Manage daemon
    Daemon {
        #[command(subcommand)]
        sub: DaemonCmd,
    },
    /// Model configuration
    Model {
        #[command(subcommand)]
        sub: ModelCmd,
    },
    /// Sessions
    Session {
        #[command(subcommand)]
        sub: SessionCmd,
    },
    /// Tabs
    Tabs {
        #[command(subcommand)]
        sub: TabsCmd,
    },
    Snapshot {
        #[arg(long)]
        session: String,
        #[arg(long, default_value = "a11y")]
        mode: String,
        #[arg(long, default_value_t = 4000)]
        budget: u64,
        #[arg(long, default_value_t = false)]
        force_vision: bool,
        #[arg(long)]
        tab: Option<String>,
    },
    Navigate {
        #[arg(long)]
        session: String,
        #[arg(long)]
        url: String,
        #[arg(long)]
        tab: Option<String>,
    },
    Click {
        #[arg(long)]
        session: String,
        #[arg(long = "ref")]
        target_ref: Option<String>,
        #[arg(long)]
        pixel_x: Option<f64>,
        #[arg(long)]
        pixel_y: Option<f64>,
        #[arg(long, default_value = "window")]
        space: String,
        #[arg(long)]
        tab: Option<String>,
    },
    Type {
        #[arg(long)]
        session: String,
        #[arg(long)]
        text: String,
        #[arg(long = "ref")]
        target_ref: Option<String>,
        #[arg(long)]
        tab: Option<String>,
    },
    Extract {
        #[arg(long)]
        session: String,
        #[arg(long)]
        selector: String,
        #[arg(long)]
        tab: Option<String>,
    },
    Screenshot {
        #[arg(long)]
        session: String,
        #[arg(long)]
        tab: Option<String>,
        #[arg(long, default_value_t = false)]
        full_page: bool,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    Act {
        #[arg(long)]
        session: String,
        #[arg(long)]
        action_json: PathBuf,
    },
    Scroll {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = 600)]
        dy: i64,
    },
    /// Wait ms, or until a desktop Scene ref/name/role/value appears. Does not move the OS cursor.
    Wait {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = 200)]
        ms: u64,
        #[arg(long = "ref")]
        target_ref: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        value: Option<String>,
    },
    Agent {
        #[command(subcommand)]
        sub: AgentCmd,
    },
    /// Install skill files for agent harnesses
    InstallSkill {
        #[arg(long, value_enum, default_value_t = Harness::Generic)]
        harness: Harness,
        #[arg(long)]
        dest: Option<PathBuf>,
    },
    Mcp {
        #[command(subcommand)]
        sub: McpCmd,
    },
    /// Edit local config fields
    Config {
        #[command(subcommand)]
        sub: ConfigCmd,
    },
    /// Desktop app computer-use (macOS AX / future Windows UIA)
    App {
        #[command(subcommand)]
        sub: AppCmd,
    },
    /// macOS LaunchAgent / service helpers
    Service {
        #[command(subcommand)]
        sub: ServiceCmd,
    },
    /// Install / update / uninstall this tool on the machine
    #[command(name = "self")]
    SelfCmdRoot {
        #[command(subcommand)]
        sub: SelfCmd,
    },
    /// Discover attachable browsers (CDP ports)
    Browser {
        #[command(subcommand)]
        sub: BrowserCmd,
    },
}

#[derive(Subcommand, Debug)]
enum InitCmd {
    /// Configure vision/chat model
    Model {
        #[arg(long, default_value = "vision")]
        name: String,
        #[arg(long, default_value = "openai-compatible")]
        provider: String,
        #[arg(long, default_value = "https://api.openai.com/v1")]
        base_url: String,
        #[arg(long, default_value = "gpt-4o-mini")]
        model: String,
        #[arg(long, default_value = "VCU_VISION_API_KEY")]
        api_key_env: String,
        #[arg(long, default_value = "vision")]
        kind: String,
    },
}

#[derive(Subcommand, Debug)]
enum DaemonCmd {
    Start {
        #[arg(long, default_value_t = false)]
        foreground: bool,
    },
    Stop,
    Status,
}

#[derive(Subcommand, Debug)]
enum ModelCmd {
    List,
    Set {
        name: String,
        #[arg(long, default_value = "openai-compatible")]
        provider: String,
        #[arg(long)]
        base_url: String,
        #[arg(long)]
        model: String,
        #[arg(long, default_value = "VCU_VISION_API_KEY")]
        api_key_env: String,
        #[arg(long, default_value = "vision")]
        kind: String,
        #[arg(long, default_value_t = true)]
        set_default: bool,
    },
    Test {
        name: String,
    },
    SetPolicy {
        #[arg(long)]
        mode: String,
    },
}

#[derive(Subcommand, Debug)]
enum SessionCmd {
    Start {
        #[arg(long, default_value = "auto")]
        browser: String,
        #[arg(long)]
        backend: Option<String>,
        #[arg(long)]
        surface: Option<String>,
        #[arg(long)]
        vision_policy: Option<String>,
        /// Desktop window id, e.g. proc:Microsoft_Edge:123
        #[arg(long)]
        app_id: Option<String>,
    },
    List,
    Show {
        id: String,
    },
    Stop {
        id: String,
    },
    /// End a desktop session via Stage abort (same path as Escape). Tears down HUD.
    Abort {
        id: String,
    },
    Checkpoint {
        id: String,
    },
    RequestHelp {
        id: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand, Debug)]
enum TabsCmd {
    List {
        #[arg(long)]
        session: String,
    },
    Borrow {
        #[arg(long)]
        session: String,
        #[arg(long)]
        tab: String,
    },
    Return {
        #[arg(long)]
        session: String,
        #[arg(long)]
        tab: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AgentCmd {
    Blackboard {
        #[arg(long)]
        session: String,
    },
    SpawnVision {
        #[arg(long)]
        session: String,
    },
}

#[derive(Subcommand, Debug)]
enum McpCmd {
    PrintConfig,
}

#[derive(Subcommand, Debug)]
enum ServiceCmd {
    /// Install macOS LaunchAgent for vcu-daemon
    Install,
    /// Remove macOS LaunchAgent
    Uninstall,
    Status,
}

#[derive(Subcommand, Debug)]
enum SelfCmd {
    /// Show where binaries/config/share are installed
    Info,
    /// Re-download and replace binaries (keeps ~/.vcu config by default)
    Update {
        #[arg(long, default_value = "latest")]
        version: String,
        #[arg(long, env = "VCU_BASE_URL")]
        base_url: Option<String>,
        #[arg(long, env = "VCU_PREFIX", default_value_t = default_prefix())]
        prefix: String,
    },
    /// Remove installed binaries, share bundle, and optional LaunchAgent
    Uninstall {
        /// Also delete ~/.vcu user config/state
        #[arg(long, default_value_t = false)]
        purge_config: bool,
        #[arg(long, env = "VCU_PREFIX", default_value_t = default_prefix())]
        prefix: String,
        /// Required safety gate
        #[arg(long)]
        yes: bool,
    },
}

fn default_prefix() -> String {
    dirs::home_dir()
        .map(|h| h.join(".local").display().to_string())
        .unwrap_or_else(|| ".".into())
}

#[derive(Subcommand, Debug)]
enum BrowserCmd {
    /// Scan localhost CDP endpoints / report attach strategy
    Discover {
        #[arg(long, default_value = "9222-9335")]
        ports: String,
    },
    /// Report user vs Agent browser profiles (login-state path)
    #[command(name = "login-state")]
    LoginState,
    /// Print only the next login-state action
    Next,
    /// Copy VCU extension into ~/.vcu/lens-extension and print load-unpacked steps (no UI clicks)
    #[command(name = "install-lens")]
    InstallLens {
        /// After copying, chrome.runtime.reload every connected Edge/Chrome lens. Never clicks Allow.
        #[arg(long, default_value_t = false)]
        reload: bool,
        /// Extension source directory containing manifest.json.
        #[arg(long)]
        from: Option<String>,
    },
    /// Observe USER Chrome/Edge without Stage HUD. Without --tab, frontmost must be Chrome/Edge. Stamps tab_id; later click/screenshot/open bind it for 60s.
    Observe {
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        pixels: bool,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long, default_value_t = 2500)]
        budget: u64,
        /// Capture this USER tab. Activates it in-window without stealing OS frontmost.
        #[arg(long)]
        tab: Option<String>,
        /// chrome or edge. Disambiguates colliding tab ids; without --tab observes that browser's focused tab.
        #[arg(long)]
        browser: Option<String>,
    },
    /// Capture a USER tab viewport PNG. Without --tab, uses last observe tab for 60s.
    Screenshot {
        #[arg(long)]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
    },
    /// Click viewport pixels (--capture) or a unique DOM selector. Without --tab, selector clicks bind last observe for 60s.
    Click {
        #[arg(long)]
        pixel_x: Option<f64>,
        #[arg(long)]
        pixel_y: Option<f64>,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long)]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
        #[arg(long, default_value = "window")]
        space: String,
        #[arg(long)]
        capture: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        guide: bool,
    },
    /// Hover a unique DOM selector via the USER extension (synthetic, trusted=false).
    Hover {
        #[arg(long)]
        selector: String,
        #[arg(long)]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Type into USER browser. Default: AX address bar. `--selector` uses USER extension DOM; a native `<select>` is set by option value or label (input_path=dom_select).
    Type {
        #[arg(long)]
        text: Option<String>,
        #[arg(long = "ref")]
        target_ref: Option<String>,
        #[arg(long)]
        selector: Option<String>,
        #[arg(long, requires = "selector")]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Scroll a USER browser tab through the extension, or AX when disconnected.
    Scroll {
        #[arg(long)]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
        #[arg(long, default_value_t = 600)]
        dy: i32,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
    },
    /// Policy-gated key (never HID). Return requires confirm_send + Send ref.
    Key {
        #[arg(long)]
        key: String,
        #[arg(long, default_value_t = false)]
        dry_run: bool,
        #[arg(long, default_value_t = false)]
        confirm_send: bool,
        #[arg(long = "ref")]
        target_ref: Option<String>,
    },
    /// Open http(s) URL as a background tab in the VCU tab group. Does not replace the user page or focus the window.
    Open {
        #[arg(long)]
        url: String,
        #[arg(long, conflicts_with = "group")]
        session_name: Option<String>,
        #[arg(long)]
        group: Option<String>,
        #[arg(long)]
        background: bool,
        /// Open a separate USER-profile window, retaining its login state.
        #[arg(long, conflicts_with = "group")]
        new_window: bool,
        /// chrome or edge; default last observe
        #[arg(long)]
        browser: Option<String>,
    },
    /// List USER browser tabs and native groups.
    Tabs,
    /// Select a tab, expand its group and focus its browser window.
    Select {
        #[arg(long)]
        tab: String,
        /// chrome or edge when tab_id collides
        #[arg(long)]
        browser: Option<String>,
    },
    /// Close exactly the named tab. No default or fallback target.
    Close {
        #[arg(long)]
        tab: String,
        /// chrome or edge when tab_id collides
        #[arg(long)]
        browser: Option<String>,
    },
    /// Group explicitly selected tabs in their current window.
    Group {
        #[arg(long, value_delimiter = ',', required = true, num_args = 1..)]
        tabs: Vec<String>,
        #[arg(long)]
        title: String,
        #[arg(long, default_value = "purple")]
        color: String,
        #[arg(long)]
        collapsed: bool,
        #[arg(long)]
        browser: Option<String>,
    },
    /// Rename, recolor, expand or collapse a native group.
    GroupUpdate {
        #[arg(long)]
        group: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        color: Option<String>,
        #[arg(long, action = clap::ArgAction::Set)]
        collapsed: Option<bool>,
        #[arg(long)]
        browser: Option<String>,
    },
    /// Remove explicit tabs from groups without closing them.
    Ungroup {
        #[arg(long, value_delimiter = ',', required = true, num_args = 1..)]
        tabs: Vec<String>,
        #[arg(long)]
        browser: Option<String>,
    },
    /// Ping USER Edge extension (no page script)
    Ping {
        /// chrome.runtime.reload unpacked SW (picks up lens files). Never clicks Allow.
        #[arg(long, default_value_t = false)]
        reload: bool,
    },
    /// DOM extract via USER Edge extension (no session HUD, no Agent Edge)
    Extract {
        #[arg(long, default_value = "a")]
        selector: String,
        #[arg(long)]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
    },
    /// Wait ms, until a DOM selector/text appears, or until a Scene name/role/ref appears (no HUD)
    Wait {
        #[arg(long, default_value_t = 200)]
        ms: u64,
        #[arg(long = "ref")]
        target_ref: Option<String>,
        #[arg(long)]
        name: Option<String>,
        #[arg(long)]
        role: Option<String>,
        /// CSS selector; polls USER extension DOM (binds last observe for 60s)
        #[arg(long)]
        selector: Option<String>,
        /// Optional substring of extract text/value
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        tab: Option<String>,
        #[arg(long)]
        browser: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum AppCmd {
    Windows,
    Snapshot {
        id: String,
        #[arg(long, default_value_t = 4000)]
        budget: u64,
        #[arg(long, default_value_t = false)]
        pixels: bool,
        #[arg(long)]
        selector: Option<String>,
    },
    Invoke {
        id: String,
        #[arg(long = "ref")]
        target_ref: String,
    },
    Focus {
        id: String,
        #[arg(long, default_value_t = false)]
        allow_focus_steal: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCmd {
    /// Show config.json
    Show,
    /// Set CDP endpoint URL
    SetCdp {
        url: String,
    },
    /// Set daemon bind port
    SetPort {
        port: u16,
    },
    /// Set app CU allowlist (comma-separated process name substrings)
    SetAppAllowlist {
        /// e.g. TextEdit,Notes,Safari
        list: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum Harness {
    Generic,
    Codex,
    Claude,
    Cursor,
    Pi,
}

fn main() {
    let started = std::thread::Builder::new()
        .name("vcu-main".into())
        .stack_size(8 * 1024 * 1024)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            runtime.block_on(cli_main());
        });
    match started {
        Ok(handle) => {
            if handle.join().is_err() {
                std::process::exit(2);
            }
        }
        Err(err) => {
            eprintln!("failed to start vcu: {err}");
            std::process::exit(2);
        }
    }
}

async fn cli_main() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();

    let cli = Cli::parse();
    let paths = match cli.user_dir.clone() {
        Some(p) => VcuPaths::from_root(p),
        None => match VcuPaths::default_user() {
            Ok(p) => p,
            Err(e) => {
                print_err(&e, true);
                std::process::exit(2);
            }
        },
    };

    let result = run(cli, paths).await;
    match result {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            print_err(&e, true);
            std::process::exit(map_exit(&e));
        }
    }
}

fn map_exit(e: &VcuError) -> i32 {
    match e.code() {
        ErrorCode::DaemonAuthFailed => 3,
        ErrorCode::BorrowRequired
        | ErrorCode::OsCursorDenied
        | ErrorCode::VisionProviderRequired
        | ErrorCode::AccessibilityDenied
        | ErrorCode::AppDenied => 4,
        ErrorCode::SessionNotFound | ErrorCode::TabNotFound | ErrorCode::ModelNotFound => 5,
        _ => 1,
    }
}

fn print_err(e: &VcuError, json: bool) {
    let env = Envelope::<Value>::from_error(e);
    if json {
        println!("{}", serde_json::to_string_pretty(&env).unwrap_or_default());
    } else {
        eprintln!("error: {:?} — {}", e.code(), e.message());
        eprintln!("hint: {}", e.repair_hint());
    }
}

fn print_ok<T: serde::Serialize>(val: &T, as_json: bool) {
    if as_json {
        let env = Envelope::ok(val);
        println!("{}", serde_json::to_string_pretty(&env).unwrap_or_default());
    } else {
        println!("{}", serde_json::to_string_pretty(val).unwrap_or_default());
    }
}

async fn run(cli: Cli, paths: VcuPaths) -> Result<i32, VcuError> {
    let json = cli.json;
    match cli.cmd {
        Commands::Init { sub } => {
            let mut cfg = paths.init_if_needed()?;
            match sub {
                None => {
                    print_ok(
                        &json!({
                            "user_dir": paths.root,
                            "endpoint": VcuPaths::endpoint_url(&cfg),
                            "pairing_token_set": !cfg.pairing_token.is_empty(),
                            "next": [
                                "vcu daemon start",
                                "vcu browser install-lens   # load unpacked in USER Chrome/Edge",
                                "vcu browser ping --json",
                                "vcu browser observe --json  # view PNG; 60s click/screenshot/open bind last observe"
                            ]
                        }),
                        json,
                    );
                }
                Some(InitCmd::Model {
                    name,
                    provider,
                    base_url,
                    model,
                    api_key_env,
                    kind,
                }) => {
                    cfg.models.insert(
                        name.clone(),
                        ModelConfig {
                            name: name.clone(),
                            provider,
                            base_url,
                            model,
                            api_key_env,
                            kind,
                        },
                    );
                    cfg.default_vision_model = Some(name.clone());
                    paths.save_config(&cfg)?;
                    print_ok(&json!({"saved": name, "default_vision_model": name}), json);
                }
            }
            Ok(0)
        }
        Commands::Doctor => {
            // prefer live daemon doctor
            match api_get(&paths, "/v1/doctor").await {
                Ok(v) => {
                    println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                    let ok = v.pointer("/data/ok").and_then(|x| x.as_bool()).unwrap_or(false);
                    Ok(if ok { 0 } else { 1 })
                }
                Err(_) => {
                    let report = vcu_server::doctor::build_report(&paths, None).await;
                    print_ok(&report, true);
                    Ok(if report.ok { 0 } else { 1 })
                }
            }
        }
        Commands::Daemon { sub } => match sub {
            DaemonCmd::Start { foreground } => daemon_start(&paths, foreground).await,
            DaemonCmd::Stop => daemon_stop(&paths),
            DaemonCmd::Status => daemon_status(&paths).await,
        },
        Commands::Model { sub } => model_cmd(&paths, sub, json).await,
        Commands::Session { sub } => session_cmd(&paths, sub, json).await,
        Commands::Tabs { sub } => tabs_cmd(&paths, sub, json).await,
        Commands::Snapshot {
            session,
            mode,
            budget,
            force_vision,
            tab,
        } => {
            let mut body = json!({"mode": mode, "budget": budget, "force_vision": force_vision});
            if let Some(t) = tab {
                body["tab_id"] = json!(t);
            }
            let v = api_post(&paths, &format!("/v1/session/{session}/snapshot"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Navigate { session, url, tab } => {
            let mut body = json!({"url": url});
            if let Some(t) = tab {
                body["tab_id"] = json!(t);
            }
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/navigate"),
                body,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Click {
            session,
            target_ref,
            pixel_x,
            pixel_y,
            space,
            tab,
        } => {
            let v = if let (Some(px), Some(py)) = (pixel_x, pixel_y) {
                let mut target = json!({});
                let mut args = json!({"pixel_x": px, "pixel_y": py, "space": space});
                if let Some(r) = &target_ref {
                    target["ref"] = json!(r);
                }
                if let Some(t) = &tab {
                    target["tab_id"] = json!(t);
                    args["tab_id"] = json!(t);
                }
                api_post(
                    &paths,
                    &format!("/v1/session/{session}/act"),
                    json!({"type":"click","target": target, "args": args}),
                )
                .await?
            } else {
                let r = target_ref.ok_or_else(|| {
                    VcuError::coded(
                        ErrorCode::InvalidInput,
                        "click requires --ref or --pixel-x/--pixel-y",
                    )
                })?;
                let mut body = json!({"ref": r});
                if let Some(t) = tab {
                    body["tab_id"] = json!(t);
                }
                api_post(&paths, &format!("/v1/session/{session}/click"), body).await?
            };
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Type {
            session,
            text,
            target_ref,
            tab,
        } => {
            let mut body = json!({"text": text});
            if let Some(r) = target_ref {
                body["ref"] = json!(r);
            }
            if let Some(t) = tab {
                body["tab_id"] = json!(t);
            }
            let v = api_post(&paths, &format!("/v1/session/{session}/type"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Extract { session, selector, tab } => {
            let mut body = json!({"selector": selector});
            if let Some(t) = tab {
                body["tab_id"] = json!(t);
            }
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/extract"),
                body,
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Screenshot {
            session,
            tab,
            full_page,
            out,
        } => {
            let mut body = json!({"full_page": full_page, "out": out});
            if let Some(t) = tab {
                body["tab_id"] = json!(t);
            }
            let v = api_post(&paths, &format!("/v1/session/{session}/screenshot"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Scroll { session, dy } => {
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/act"),
                json!({"type": "scroll", "target": {}, "args": {"dy": dy}}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Wait {
            session,
            ms,
            target_ref,
            name,
            role,
            value,
        } => {
            let mut args = json!({"ms": ms});
            let mut target = json!({});
            if let Some(r) = target_ref {
                args["ref"] = json!(r.clone());
                target["ref"] = json!(r);
            }
            if let Some(n) = name {
                args["name"] = json!(n);
            }
            if let Some(r) = role {
                args["role"] = json!(r);
            }
            if let Some(v) = value {
                args["value"] = json!(v);
            }
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/act"),
                json!({"type": "wait", "target": target, "args": args}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Act {
            session,
            action_json,
        } => {
            let text = fs::read_to_string(action_json)
                .map_err(|e| VcuError::with_detail(ErrorCode::InvalidInput, "read action", e.to_string()))?;
            let body: Value = serde_json::from_str(&text)?;
            let v = api_post(&paths, &format!("/v1/session/{session}/act"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Agent { sub } => match sub {
            AgentCmd::Blackboard { session } => {
                let v = api_get(&paths, &format!("/v1/session/{session}/blackboard")).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            AgentCmd::SpawnVision { session } => {
                print_ok(
                    &json!({
                        "session": session,
                        "instruction": "Use a vision-capable model. Read shared state via `vcu agent blackboard --session <id>`. Do not move the OS cursor. Prefer DOM refs from candidates[]. Write findings back by asking the main agent to `vcu session checkpoint`.",
                        "blackboard_cmd": format!("vcu agent blackboard --session {session} --json")
                    }),
                    true,
                );
                Ok(0)
            }
        },
        Commands::InstallSkill { harness, dest } => install_skill(harness, dest, json),
        Commands::App { sub } => match sub {
            AppCmd::Windows => {
                let v = api_get(&paths, "/v1/app/windows").await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            AppCmd::Snapshot { id, budget, pixels, selector } => {
                let mut body = json!({"id": id, "budget": budget, "pixels": pixels});
                if let Some(sel) = selector {
                    body["selector"] = json!(sel);
                }
                let v = api_post(
                    &paths,
                    "/v1/app/snapshot",
                    body,
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            AppCmd::Invoke { id, target_ref } => {
                let v = api_post(
                    &paths,
                    "/v1/app/invoke",
                    json!({"id": id, "ref": target_ref}),
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            AppCmd::Focus {
                id,
                allow_focus_steal,
            } => {
                let v = api_post(
                    &paths,
                    "/v1/app/focus",
                    json!({"id": id, "allow_focus_steal": allow_focus_steal}),
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
        },
        Commands::Config { sub } => match sub {
            ConfigCmd::Show => {
                let cfg = paths.load_config().or_else(|_| paths.init_if_needed())?;
                print_ok(&cfg, true);
                Ok(0)
            }
            ConfigCmd::SetCdp { url } => {
                let mut cfg = paths.load_config().or_else(|_| paths.init_if_needed())?;
                cfg.cdp_url = Some(url.clone());
                paths.save_config(&cfg)?;
                print_ok(&json!({"cdp_url": url}), true);
                Ok(0)
            }
            ConfigCmd::SetPort { port } => {
                let mut cfg = paths.load_config().or_else(|_| paths.init_if_needed())?;
                cfg.daemon_port = port;
                paths.save_config(&cfg)?;
                print_ok(&json!({"daemon_port": port}), true);
                Ok(0)
            }
            ConfigCmd::SetAppAllowlist { list } => {
                let mut cfg = paths.load_config().or_else(|_| paths.init_if_needed())?;
                cfg.app_allowlist = list
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                paths.save_config(&cfg)?;
                print_ok(&json!({"app_allowlist": cfg.app_allowlist}), true);
                Ok(0)
            }
        },
        Commands::Service { sub } => match sub {
            ServiceCmd::Install => {
                #[cfg(target_os = "macos")]
                {
                    // Prefer in-process installer (finds vcu-daemon next to this binary).
                    install_macos_launch_agent(&paths)?;
                    print_ok(&json!({"installed": true, "label": "com.vcu.daemon"}), true);
                    Ok(0)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(VcuError::coded(ErrorCode::NotImplemented, "service install is macOS-only currently"))
                }
            }
            ServiceCmd::Uninstall => {
                #[cfg(target_os = "macos")]
                {
                    uninstall_macos_launch_agent()?;
                    print_ok(&json!({"removed": true}), true);
                    Ok(0)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(VcuError::coded(ErrorCode::NotImplemented, "service uninstall is macOS-only currently"))
                }
            }
            ServiceCmd::Status => {
                #[cfg(target_os = "macos")]
                {
                    let out = std::process::Command::new("launchctl")
                        .args(["print", &format!("gui/{}/com.vcu.daemon", libc_uid())])
                        .output();
                    let text = out
                        .map(|o| String::from_utf8_lossy(&o.stdout).to_string() + &String::from_utf8_lossy(&o.stderr))
                        .unwrap_or_else(|e| e.to_string());
                    let loaded = text.contains("com.vcu.daemon") && !text.to_lowercase().contains("could not find");
                    print_ok(&json!({"loaded": loaded, "detail": text.chars().take(500).collect::<String>()}), true);
                    Ok(0)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    print_ok(&json!({"loaded": false, "detail": "not macos"}), true);
                    Ok(0)
                }
            }
        },
        Commands::SelfCmdRoot { sub } => match sub {
            SelfCmd::Info => {
                let prefix = default_prefix();
                let bin = PathBuf::from(&prefix).join("bin");
                let share = PathBuf::from(&prefix).join("share/vcu");
                let exe = std::env::current_exe().ok();
                print_ok(
                    &json!({
                        "version": env!("CARGO_PKG_VERSION"),
                        "current_exe": exe,
                        "prefix": prefix,
                        "bin_dir": bin,
                        "share_dir": share,
                        "user_dir": paths.root,
                        "bins_present": {
                            "vcu": bin.join("vcu").exists() || bin.join("vcu.exe").exists(),
                            "vcu-daemon": bin.join("vcu-daemon").exists() || bin.join("vcu-daemon.exe").exists(),
                            "vcu-mcp": bin.join("vcu-mcp").exists() || bin.join("vcu-mcp.exe").exists(),
                            "vcu-stage": bin.join("vcu-stage").exists(),
                        },
                        "extension_dir": share.join("extension"),
                        "note": "Codex Computer Use and other third-party tools are never touched by vcu self uninstall"
                    }),
                    true,
                );
                Ok(0)
            }
            SelfCmd::Update {
                version,
                base_url,
                prefix,
            } => self_update(&version, base_url.as_deref(), &prefix),
            SelfCmd::Uninstall {
                purge_config,
                prefix,
                yes,
            } => {
                if !yes {
                    return Err(VcuError::coded(
                        ErrorCode::InvalidInput,
                        "refusing uninstall without --yes (does not touch Codex Computer Use)",
                    ));
                }
                self_uninstall(&paths, &prefix, purge_config)
            }
        },
        Commands::Browser { sub } => match sub {
            BrowserCmd::Discover { ports } => {
                let report = browser_discover(&ports);
                print_ok(&report, true);
                Ok(0)
            }
            BrowserCmd::InstallLens { reload, from } => {
                let report = install_user_lens(&paths, from.as_deref())?;
                if !reload {
                    print_ok(&report, true);
                    return Ok(0);
                }
                let v = api_post(&paths, "/v1/browser/ping", json!({"reload": true})).await?;
                let mut out = report;
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("reloading".into(), json!(true));
                    obj.insert("reload".into(), v.get("data").cloned().unwrap_or(v.clone()));
                }
                print_ok(&out, true);
                Ok(ok_exit(&v))
            }
            BrowserCmd::LoginState => {
                match api_get(&paths, "/v1/browser/login-state").await {
                    Ok(v) => {
                        println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                        Ok(ok_exit(&v))
                    }
                    Err(e) => {
                        // Daemon down: still classify local processes.
                        let _ = e;
                        let report = local_login_state_fallback();
                        print_ok(&report, true);
                        Ok(0)
                    }
                }
            }
            BrowserCmd::Next => {
                let v = api_get(&paths, "/v1/browser/login-state").await?;
                let action = v
                    .pointer("/data/next_action")
                    .and_then(|x| x.as_str())
                    .unwrap_or("vcu browser observe");
                print_ok(
                    &json!({
                        "next_action": action,
                        "lens_dir": v.pointer("/data/lens_dir").and_then(|x| x.as_str()).unwrap_or(""),
                        "lens_copied": v.pointer("/data/lens_copied").and_then(|x| x.as_bool()).unwrap_or(false),
                        "extension_profile": v.pointer("/data/extension_profile").and_then(|x| x.as_str()).unwrap_or(""),
                        "observe": "vcu browser observe --json",
                        "install_lens": "vcu browser install-lens",
                    }),
                    true,
                );
                Ok(ok_exit(&v))
            }
            BrowserCmd::Observe {
                pixels,
                selector,
                budget,
                tab,
                browser,
            } => {
                let mut body = json!({"pixels": pixels, "budget": budget});
                if let Some(sel) = selector {
                    body["selector"] = json!(sel);
                }
                if let Some(t) = tab {
                    body["tab_id"] = json!(t);
                }
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(&paths, "/v1/browser/observe", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Screenshot { tab, browser } => {
                let mut body = json!({});
                if let Some(t) = tab { body["tab_id"] = json!(t); }
                if let Some(b) = browser { body["browser"] = json!(b); }
                let v = api_post(&paths, "/v1/browser/screenshot", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Click {
                pixel_x,
                pixel_y,
                selector,
                tab,
                browser,
                space,
                capture,
                dry_run,
                guide,
            } => {
                let v = api_post(
                    &paths,
                    "/v1/browser/click",
                    json!({
                        "pixel_x": pixel_x,
                        "pixel_y": pixel_y,
                        "selector": selector,
                        "tab_id": tab,
                        "browser": browser,
                        "space": space,
                        "capture_id": capture,
                        "dry_run": dry_run,
                        "guide": guide,
                    }),
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Hover { selector, tab, browser, dry_run } => {
                let mut body = json!({ "selector": selector, "dry_run": dry_run });
                if let Some(id) = tab {
                    body["tab_id"] = json!(id);
                }
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(&paths, "/v1/browser/hover", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Type {
                text,
                target_ref,
                selector,
                tab,
                browser,
                dry_run,
            } => {
                let mut body = json!({ "dry_run": dry_run });
                if let Some(t) = text {
                    body["text"] = json!(t);
                }
                if let Some(r) = target_ref {
                    body["ref"] = json!(r);
                }
                if let Some(sel) = selector {
                    body["selector"] = json!(sel);
                }
                if let Some(t) = tab { body["tab_id"] = json!(t); }
                if let Some(b) = browser { body["browser"] = json!(b); }
                let v = api_post(&paths, "/v1/browser/type", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Scroll { dy, dry_run, tab, browser } => {
                let mut body = json!({ "dy": dy, "dry_run": dry_run, "tab_id": tab });
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(
                    &paths,
                    "/v1/browser/scroll",
                    body,
                )
                .await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Key {
                key,
                dry_run,
                confirm_send,
                target_ref,
            } => {
                let mut body = json!({
                    "key": key,
                    "dry_run": dry_run,
                    "confirm_send": confirm_send,
                });
                if let Some(r) = target_ref {
                    body["ref"] = json!(r);
                }
                let v = api_post(&paths, "/v1/browser/key", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Open { url, session_name, group, background, new_window, browser } => {
                let _ = background;
                let mut body = json!({"url": url, "active": false, "new_window": new_window});
                if let Some(name) = session_name { body["session_name"] = json!(name); }
                if let Some(id) = group { body["group_id"] = json!(id); }
                if let Some(b) = browser { body["browser"] = json!(b); }
                let v = api_post(&paths, "/v1/browser/open", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Tabs => {
                let v = api_get(&paths, "/v1/browser/tabs").await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Select { tab, browser } => {
                let mut body = json!({"tab_id": tab});
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(&paths, "/v1/browser/select", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Close { tab, browser } => {
                let mut body = json!({"tab_id": tab});
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(&paths, "/v1/browser/close", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Group { tabs, title, color, collapsed, browser } => {
                let mut body = json!({"tab_ids": tabs, "title": title, "color": color, "collapsed": collapsed});
                if let Some(b) = browser { body["browser"] = json!(b); }
                let v = api_post(&paths, "/v1/browser/group", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::GroupUpdate { group, title, color, collapsed, browser } => {
                let mut body = json!({"group_id": group});
                if let Some(v) = title { body["title"] = json!(v); }
                if let Some(v) = color { body["color"] = json!(v); }
                if let Some(v) = collapsed { body["collapsed"] = json!(v); }
                if let Some(b) = browser { body["browser"] = json!(b); }
                let v = api_post(&paths, "/v1/browser/group/update", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Ungroup { tabs, browser } => {
                let mut body = json!({"tab_ids": tabs});
                if let Some(b) = browser { body["browser"] = json!(b); }
                let v = api_post(&paths, "/v1/browser/ungroup", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Ping { reload } => {
                let v = api_post(&paths, "/v1/browser/ping", json!({"reload": reload})).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Extract { selector, tab, browser } => {
                let mut body = json!({ "selector": selector });
                if let Some(t) = tab {
                    body["tab_id"] = json!(t);
                }
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(&paths, "/v1/browser/extract", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
            BrowserCmd::Wait {
                ms,
                target_ref,
                name,
                role,
                selector,
                text,
                tab,
                browser,
            } => {
                let mut body = json!({ "ms": ms });
                if let Some(r) = target_ref {
                    body["ref"] = json!(r);
                }
                if let Some(n) = name {
                    body["name"] = json!(n);
                }
                if let Some(r) = role {
                    body["role"] = json!(r);
                }
                if let Some(s) = selector {
                    body["selector"] = json!(s);
                }
                if let Some(t) = text {
                    body["text"] = json!(t);
                }
                if let Some(t) = tab {
                    body["tab_id"] = json!(t);
                }
                if let Some(b) = browser {
                    body["browser"] = json!(b);
                }
                let v = api_post(&paths, "/v1/browser/wait", body).await?;
                println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
                Ok(ok_exit(&v))
            }
        },
        Commands::Mcp { sub } => match sub {
            McpCmd::PrintConfig => {
                let bin = std::env::current_exe()
                    .ok()
                    .and_then(|p| p.parent().map(|d| d.join("vcu-mcp")))
                    .unwrap_or_else(|| PathBuf::from("vcu-mcp"));
                print_ok(
                    &json!({
                        "mcpServers": {
                            "vcu": {
                                "command": bin,
                                "args": ["--user-dir", paths.root]
                            }
                        }
                    }),
                    true,
                );
                Ok(0)
            }
        },
    }
}

fn ok_exit(v: &Value) -> i32 {
    if v.get("ok").and_then(|x| x.as_bool()).unwrap_or(false) {
        0
    } else {
        1
    }
}

async fn model_cmd(paths: &VcuPaths, sub: ModelCmd, json: bool) -> Result<i32, VcuError> {
    let mut cfg = paths.load_config().or_else(|_| paths.init_if_needed())?;
    match sub {
        ModelCmd::List => {
            print_ok(
                &json!({
                    "default_vision_model": cfg.default_vision_model,
                    "vision_policy": cfg.vision_policy,
                    "models": cfg.models,
                }),
                json,
            );
            Ok(0)
        }
        ModelCmd::Set {
            name,
            provider,
            base_url,
            model,
            api_key_env,
            kind,
            set_default,
        } => {
            cfg.models.insert(
                name.clone(),
                ModelConfig {
                    name: name.clone(),
                    provider,
                    base_url,
                    model,
                    api_key_env,
                    kind,
                },
            );
            if set_default {
                cfg.default_vision_model = Some(name.clone());
            }
            paths.save_config(&cfg)?;
            print_ok(&json!({"saved": name}), json);
            Ok(0)
        }
        ModelCmd::Test { name } => {
            let v = api_post(paths, "/v1/model/test", json!({"name": name})).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        ModelCmd::SetPolicy { mode } => {
            let p = VisionPolicy::parse(&mode).ok_or_else(|| {
                VcuError::coded(ErrorCode::InvalidInput, format!("bad policy {mode}"))
            })?;
            cfg.vision_policy = p;
            paths.save_config(&cfg)?;
            print_ok(&json!({"vision_policy": p}), json);
            Ok(0)
        }
    }
}

async fn session_cmd(paths: &VcuPaths, sub: SessionCmd, _json: bool) -> Result<i32, VcuError> {
    match sub {
        SessionCmd::Start {
            browser,
            backend,
            surface,
            vision_policy,
            app_id,
        } => {
            let (surface, backend) = match (surface, backend) {
                (Some(s), Some(b)) => (s, Some(b)),
                (Some(s), None) if s == "desktop" => (s, Some("desktop".into())),
                (Some(s), None) => (s, Some("mock".into())),
                (None, Some(b)) if b == "desktop" => ("desktop".into(), Some(b)),
                (None, Some(b)) => ("browser_agent".into(), Some(b)),
                (None, None) => ("desktop".into(), Some("desktop".into())),
            };
            let mut body = json!({"browser": browser, "surface": surface});
            if let Some(b) = backend {
                body["backend"] = json!(b);
            }
            if let Some(vp) = vision_policy {
                body["vision_policy"] = json!(vp);
            }
            if let Some(id) = app_id {
                body["app_id"] = json!(id);
            }
            let v = api_post(paths, "/v1/session/start", body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        SessionCmd::List => {
            let v = api_get(paths, "/v1/session/list").await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        SessionCmd::Show { id } => {
            let v = api_get(paths, &format!("/v1/session/{id}")).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        SessionCmd::Stop { id } => {
            if id == "all" {
                let listed = api_get(paths, "/v1/session/list").await?;
                let mut ids = Vec::new();
                if let Some(arr) = listed.get("data").and_then(|v| v.as_array()) {
                    for s in arr {
                        if let Some(sid) = s.get("session_id").and_then(|v| v.as_str()) {
                            ids.push(sid.to_string());
                        }
                    }
                }
                let mut stopped = Vec::new();
                for sid in &ids {
                    let v = api_post(paths, &format!("/v1/session/{sid}/stop"), json!({})).await?;
                    stopped.push(json!({"session_id": sid, "ok": v.get("ok")}));
                }
                print_ok(&json!({"stopped": stopped, "count": stopped.len()}), true);
                return Ok(0);
            }
            let v = api_post(paths, &format!("/v1/session/{id}/stop"), json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        SessionCmd::Abort { id } => {
            let v = api_post(paths, &format!("/v1/session/{id}/abort"), json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        SessionCmd::Checkpoint { id } => {
            let v = api_post(paths, &format!("/v1/session/{id}/checkpoint"), json!({})).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        SessionCmd::RequestHelp { id, reason } => {
            let v = api_post(
                paths,
                &format!("/v1/session/{id}/request-help"),
                json!({"reason": reason}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
    }
}

async fn tabs_cmd(paths: &VcuPaths, sub: TabsCmd, _json: bool) -> Result<i32, VcuError> {
    match sub {
        TabsCmd::List { session } => {
            let v = api_get(paths, &format!("/v1/session/{session}/tabs")).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        TabsCmd::Borrow { session, tab } => {
            let v = api_post(
                paths,
                &format!("/v1/session/{session}/tabs/borrow"),
                json!({"tab_id": tab}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        TabsCmd::Return { session, tab } => {
            let body = if let Some(t) = tab {
                json!({"tab_id": t})
            } else {
                json!({})
            };
            let v = api_post(paths, &format!("/v1/session/{session}/tabs/return"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
    }
}

fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    Command::new("kill")
        .args(["-0", &pid.to_string()])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

fn read_daemon_pid(paths: &VcuPaths) -> Option<u32> {
    fs::read_to_string(paths.pid_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

async fn daemon_start(paths: &VcuPaths, foreground: bool) -> Result<i32, VcuError> {
    let cfg = paths.init_if_needed()?;
    if api_get(paths, "/v1/health").await.is_ok() {
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "already_running": true,
                "pid": read_daemon_pid(paths),
                "endpoint": VcuPaths::endpoint_url(&cfg)
            }))
            .unwrap()
        );
        return Ok(0);
    }
    if let Some(pid) = read_daemon_pid(paths) {
        if pid_alive(pid) {
            return Err(VcuError::with_detail(
                ErrorCode::DaemonAlreadyRunning,
                "vcu-daemon pid is alive but health check failed",
                format!("pid={pid}"),
            ));
        }
        let _ = fs::remove_file(paths.pid_path());
        let _ = fs::remove_file(paths.lock_path());
        let _ = fs::remove_file(paths.endpoint_path());
    }
    if foreground {
        // run server in this process
        let handle = vcu_server::start_daemon(paths.clone(), cfg).await?;
        println!(
            "{}",
            serde_json::to_string_pretty(&json!({
                "ok": true,
                "endpoint": format!("http://{}", handle.addr),
                "pid": std::process::id()
            }))
            .unwrap()
        );
        handle.join.await.ok();
        return Ok(0);
    }
    // spawn vcu-daemon
    let daemon_bin = std::env::current_exe()
        .ok()
        .and_then(|p| {
            let d = p.parent()?.join("vcu-daemon");
            if d.exists() {
                Some(d)
            } else {
                None
            }
        })
        .or_else(|| which("vcu-daemon"))
        .unwrap_or_else(|| PathBuf::from("vcu-daemon"));
    let mut cmd = Command::new(&daemon_bin);
    cmd.arg("--user-dir").arg(&paths.root);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    // Detach from caller TTY/process group so SIGHUP on shell exit
    // does not kill the long-running daemon.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let child = cmd.spawn().map_err(|e| {
        VcuError::with_detail(
            ErrorCode::DaemonNotRunning,
            "failed to spawn vcu-daemon",
            format!("{e}; bin={daemon_bin:?}; try `cargo build -p vcu-daemon` or `vcu daemon start --foreground`"),
        )
    })?;
    // wait until health
    for _ in 0..30 {
        tokio::time::sleep(Duration::from_millis(100)).await;
        if api_get(paths, "/v1/health").await.is_ok() {
            println!(
                "{}",
                serde_json::to_string_pretty(&json!({
                    "ok": true,
                    "pid": child.id(),
                    "endpoint": VcuPaths::endpoint_url(&paths.load_config()?)
                }))
                .unwrap()
            );
            return Ok(0);
        }
    }
    Err(VcuError::coded(
        ErrorCode::DaemonNotRunning,
        "daemon spawned but health check failed",
    ))
}

fn which(bin: &str) -> Option<PathBuf> {
    std::env::var_os("PATH").and_then(|paths| {
        for p in std::env::split_paths(&paths) {
            let cand = p.join(bin);
            if cand.is_file() {
                return Some(cand);
            }
        }
        None
    })
}

fn daemon_stop(paths: &VcuPaths) -> Result<i32, VcuError> {
    if let Ok(pid_s) = fs::read_to_string(paths.pid_path()) {
        if let Ok(pid) = pid_s.trim().parse::<i32>() {
            let _ = Command::new("kill").arg(pid.to_string()).status();
            let _ = fs::remove_file(paths.pid_path());
            let _ = fs::remove_file(paths.lock_path());
            let _ = fs::remove_file(paths.endpoint_path());
            println!("{}", serde_json::to_string_pretty(&json!({"ok": true, "stopped": pid})).unwrap());
            return Ok(0);
        }
    }
    Err(VcuError::coded(
        ErrorCode::DaemonNotRunning,
        "no daemon pid file",
    ))
}

async fn daemon_status(paths: &VcuPaths) -> Result<i32, VcuError> {
    match api_get(paths, "/v1/health").await {
        Ok(v) => {
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(0)
        }
        Err(e) => Err(e),
    }
}

fn install_skill(harness: Harness, dest: Option<PathBuf>, json: bool) -> Result<i32, VcuError> {
    let content = include_str!("../../../skills/generic/SKILL.md");
    let dest = dest.unwrap_or_else(|| match harness {
        Harness::Codex => PathBuf::from(".agents/skills/vcu"),
        Harness::Claude => PathBuf::from(".claude/skills/vcu"),
        Harness::Cursor => PathBuf::from(".cursor/skills/vcu"),
        Harness::Pi => PathBuf::from(".pi/skills/vcu"),
        Harness::Generic => PathBuf::from("skills/generic"),
    });
    fs::create_dir_all(&dest).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "mkdir skill", e.to_string())
    })?;
    let path = dest.join("SKILL.md");
    fs::write(&path, content).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "write skill", e.to_string())
    })?;
    print_ok(&json!({"written": path}), json);
    Ok(0)
}

async fn client(paths: &VcuPaths) -> Result<(reqwest::Client, String, String), VcuError> {
    let cfg = paths.load_config().map_err(|_| {
        VcuError::coded(ErrorCode::InvalidInput, "run `vcu init` first")
    })?;
    let endpoint = if paths.endpoint_path().exists() {
        fs::read_to_string(paths.endpoint_path())
            .unwrap_or_else(|_| VcuPaths::endpoint_url(&cfg))
            .trim()
            .to_string()
    } else {
        VcuPaths::endpoint_url(&cfg)
    };
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "http client", e.to_string()))?;
    Ok((client, endpoint, cfg.pairing_token))
}

async fn api_get(paths: &VcuPaths, path: &str) -> Result<Value, VcuError> {
    let (client, endpoint, token) = client(paths).await?;
    let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
    let resp = client
        .get(&url)
        .header("X-Vcu-Token", token)
        .send()
        .await
        .map_err(|e| {
            VcuError::with_detail(ErrorCode::DaemonNotRunning, format!("GET {url}"), e.to_string())
        })?;
    let v: Value = resp.json().await.map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "decode", e.to_string())
    })?;
    Ok(v)
}

async fn api_post(paths: &VcuPaths, path: &str, body: Value) -> Result<Value, VcuError> {
    let (client, endpoint, token) = client(paths).await?;
    let url = format!("{}{}", endpoint.trim_end_matches('/'), path);
    let resp = client
        .post(&url)
        .header("X-Vcu-Token", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            VcuError::with_detail(ErrorCode::DaemonNotRunning, format!("POST {url}"), e.to_string())
        })?;
    let v: Value = resp.json().await.map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "decode", e.to_string())
    })?;
    Ok(v)
}




fn self_update(version: &str, base_url: Option<&str>, prefix: &str) -> Result<i32, VcuError> {
    let base = base_url
        .unwrap_or("https://github.com/zhouhanker/versatile-computer-use/releases/latest/download");
    let script_name = installer_script_name();
    let mut script_candidates = vec![
        PathBuf::from(prefix).join("share/vcu/scripts/install").join(script_name),
        PathBuf::from("scripts/install").join(script_name),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent().and_then(|b| b.parent()) {
            script_candidates.insert(
                0,
                root.join("share/vcu/scripts/install").join(script_name),
            );
        }
    }
    let local_script = script_candidates.into_iter().find(|p| p.exists());
    let output = if let Some(script) = local_script {
        run_installer(&script, version, base, prefix)
    } else {
        let tmp = std::env::temp_dir().join(script_name);
        let url = format!("{}/{}", base.trim_end_matches('/'), script_name);
        let body = std::process::Command::new(download_tool())
            .args(download_args(&url))
            .output()
            .map_err(|e| {
                VcuError::with_detail(ErrorCode::Internal, format!("download {script_name}"), e.to_string())
            })?;
        if !body.status.success() {
            let tail = installer_output_tail(&body.stderr, 4);
            let detail = if tail.is_empty() {
                format!("download exit status {}", body.status)
            } else {
                tail
            };
            return Err(VcuError::with_detail(
                ErrorCode::Internal,
                format!(
                    "failed to download {script_name} from {url}; {}",
                    installer_mirror_hint()
                ),
                detail,
            ));
        }
        std::fs::write(&tmp, &body.stdout)?;
        run_installer(&tmp, version, base, prefix)
    }
    .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "update failed", e.to_string()))?;
    if output.status.success() {
        print_ok(
            &json!({
                "updated": true,
                "version_requested": version,
                "prefix": prefix,
                "base_url": base
            }),
            true,
        );
        Ok(0)
    } else {
        let mut detail = format!("installer {}", output.status);
        let stderr = installer_output_tail(&output.stderr, 6);
        let stdout = installer_output_tail(&output.stdout, 6);
        if !stderr.is_empty() {
            detail.push_str(&format!("; stderr: {stderr}"));
        }
        if !stdout.is_empty() {
            detail.push_str(&format!("; stdout: {stdout}"));
        }
        Err(VcuError::with_detail(
            ErrorCode::Internal,
            format!(
                "update installer exited non-zero (base_url {base}); {}",
                installer_mirror_hint()
            ),
            detail,
        ))
    }
}

fn installer_script_name() -> &'static str {
    if cfg!(windows) {
        "install.ps1"
    } else {
        "install.sh"
    }
}

fn installer_mirror_hint() -> &'static str {
    if cfg!(windows) {
        "no published asset? set VCU_BASE_URL=file://$PWD/dist and run install.ps1"
    } else {
        "no published asset? run `bash scripts/pack-release.sh` then `VCU_BASE_URL=file://$PWD/dist vcu self update`"
    }
}

fn download_tool() -> &'static str {
    if cfg!(windows) { "curl.exe" } else { "curl" }
}

fn download_args(url: &str) -> [&str; 2] {
    ["-fsSL", url]
}

fn run_installer(
    script: &std::path::Path,
    version: &str,
    base: &str,
    prefix: &str,
) -> std::io::Result<std::process::Output> {
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(windows) {
        let mut cmd = Command::new("powershell.exe");
        cmd.args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ]);
        cmd.arg(script);
        cmd
    } else {
        let mut cmd = Command::new("bash");
        cmd.arg(script);
        cmd
    };
    cmd.env("VCU_VERSION", version)
        .env("VCU_BASE_URL", base)
        .env("VCU_PREFIX", prefix)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
}

/// Keep only the last few non-empty lines of installer output so an error
/// envelope stays bounded but still explains why the installer failed.
fn installer_output_tail(bytes: &[u8], max_lines: usize) -> String {
    let text = decode_installer_bytes(bytes);
    let lines: Vec<&str> = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    let start = lines.len().saturating_sub(max_lines);
    lines[start..].join(" | ")
}

fn decode_installer_bytes(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xFF, 0xFE]).unwrap_or(bytes);
    if looks_like_utf16_le(bytes) {
        let units: Vec<u16> = bytes
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

fn looks_like_utf16_le(bytes: &[u8]) -> bool {
    if bytes.len() < 8 {
        return false;
    }
    let pairs = bytes.len() / 2;
    let nuls = bytes.iter().skip(1).step_by(2).filter(|b| **b == 0).count();
    nuls * 4 >= pairs * 3
}

fn self_uninstall(paths: &VcuPaths, prefix: &str, purge_config: bool) -> Result<i32, VcuError> {
    use std::fs;
    // NEVER touch Codex Computer Use or third-party computer-use helpers.
    #[cfg(target_os = "macos")]
    {
        let _ = uninstall_macos_launch_agent();
    }
    let bin = PathBuf::from(prefix).join("bin");
    let share = PathBuf::from(prefix).join("share/vcu");
    let mut removed = Vec::new();
    for name in ["vcu", "vcu-daemon", "vcu-mcp", "vcu-stage", "vcu.exe", "vcu-daemon.exe", "vcu-mcp.exe"] {
        let p = bin.join(name);
        if p.exists() {
            let _ = fs::remove_file(&p);
            removed.push(p.display().to_string());
        }
    }
    if share.exists() {
        let _ = fs::remove_dir_all(&share);
        removed.push(share.display().to_string());
    }
    let mut purged_config = false;
    if purge_config && paths.root.exists() {
        // only delete if it looks like a vcu dir (has config.json)
        if paths.config_path().exists() {
            let _ = fs::remove_dir_all(&paths.root);
            purged_config = true;
        }
    }
    print_ok(
        &json!({
            "uninstalled": true,
            "removed": removed,
            "purged_config": purged_config,
            "prefix": prefix,
            "untouched": [
                "Codex Computer Use (~/.codex/computer-use)",
                "OriginOne gpt-bridge computer-helper",
                "Browser profiles / cookies",
            ]
        }),
        true,
    );
    Ok(0)
}

fn copy_dir_filtered(src: &std::path::Path, dst: &std::path::Path) -> Result<u32, VcuError> {
    fs::create_dir_all(dst).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "mkdir lens-extension", e.to_string())
    })?;
    let mut n = 0u32;
    for ent in fs::read_dir(src).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "read extension dir", e.to_string())
    })? {
        let ent = ent.map_err(|e| {
            VcuError::with_detail(ErrorCode::Internal, "read_dir", e.to_string())
        })?;
        let name = ent.file_name();
        let name_s = name.to_string_lossy();
        if name_s.starts_with('.') || name_s.ends_with(".bak") {
            continue;
        }
        let from = ent.path();
        let to = dst.join(&name);
        if from.is_dir() {
            n += copy_dir_filtered(&from, &to)?;
        } else {
            fs::copy(&from, &to).map_err(|e| {
                VcuError::with_detail(ErrorCode::Internal, "copy extension file", e.to_string())
            })?;
            n += 1;
        }
    }
    Ok(n)
}

fn manifest_version(dir: &std::path::Path) -> Option<(u32, u32, u32)> {
    let raw = fs::read_to_string(dir.join("manifest.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let s = v.get("version")?.as_str()?;
    let mut it = s.split('.');
    Some((
        it.next()?.parse().ok()?,
        it.next().and_then(|x| x.parse().ok()).unwrap_or(0),
        it.next().and_then(|x| x.parse().ok()).unwrap_or(0),
    ))
}

fn resolve_packaged_extension(from: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = from {
        let pb = PathBuf::from(p);
        if pb.join("manifest.json").exists() {
            return Some(pb);
        }
        return None;
    }
    if let Ok(p) = std::env::var("VCU_EXTENSION_DIR") {
        let pb = PathBuf::from(p);
        if pb.join("manifest.json").exists() {
            return Some(pb);
        }
    }
    let mut cands = Vec::new();
    if let Some(home) = dirs::home_dir() {
        cands.push(home.join(".local/share/vcu/extension"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            cands.push(parent.join("../share/vcu/extension"));
        }
    }
    cands.push(PathBuf::from("extension"));
    cands
        .into_iter()
        .filter(|p| p.join("manifest.json").exists())
        .max_by_key(|p| manifest_version(p).unwrap_or((0, 0, 0)))
}

fn install_user_lens(paths: &VcuPaths, from: Option<&str>) -> Result<Value, VcuError> {
    let src = resolve_packaged_extension(from).ok_or_else(|| {
        VcuError::coded(
            ErrorCode::InvalidInput,
            "cannot find packaged extension (expected ~/.local/share/vcu/extension)",
        )
    })?;
    let dest = paths.root.join("lens-extension");
    let files = copy_dir_filtered(&src, &dest)?;
    Ok(json!({
        "copied_files": files,
        "src": src,
        "load_unpacked": dest,
        "hud": false,
        "clicked_ui": false,
        "steps": [
            "Open edge://extensions in the USER Edge (the logged-in window, not Agent Edge)",
            "Enable Developer mode",
            format!("Load unpacked → {}", dest.display()),
            "Then `vcu browser login-state` should show extension_profile=user"
        ],
        "never": ["click Allow debugging", "load into empty Agent profile"]
    }))
}

fn local_login_state_fallback() -> serde_json::Value {
    json!({
        "preferred_path": "desktop_user_window",
        "note": "daemon unreachable; run `vcu daemon start` then `vcu browser login-state` for live pids",
        "cdp": browser_discover("9222-9222"),
        "never": ["click Edge Allow debugging", "empty agent profile as login-state", "WeChat", "OS cursor warp"]
    })
}

fn browser_discover(ports_spec: &str) -> serde_json::Value {
    use std::io::{Read, Write};
    use std::net::TcpStream;
    use std::time::Duration;

    let mut ports = Vec::new();
    for part in ports_spec.split(',') {
        let part = part.trim();
        if let Some((a, b)) = part.split_once('-') {
            if let (Ok(a), Ok(b)) = (a.parse::<u16>(), b.parse::<u16>()) {
                let (lo, hi) = if a <= b { (a, b) } else { (b, a) };
                for p in lo..=hi {
                    ports.push(p);
                }
            }
        } else if let Ok(p) = part.parse::<u16>() {
            ports.push(p);
        }
    }
    let mut found = Vec::new();
    for p in ports {
        let addr = format!("127.0.0.1:{p}");
        let Ok(parsed) = addr.parse() else { continue };
        let Ok(mut stream) = TcpStream::connect_timeout(&parsed, Duration::from_millis(150)) else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(250)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(250)));
        let req = format!(
            "GET /json/version HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\nConnection: close\r\n\r\n"
        );
        if stream.write_all(req.as_bytes()).is_err() {
            continue;
        }
        let mut buf = String::new();
        let _ = stream.read_to_string(&mut buf);

        if buf.contains("200") && buf.contains('{') {
            let json_start = match buf.find('{') {
                Some(i) => i,
                None => continue,
            };
            let body = &buf[json_start..];
            let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(json!({}));
            found.push(json!({
                "port": p,
                "endpoint": format!("http://127.0.0.1:{p}"),
                "mode": "http_json",
                "browser": parsed.get("Browser").cloned().unwrap_or(json!(null)),
                "ws": parsed.get("webSocketDebuggerUrl").cloned().unwrap_or(json!(null)),
                "attach": "cdp_existing_debug_session",
                "allow_dialog": "usually_once_or_none_for_flag_launched"
            }));
            continue;
        }

        // Port is open. Try a short WS upgrade probe (must not hang discover).
        let mut ws_ok = false;
        if let Ok(mut stream) = TcpStream::connect_timeout(&parsed, Duration::from_millis(150)) {
            let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
            let upgrade = format!(
                "GET /devtools/browser HTTP/1.1\r\nHost: 127.0.0.1:{p}\r\nConnection: Upgrade\r\nUpgrade: websocket\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n"
            );
            if stream.write_all(upgrade.as_bytes()).is_ok() {
                let mut b = [0u8; 256];
                if let Ok(n) = stream.read(&mut b) {
                    let head = String::from_utf8_lossy(&b[..n]);
                    ws_ok = head.contains("101");
                }
            }
        }

        // UI remote-debugging often 404s /json/*; still a valid takeover candidate.
        let looks_like_ui_rd = buf.contains("404") || buf.is_empty() || buf.contains("403");
        if ws_ok || looks_like_ui_rd {
            found.push(json!({
                "port": p,
                "endpoint": format!("http://127.0.0.1:{p}"),
                "mode": if ws_ok { "browser_ws" } else { "browser_ws_pending" },
                "browser": null,
                "ws": format!("ws://127.0.0.1:{p}/devtools/browser"),
                "attach": "cdp_browser_ws_flat_attach",
                "ws_handshake": if ws_ok { "101" } else { "pending_or_blocked" },
                "note": "CDP abandoned for login-state. Do not click Allow. Use USER Edge extension extract/observe.",
                "allow_dialog": "abandoned_do_not_click",
                "codex_like_alternative": "vcu browser observe / extension extract on USER Edge"
            }));
        }
    }
    json!({
        "found": found,
        "count": found.len(),
        "strategy": {
            "preferred_takeover": "USER Edge unpacked extension (VCU Browser Bridge). CDP is abandoned.",
            "no_repeated_allow": "Never click Allow. Login-state cookies/DOM come from the USER profile extension, not CDP.",
            "codex_like_ux": [
                "Load unpacked ~/.vcu/lens-extension in USER Edge",
                "vcu browser observe / extension extract — no HUD, no Allow",
                "Operate on the logged-in window; do not use empty Agent Edge",
                "os_cursor=deny; never click Allow; never WeChat"
            ],
            "how_to_enable_takeover_macos": [
                "REQUIRED: Load unpacked ~/.vcu/lens-extension (or ~/.local/share/vcu/extension) in USER Edge",
                "Then vcu browser login-state should show extension_profile=user",
                "DOM: vcu session start --backend extension on the USER browser, or extract via the bridge",
                "Do not enable remote debugging. Do not click Allow. Do not set-cdp."
            ],
            "new_browser_vs_takeover": {
                "headless_or_temp_profile": "NEW browser — no user login cookies",
                "cdp_attach_running": "ABANDONED — do not use CDP / Allow for login-state",
                "extension_user_window": "LOGIN-STATE — unpacked extension in USER Edge"
            },
            "never_touch": ["Codex Computer Use", "WeChat automation"]
        }
    })
}


#[cfg(target_os = "macos")]
fn libc_uid() -> u32 {
    unsafe { libc::getuid() }
}

#[cfg(target_os = "macos")]
fn install_macos_launch_agent(paths: &VcuPaths) -> Result<(), VcuError> {
    use std::fs;
    let home = dirs::home_dir().ok_or_else(|| VcuError::coded(ErrorCode::Internal, "no home"))?;
    let bin = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("vcu-daemon")))
        .filter(|p| p.exists())
        .or_else(|| {
            let p = home.join(".local/bin/vcu-daemon");
            if p.exists() { Some(p) } else { None }
        })
        .ok_or_else(|| VcuError::coded(ErrorCode::DaemonNotRunning, "vcu-daemon binary not found on PATH or next to vcu"))?;
    let label = "com.vcu.daemon";
    let plist_path = home.join("Library/LaunchAgents").join(format!("{label}.plist"));
    fs::create_dir_all(plist_path.parent().unwrap())?;
    fs::create_dir_all(home.join("Library/Logs/vcu"))?;
    let vcu_dir = paths.root.display();
    let bin_s = bin.display();
    let log_out = home.join("Library/Logs/vcu/daemon.out.log");
    let log_err = home.join("Library/Logs/vcu/daemon.err.log");
    let plist = format!(r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>Label</key><string>{label}</string>
  <key>ProgramArguments</key>
  <array>
    <string>{bin_s}</string>
    <string>--user-dir</string>
    <string>{vcu_dir}</string>
  </array>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><false/>
  <key>StandardOutPath</key><string>{out}</string>
  <key>StandardErrorPath</key><string>{err}</string>
</dict></plist>
"#, out=log_out.display(), err=log_err.display());
    fs::write(&plist_path, plist)?;
    let uid = unsafe { libc::getuid() };
    let target = format!("gui/{uid}/{label}");
    let _ = std::process::Command::new("launchctl").args(["bootout", &target]).status();
    let st = std::process::Command::new("launchctl")
        .args(["bootstrap", &format!("gui/{uid}"), plist_path.to_str().unwrap()])
        .status()
        .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "launchctl bootstrap", e.to_string()))?;
    if !st.success() {
        return Err(VcuError::coded(ErrorCode::Internal, "launchctl bootstrap failed"));
    }
    let _ = std::process::Command::new("launchctl").args(["kickstart", "-k", &target]).status();
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_macos_launch_agent() -> Result<(), VcuError> {
    let home = dirs::home_dir().ok_or_else(|| VcuError::coded(ErrorCode::Internal, "no home"))?;
    let label = "com.vcu.daemon";
    let uid = unsafe { libc::getuid() };
    let target = format!("gui/{uid}/{label}");
    let _ = std::process::Command::new("launchctl").args(["bootout", &target]).status();
    let plist = home.join("Library/LaunchAgents").join(format!("{label}.plist"));
    let _ = std::fs::remove_file(plist);
    Ok(())
}

#[cfg(test)]
mod installer_output_tests {
    use super::installer_output_tail;

    #[test]
    fn decodes_utf16_le_installer_output() {
        let text = "Downloading file:///dist/vcu-latest-windows-x64.tar.gz";
        let mut bytes = Vec::new();
        for unit in text.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        let tail = installer_output_tail(&bytes, 4);
        assert!(tail.contains(".tar.gz"), "{tail}");
        assert!(!tail.contains('\0'), "{tail}");
    }
}
