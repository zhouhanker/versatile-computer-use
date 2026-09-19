use std::fs;
use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::net::SocketAddr;
use std::path::PathBuf;

use tokio::net::TcpListener;
use tracing::info;
use vcu_core::{ErrorCode, UserConfig, VcuError, VcuPaths, VcuResult};

use crate::api;
use crate::state::AppState;

pub struct DaemonHandle {
    pub addr: SocketAddr,
    pub state: AppState,
    pub join: tokio::task::JoinHandle<()>,
    _lock: Option<DaemonLock>,
}

struct DaemonLock {
    file: Option<fs::File>,
    path: PathBuf,
}

impl Drop for DaemonLock {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(file) = self.file.as_ref() {
            use std::os::unix::io::AsRawFd;
            let _ = flock(file.as_raw_fd(), LOCK_UN);
        }
        // Close before unlink: Windows cannot delete a file that is still open
        // with exclusive share_mode(0).
        drop(self.file.take());
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(unix)]
const LOCK_EX: i32 = 2;
#[cfg(unix)]
const LOCK_NB: i32 = 4;
#[cfg(unix)]
const LOCK_UN: i32 = 8;

#[cfg(unix)]
fn flock(fd: i32, op: i32) -> std::io::Result<()> {
    extern "C" {
        fn flock(fd: i32, operation: i32) -> i32;
    }
    let rc = unsafe { flock(fd, op) };
    if rc == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

fn pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    {
        extern "C" {
            fn kill(pid: i32, sig: i32) -> i32;
        }
        let rc = unsafe { kill(pid as i32, 0) };
        if rc == 0 {
            return true;
        }
        // EPERM (1) means the process exists but we cannot signal it.
        std::io::Error::last_os_error().raw_os_error() == Some(1)
    }
    #[cfg(windows)]
    {
        const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
        extern "system" {
            fn OpenProcess(access: u32, inherit: i32, pid: u32) -> isize;
            fn CloseHandle(handle: isize) -> i32;
        }
        let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if handle == 0 {
            return false;
        }
        unsafe {
            CloseHandle(handle);
        }
        true
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}

fn read_pid_file(path: &std::path::Path) -> Option<u32> {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Windows `ERROR_SHARING_VIOLATION` — exclusive `share_mode(0)` already held.
#[cfg(windows)]
const ERROR_SHARING_VIOLATION: i32 = 32;

fn is_exclusive_lock_held(err: &std::io::Error) -> bool {
    #[cfg(windows)]
    {
        err.raw_os_error() == Some(ERROR_SHARING_VIOLATION)
    }
    #[cfg(not(windows))]
    {
        let _ = err;
        false
    }
}

fn open_daemon_lock_file(path: &std::path::Path) -> std::io::Result<fs::File> {
    let mut opts = OpenOptions::new();
    opts.create(true).read(true).write(true).truncate(false);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        // Deny read/write/delete sharing so a second open in this process or
        // another process fails immediately. Unix still uses flock below.
        opts.share_mode(0);
    }
    opts.open(path)
}

fn write_pid_to_lock(file: &mut fs::File) -> std::io::Result<()> {
    file.set_len(0)?;
    file.seek(SeekFrom::Start(0))?;
    write!(file, "{}", std::process::id())?;
    file.flush()?;
    Ok(())
}

fn already_running(path: &std::path::Path, holder: u32, err: impl std::fmt::Display) -> VcuError {
    VcuError::with_detail(
        ErrorCode::DaemonAlreadyRunning,
        "vcu-daemon already running",
        format!("lock={} pid={holder} err={err}", path.display()),
    )
}

fn acquire_daemon_lock(paths: &VcuPaths) -> VcuResult<DaemonLock> {
    paths.ensure_layout()?;
    let path = paths.lock_path();
    let mut file = match open_daemon_lock_file(&path) {
        Ok(f) => f,
        Err(e) if is_exclusive_lock_held(&e) => {
            let holder = read_pid_file(&path)
                .or_else(|| read_pid_file(&paths.pid_path()))
                .unwrap_or(0);
            return Err(already_running(&path, holder, e));
        }
        Err(e) => {
            return Err(VcuError::with_detail(
                ErrorCode::Internal,
                "open daemon.lock failed",
                e.to_string(),
            ));
        }
    };

    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        if let Err(e) = flock(file.as_raw_fd(), LOCK_EX | LOCK_NB) {
            let holder = read_pid_file(&path)
                .or_else(|| read_pid_file(&paths.pid_path()))
                .unwrap_or(0);
            return Err(already_running(&path, holder, e));
        }
    }

    if let Some(pid) = read_pid_file(&paths.pid_path()) {
        if pid != std::process::id() && pid_alive(pid) {
            return Err(VcuError::with_detail(
                ErrorCode::DaemonAlreadyRunning,
                "vcu-daemon already running",
                format!("pid={pid}"),
            ));
        }
    }

    write_pid_to_lock(&mut file).map_err(|e| {
        VcuError::with_detail(
            ErrorCode::Internal,
            "write daemon.lock failed",
            e.to_string(),
        )
    })?;
    Ok(DaemonLock {
        file: Some(file),
        path,
    })
}

pub async fn start_daemon(paths: VcuPaths, config: UserConfig) -> VcuResult<DaemonHandle> {
    paths.ensure_layout()?;
    let lock = acquire_daemon_lock(&paths)?;
    let host = config.daemon_host.clone();
    let port = config.daemon_port;
    let state = AppState::new(paths.clone(), config.clone());
    let app = api::router(state.clone());
    let addr: SocketAddr =
        format!("{host}:{port}")
            .parse()
            .map_err(|e: std::net::AddrParseError| {
                VcuError::with_detail(ErrorCode::Internal, "bad bind addr", e.to_string())
            })?;
    let listener = match TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            return Err(VcuError::with_detail(
                ErrorCode::Internal,
                "bind failed",
                e.to_string(),
            ));
        }
    };
    let local = listener
        .local_addr()
        .map_err(|e| VcuError::with_detail(ErrorCode::Internal, "local_addr", e.to_string()))?;
    let endpoint = format!("http://{local}");
    fs::write(paths.endpoint_path(), &endpoint)?;
    fs::write(paths.pid_path(), std::process::id().to_string())?;
    info!(%endpoint, "vcu daemon listening");
    let join = tokio::spawn(async move {
        if let Err(e) = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            tracing::error!(error=%e, "daemon server error");
        }
    });
    Ok(DaemonHandle {
        addr: local,
        state,
        join,
        _lock: Some(lock),
    })
}

pub fn default_user_dir_override(override_dir: Option<PathBuf>) -> VcuResult<VcuPaths> {
    match override_dir {
        Some(p) => Ok(VcuPaths::from_root(p)),
        None => VcuPaths::default_user(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vcu_core::ErrorCode;

    #[test]
    fn second_lock_is_denied_then_released() {
        let dir = tempfile::tempdir().unwrap();
        let paths = VcuPaths::from_root(dir.path());
        let first = acquire_daemon_lock(&paths).expect("first lock");
        let second = acquire_daemon_lock(&paths);
        let err = match second {
            Err(e) => e,
            Ok(_) => panic!("second lock must fail with DaemonAlreadyRunning"),
        };
        assert_eq!(err.code(), ErrorCode::DaemonAlreadyRunning);
        drop(first);
        let third = acquire_daemon_lock(&paths).expect("lock after drop");
        drop(third);
    }
}
