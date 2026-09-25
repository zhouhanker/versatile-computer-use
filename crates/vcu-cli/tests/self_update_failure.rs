use std::process::Command;

#[test]
fn self_update_reports_installer_output_and_local_mirror_hint() {
    let root = std::env::temp_dir().join(format!("vcu-self-update-{}", std::process::id()));
    let dist = root.join("dist");
    let prefix = root.join("prefix");
    let script_dir = prefix.join("share/vcu/scripts/install");
    std::fs::create_dir_all(&dist).unwrap();
    std::fs::create_dir_all(&script_dir).unwrap();
    let script_name = if cfg!(windows) { "install.ps1" } else { "install.sh" };
    let repo_script = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts/install")
        .join(script_name);
    std::fs::copy(&repo_script, script_dir.join(script_name)).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_vcu"))
        .env("VCU_BASE_URL", format!("file://{}", dist.display()))
        .env("VCU_PREFIX", prefix.display().to_string())
        .args(["self", "update", "--json"])
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&root);

    assert!(!output.status.success(), "self update must fail without an archive");
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ok"], false, "{value}");
    let message = value["error"]["message"].as_str().unwrap_or("");
    let detail = value["error"]["detail"].as_str().unwrap_or("");
    assert!(message.contains("non-zero"), "{message}");
    assert!(message.contains("VCU_BASE_URL=file://"), "{message}");
    assert!(detail.contains("installer"), "{detail}");
    assert!(detail.contains(".tar.gz") || detail.contains("No such file"), "{detail}");
}
