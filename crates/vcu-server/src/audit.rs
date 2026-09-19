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


#[cfg(test)]
mod tests {
    use super::*;
    use vcu_core::UserConfig;

    #[test]
    fn audit_is_opt_in_and_records_source() {
        let dir = tempfile::tempdir().unwrap();
        let paths = VcuPaths::from_root(dir.path());
        paths.save_config(&UserConfig::default()).unwrap();
        std::env::remove_var("VCU_AUDIT");
        append(&paths, "session.snapshot", serde_json::json!({"tab":"proc:Finder:1"}));
        assert!(!paths.root.join("audit.jsonl").exists());
        std::env::set_var("VCU_AUDIT", "1");
        append(
            &paths,
            "session.act",
            serde_json::json!({"tab":"proc:TextEdit:1","source":"ax_set_value","surface":"desktop"}),
        );
        std::env::remove_var("VCU_AUDIT");
        let line = std::fs::read_to_string(paths.root.join("audit.jsonl")).unwrap();
        assert!(line.contains("session.act"));
        assert!(line.contains("ax_set_value"));
        assert!(line.contains("proc:TextEdit:1"));
    }
}
