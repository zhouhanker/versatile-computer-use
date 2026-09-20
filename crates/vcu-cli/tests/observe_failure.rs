use std::io::{Read, Write};
use std::net::TcpListener;
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
    let server = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut served = 0;
        while served < 1 && Instant::now() < deadline {
            let Ok((mut socket, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(10));
                continue;
            };
            socket.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
            let mut request = Vec::new();
            loop {
                let mut chunk = [0; 4096];
                let n = socket.read(&mut chunk).unwrap();
                request.extend_from_slice(&chunk[..n]);
                if n == 0 || request.windows(4).any(|w| w == b"\r\n\r\n") { break; }
            }
            let _request = String::from_utf8_lossy(&request);
            let body = json!({"ok":false,"error":{"code":"ActionFailed","message":"window screenshot failed"}}).to_string();
            write!(socket, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            served += 1;
        }
        assert_eq!(served, 1);
    });
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vcu"))
        .arg("--user-dir").arg(&root).args(["browser", "observe", "--json"]).output().unwrap();
    server.join().unwrap();
    let _ = std::fs::remove_dir_all(&root);
    assert!(!output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false, "{value}");
    assert_eq!(value["error"]["message"], "window screenshot failed");
    assert!(value.get("data").is_none());
}
