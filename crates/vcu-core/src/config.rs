use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use rand::RngCore;
use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, VcuError, VcuResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VisionPolicy {
    DomFirst,
    VisionFirst,
    DomOnly,
    VisionAlways,
}

impl VisionPolicy {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "dom_first" => Some(Self::DomFirst),
            "vision_first" => Some(Self::VisionFirst),
            "dom_only" => Some(Self::DomOnly),
            "vision_always" => Some(Self::VisionAlways),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::DomFirst => "dom_first",
            Self::VisionFirst => "vision_first",
            Self::DomOnly => "dom_only",
            Self::VisionAlways => "vision_always",
        }
    }
}

impl Default for VisionPolicy {
    fn default() -> Self {
        Self::DomFirst
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    pub model: String,
    /// Environment variable name holding the API key (never the key itself).
    pub api_key_env: String,
    #[serde(default)]
    pub kind: String, // vision | chat
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserConfig {
    pub version: u32,
    pub pairing_token: String,
    pub daemon_host: String,
    pub daemon_port: u16,
    #[serde(default)]
    pub vision_policy: VisionPolicy,
    #[serde(default)]
    pub models: BTreeMap<String, ModelConfig>,
    #[serde(default)]
    pub default_vision_model: Option<String>,
    #[serde(default)]
    pub cdp_url: Option<String>,
    /// Process name substrings allowed for app CU (empty = backend default).
    #[serde(default)]
    pub app_allowlist: Vec<String>,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            version: 1,
            pairing_token: generate_token(),
            daemon_host: "127.0.0.1".into(),
            daemon_port: 17890,
            vision_policy: VisionPolicy::default(),
            models: BTreeMap::new(),
            default_vision_model: None,
            cdp_url: None,
            app_allowlist: Vec::new(),
        }
    }
}

fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

#[derive(Debug, Clone)]
pub struct VcuPaths {
    pub root: PathBuf,
}

impl VcuPaths {
    pub fn from_root(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_user() -> VcuResult<Self> {
        let home = dirs::home_dir().ok_or_else(|| {
            VcuError::coded(ErrorCode::Internal, "cannot resolve home directory")
        })?;
        Ok(Self { root: home.join(".vcu") })
    }

    pub fn config_path(&self) -> PathBuf {
        self.root.join("config.json")
    }

    pub fn pid_path(&self) -> PathBuf {
        self.root.join("daemon.pid")
    }

    pub fn endpoint_path(&self) -> PathBuf {
        self.root.join("daemon.endpoint")
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn captures_dir(&self) -> PathBuf {
        self.root.join("captures")
    }

    pub fn skills_dir(&self) -> PathBuf {
        self.root.join("skills")
    }

    pub fn ensure_layout(&self) -> VcuResult<()> {
        for d in [
            self.root.clone(),
            self.sessions_dir(),
            self.captures_dir(),
            self.skills_dir(),
        ] {
            fs::create_dir_all(&d)?;
        }
        Ok(())
    }

    pub fn load_config(&self) -> VcuResult<UserConfig> {
        let path = self.config_path();
        if !path.exists() {
            return Err(VcuError::coded(
                ErrorCode::InvalidInput,
                format!("config missing at {}; run `vcu init`", path.display()),
            ));
        }
        let text = fs::read_to_string(&path)?;
        let cfg: UserConfig = serde_json::from_str(&text)?;
        Ok(cfg)
    }

    pub fn save_config(&self, cfg: &UserConfig) -> VcuResult<()> {
        self.ensure_layout()?;
        let text = serde_json::to_string_pretty(cfg)?;
        fs::write(self.config_path(), text)?;
        Ok(())
    }

    pub fn init_if_needed(&self) -> VcuResult<UserConfig> {
        self.ensure_layout()?;
        let path = self.config_path();
        if path.exists() {
            return self.load_config();
        }
        let cfg = UserConfig::default();
        self.save_config(&cfg)?;
        Ok(cfg)
    }

    pub fn endpoint_url(cfg: &UserConfig) -> String {
        format!("http://{}:{}", cfg.daemon_host, cfg.daemon_port)
    }
}

// fix typo - I used `pub def` by mistake

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn init_and_reload_config() {
        let dir = tempdir().unwrap();
        let paths = VcuPaths::from_root(dir.path());
        let cfg = paths.init_if_needed().unwrap();
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.pairing_token.len(), 64);
        let loaded = paths.load_config().unwrap();
        assert_eq!(loaded.pairing_token, cfg.pairing_token);
    }

    #[test]
    fn vision_policy_roundtrip() {
        assert_eq!(VisionPolicy::parse("dom_first"), Some(VisionPolicy::DomFirst));
        assert_eq!(VisionPolicy::DomOnly.as_str(), "dom_only");
    }
}
