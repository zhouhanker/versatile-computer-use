//! Copy VCU MV3 extension for USER-profile load-unpacked. Never clicks Edge UI.
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use vcu_core::{ErrorCode, VcuError, VcuPaths, VcuResult};

fn copy_dir_filtered(src: &Path, dst: &Path) -> VcuResult<u32> {
    fs::create_dir_all(dst).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "mkdir lens-extension", e.to_string())
    })?;
    let mut n = 0u32;
    for ent in fs::read_dir(src).map_err(|e| {
        VcuError::with_detail(ErrorCode::Internal, "read extension dir", e.to_string())
    })? {
        let ent = ent.map_err(|e| VcuError::with_detail(ErrorCode::Internal, "read_dir", e.to_string()))?;
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

fn resolve_packaged_extension(paths: &VcuPaths) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VCU_EXTENSION_DIR") {
        let pb = PathBuf::from(p);
        if pb.join("manifest.json").exists() {
            return Some(pb);
        }
    }
    if let Some(home) = paths.root.parent() {
        let share = home.join(".local/share/vcu/extension");
        if share.join("manifest.json").exists() {
            return Some(share);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let cand = parent.join("../share/vcu/extension");
            if cand.join("manifest.json").exists() {
                return Some(cand);
            }
        }
    }
    let cwd = PathBuf::from("extension");
    if cwd.join("manifest.json").exists() {
        return Some(cwd);
    }
    None
}

pub fn install_user_lens(paths: &VcuPaths) -> VcuResult<Value> {
    let src = resolve_packaged_extension(paths).ok_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn copies_manifest_and_skips_bak() {
        let src = tempdir().unwrap();
        fs::write(src.path().join("manifest.json"), b"{}").unwrap();
        fs::write(src.path().join("background.js"), b"1").unwrap();
        fs::write(src.path().join("x.bak"), b"no").unwrap();
        let dst = tempdir().unwrap();
        let n = copy_dir_filtered(src.path(), dst.path()).unwrap();
        assert_eq!(n, 2);
        assert!(dst.path().join("manifest.json").exists());
        assert!(!dst.path().join("x.bak").exists());
    }
}
