use std::io::{ErrorKind, Read, Write};
use std::net::{Shutdown, TcpListener};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use vcu_core::{new_id, UserConfig, VcuPaths};

#[test]
fn observe_does_not_wrap_a_failed_snapshot_in_success() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let root = std::env::temp_dir().join(format!("vcu-observe-error-{}", new_id()));
    let paths = VcuPaths::from_root(&root);
    let mut cfg = UserConfig::default();
    cfg.daemon_port = listener.local_addr().unwrap().port();
    paths.save_config(&cfg).unwrap();

    let stop = Arc::new(AtomicBool::new(false));
    let stop_for_server = Arc::clone(&stop);
    let server = std::thread::spawn(move || serve_failed_observe(listener, stop_for_server));

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vcu"))
        .arg("--user-dir")
        .arg(&root)
        .args(["browser", "observe", "--json"])
        .output()
        .unwrap();
    stop.store(true, Ordering::SeqCst);
    let served = server.join().expect("observe mock thread");
    let _ = std::fs::remove_dir_all(&root);

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        served, 1,
        "mock daemon was not contacted\nstatus: {:?}\nstdout:\n{stdout}\nstderr:\n{stderr}",
        output.status
    );
    assert!(
        !output.status.success(),
        "failed observe must not exit 0\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );
    let value: Value = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("observe stdout is not JSON: {err}\n{stdout}"));
    assert_eq!(value["ok"], false, "{value}");
    assert_eq!(value["error"]["message"], "window screenshot failed");
    assert!(value.get("data").is_none(), "{value}");
}

fn serve_failed_observe(listener: TcpListener, stop: Arc<AtomicBool>) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(60);
    while !stop.load(Ordering::SeqCst) && Instant::now() < deadline {
        match listener.accept() {
            Ok((mut socket, _)) => {
                socket.set_nonblocking(false).expect("blocking socket");
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .expect("read timeout");
                read_headers(&mut socket, deadline);
                let body = json!({
                    "ok": false,
                    "error": {"code": "ActionFailed", "message": "window screenshot failed"}
                })
                .to_string();
                write!(
                    socket,
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                )
                .expect("write observe failure");
                let _ = socket.flush();
                let _ = socket.shutdown(Shutdown::Both);
                return 1;
            }
            Err(err) if err.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("accept failed: {err}"),
        }
    }
    0
}

fn read_headers(socket: &mut impl Read, deadline: Instant) {
    let mut request = Vec::new();
    while Instant::now() < deadline {
        let mut chunk = [0; 4096];
        match socket.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                request.extend_from_slice(&chunk[..n]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    return;
                }
            }
            Err(err)
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut =>
            {
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    return;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("read observe request: {err}"),
        }
    }
    panic!("timed out reading observe request");
}
