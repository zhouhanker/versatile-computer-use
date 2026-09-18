use std::path::PathBuf;

use clap::Parser;
use vcu_core::VcuPaths;

#[derive(Parser, Debug)]
#[command(name = "vcu-daemon", version)]
struct Args {
    #[arg(long, env = "VCU_DIR")]
    user_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() {
    #[cfg(unix)]
    unsafe {
        // Survive parent-shell hangup when started without launchd/nohup.
        libc::signal(libc::SIGHUP, libc::SIG_IGN);
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(false)
        .compact()
        .init();
    let args = Args::parse();
    let paths = match args.user_dir {
        Some(p) => VcuPaths::from_root(p),
        None => VcuPaths::default_user().expect("home"),
    };
    let cfg = paths.init_if_needed().expect("init config");
    match vcu_server::start_daemon(paths, cfg).await {
        Ok(handle) => {
            tracing::info!(addr=%handle.addr, "listening");
            let _ = handle.join.await;
        }
        Err(e) => {
            eprintln!("daemon failed: {e}");
            std::process::exit(1);
        }
    }
}
