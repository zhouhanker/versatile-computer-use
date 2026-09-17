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
    },
    Navigate {
        #[arg(long)]
        session: String,
        #[arg(long)]
        url: String,
    },
    Click {
        #[arg(long)]
        session: String,
        #[arg(long = "ref")]
        target_ref: String,
    },
    Type {
        #[arg(long)]
        session: String,
        #[arg(long)]
        text: String,
        #[arg(long = "ref")]
        target_ref: Option<String>,
    },
    Extract {
        #[arg(long)]
        session: String,
        #[arg(long)]
        selector: String,
    },
    Screenshot {
        #[arg(long)]
        session: String,
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
    Wait {
        #[arg(long)]
        session: String,
        #[arg(long, default_value_t = 200)]
        ms: u64,
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
        #[arg(long, default_value = "mock")]
        backend: String,
        #[arg(long)]
        vision_policy: Option<String>,
    },
    List,
    Show {
        id: String,
    },
    Stop {
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
}

#[derive(Subcommand, Debug)]
enum AppCmd {
    Windows,
    Snapshot {
        id: String,
        #[arg(long, default_value_t = 4000)]
        budget: u64,
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

#[tokio::main]
async fn main() {
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
        ErrorCode::BorrowRequired | ErrorCode::OsCursorDenied | ErrorCode::VisionProviderRequired => 4,
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
                                "Load extension from ./extension (Chrome/Edge unpacked) OR use --backend mock/cdp",
                                "vcu init model   # if main agent lacks vision"
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
        } => {
            let body = json!({"mode": mode, "budget": budget, "force_vision": force_vision});
            let v = api_post(&paths, &format!("/v1/session/{session}/snapshot"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Navigate { session, url } => {
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/navigate"),
                json!({"url": url}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Click { session, target_ref } => {
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/click"),
                json!({"ref": target_ref}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Type {
            session,
            text,
            target_ref,
        } => {
            let mut body = json!({"text": text});
            if let Some(r) = target_ref {
                body["ref"] = json!(r);
            }
            let v = api_post(&paths, &format!("/v1/session/{session}/type"), body).await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Extract { session, selector } => {
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/extract"),
                json!({"selector": selector}),
            )
            .await?;
            println!("{}", serde_json::to_string_pretty(&v).unwrap_or_default());
            Ok(ok_exit(&v))
        }
        Commands::Screenshot {
            session,
            full_page,
            out,
        } => {
            let body = json!({"full_page": full_page, "out": out});
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
        Commands::Wait { session, ms } => {
            let v = api_post(
                &paths,
                &format!("/v1/session/{session}/act"),
                json!({"type": "wait", "target": {}, "args": {"ms": ms}}),
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
            AppCmd::Snapshot { id, budget } => {
                let v = api_post(
                    &paths,
                    "/v1/app/snapshot",
                    json!({"id": id, "budget": budget}),
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
            vision_policy,
        } => {
            let mut body = json!({"browser": browser, "backend": backend});
            if let Some(vp) = vision_policy {
                body["vision_policy"] = json!(vp);
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
            let v = api_post(paths, &format!("/v1/session/{id}/stop"), json!({})).await?;
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

async fn daemon_start(paths: &VcuPaths, foreground: bool) -> Result<i32, VcuError> {
    let cfg = paths.init_if_needed()?;
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
        .timeout(Duration::from_secs(30))
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
    use std::process::Command;
    let base = base_url
        .unwrap_or("https://github.com/zhouhanker/versatile-computer-use/releases/latest/download");
    // Prefer shipping install.sh next to this binary's share, else curl remote install.sh
    let mut script_candidates = vec![
        PathBuf::from(prefix).join("share/vcu/scripts/install/install.sh"),
        PathBuf::from("scripts/install/install.sh"),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent().and_then(|b| b.parent()) {
            // prefix/bin/vcu -> prefix/share/vcu/scripts/...
            script_candidates.insert(0, root.join("share/vcu/scripts/install/install.sh"));
        }
    }
    let local_script = script_candidates.into_iter().find(|p| p.exists());
    use std::process::Stdio;
    let status = if let Some(script) = local_script {
        Command::new("bash")
            .arg(script)
            .env("VCU_VERSION", version)
            .env("VCU_BASE_URL", base)
            .env("VCU_PREFIX", prefix)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    } else {
        // download install.sh then run
        let tmp = std::env::temp_dir().join("vcu-install.sh");
        let url = format!("{}/install.sh", base.trim_end_matches('/'));
        let body = std::process::Command::new("curl")
            .args(["-fsSL", &url])
            .output()
            .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "curl install.sh", e.to_string()))?;
        if !body.status.success() {
            return Err(VcuError::coded(
                ErrorCode::Internal,
                format!("failed to download install.sh from {url}"),
            ));
        }
        std::fs::write(&tmp, &body.stdout)?;
        Command::new("bash")
            .arg(&tmp)
            .env("VCU_VERSION", version)
            .env("VCU_BASE_URL", base)
            .env("VCU_PREFIX", prefix)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    }
    .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "update failed", e.to_string()))?;
    if status.success() {
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
        Err(VcuError::coded(ErrorCode::Internal, "update installer exited non-zero"))
    }
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
    for name in ["vcu", "vcu-daemon", "vcu-mcp", "vcu.exe", "vcu-daemon.exe", "vcu-mcp.exe"] {
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
        let Ok(mut stream) = TcpStream::connect_timeout(
            &addr.parse().unwrap_or_else(|_| "127.0.0.1:1".parse().unwrap()),
            Duration::from_millis(200),
        ) else {
            continue;
        };
        let _ = stream.set_read_timeout(Some(Duration::from_millis(400)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));
        let req = format!(
            "GET /json/version HTTP/1.1
Host: 127.0.0.1:{p}
Connection: close

"
        );
        if stream.write_all(req.as_bytes()).is_err() {
            continue;
        }
        let mut buf = String::new();
        let _ = stream.read_to_string(&mut buf);
        if !buf.contains("200") || !buf.contains('{') {
            continue;
        }
        let json_start = match buf.find('{') {
            Some(i) => i,
            None => continue,
        };
        let body = &buf[json_start..];
        let parsed: serde_json::Value = serde_json::from_str(body).unwrap_or(json!({}));
        found.push(json!({
            "port": p,
            "endpoint": format!("http://127.0.0.1:{p}"),
            "browser": parsed.get("Browser").cloned().unwrap_or(json!(null)),
            "ws": parsed.get("webSocketDebuggerUrl").cloned().unwrap_or(json!(null)),
            "attach": "cdp_existing_debug_session"
        }));
    }
    json!({
        "found": found,
        "count": found.len(),
        "strategy": {
            "preferred_takeover": "Attach CDP to already-running browser with remote debugging enabled (keeps cookies/login).",
            "codex_like_ux": [
                "Operate in a dedicated agent surface when possible (Agent Window / separate tab)",
                "Do not steal OS cursor; use page Input/DOM actions",
                "Tab borrow/return for user tabs",
                "Avoid full-screen OS cursor hijack; prefer DOM/AX actions"
            ],
            "how_to_enable_takeover_macos": [
                "Edge: open edge://inspect/#remote-debugging and enable remote debugging",
                "Chrome: open chrome://inspect/#remote-debugging and enable remote debugging",
                "Or relaunch browser with --remote-debugging-port=9222 using your normal profile (see docs/macos/BROWSER_TAKEOVER.md)",
                "Then: vcu browser discover && vcu config set-cdp http://127.0.0.1:<port> && vcu session start --backend cdp"
            ],
            "new_browser_vs_takeover": {
                "headless_or_temp_profile": "NEW browser — no user login cookies",
                "cdp_attach_running": "TAKEOVER — same profile/session if debugging enabled on that instance",
                "extension_agent_window": "PARALLEL agent window in same browser process; user tabs need explicit borrow"
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
  <key>KeepAlive</key><true/>
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
