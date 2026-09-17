//! Append-only local audit log (JSONL). Enabled with VCU_AUDIT=1.
use std::fs::OpenOptions;
use std::io::Write;
use serde_json::{json, Value};
use vcu_core::VcuPaths;

pub fn append(paths: &VcuPaths, kind: &str, payload: Value) {
    let enabled = std::env::var("VCU_AUDIT")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    if !enabled {
        return;
    }
    let _ = paths.ensure_layout();
    let path = paths.root.join("audit.jsonl");
    let line = json!({
        "ts": chrono::Utc::now().to_rfc3339(),
        "kind": kind,
        "payload": payload,
    });
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{line}");
    }
}
