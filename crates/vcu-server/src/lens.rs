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

fn manifest_version(dir: &Path) -> Option<(u32, u32, u32)> {
    let raw = fs::read_to_string(dir.join("manifest.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&raw).ok()?;
    let s = v.get("version")?.as_str()?;
    let mut it = s.split('.');
    Some((
        it.next()?.parse().ok()?,
        it.next().and_then(|x| x.parse().ok()).unwrap_or(0),
        it.next().and_then(|x| x.parse().ok()).unwrap_or(0),
    ))
}

fn pick_newest(cands: Vec<PathBuf>) -> Option<PathBuf> {
    cands
        .into_iter()
        .filter(|p| p.join("manifest.json").exists())
        .max_by_key(|p| manifest_version(p).unwrap_or((0, 0, 0)))
}

fn resolve_packaged_extension(paths: &VcuPaths) -> Option<PathBuf> {
    if let Ok(p) = std::env::var("VCU_EXTENSION_DIR") {
        let pb = PathBuf::from(p);
        if pb.join("manifest.json").exists() {
            return Some(pb);
        }
    }
    let mut cands = Vec::new();
    if let Some(home) = paths.root.parent() {
        cands.push(home.join(".local/share/vcu/extension"));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            cands.push(parent.join("../share/vcu/extension"));
        }
    }
    cands.push(PathBuf::from("extension"));
    pick_newest(cands)
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
