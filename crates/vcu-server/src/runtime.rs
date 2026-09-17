use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use tokio::net::TcpListener;
use tracing::info;
use vcu_core::{UserConfig, VcuPaths, VcuResult};

use crate::api;
use crate::state::AppState;

pub struct DaemonHandle {
    pub addr: SocketAddr,
    pub state: AppState,
    pub join: tokio::task::JoinHandle<()>,
}

pub async fn start_daemon(paths: VcuPaths, config: UserConfig) -> VcuResult<DaemonHandle> {
    paths.ensure_layout()?;
    let host = config.daemon_host.clone();
    let port = config.daemon_port;
    let state = AppState::new(paths.clone(), config.clone());
    let app = api::router(state.clone());
    let addr: SocketAddr = format!("{host}:{port}")
        .parse()
        .map_err(|e: std::net::AddrParseError| vcu_core::VcuError::with_detail(vcu_core::ErrorCode::Internal, "bad bind addr", e.to_string()))?;
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        vcu_core::VcuError::with_detail(vcu_core::ErrorCode::Internal, "bind failed", e.to_string())
    })?;
    let local = listener.local_addr().map_err(|e| {
        vcu_core::VcuError::with_detail(vcu_core::ErrorCode::Internal, "local_addr", e.to_string())
    })?;
    let endpoint = format!("http://{local}");
    fs::write(paths.endpoint_path(), &endpoint)?;
    fs::write(paths.pid_path(), std::process::id().to_string())?;
    info!(%endpoint, "vcu daemon listening");
    let join = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!(error=%e, "daemon server error");
        }
    });
    Ok(DaemonHandle {
        addr: local,
        state,
        join,
    })
}

pub fn default_user_dir_override(override_dir: Option<PathBuf>) -> VcuResult<VcuPaths> {
    match override_dir {
        Some(p) => Ok(VcuPaths::from_root(p)),
        None => VcuPaths::default_user(),
    }
}
