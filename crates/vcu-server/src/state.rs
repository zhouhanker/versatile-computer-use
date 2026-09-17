use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use vcu_core::{Blackboard, Session, UserConfig, VcuPaths};

use crate::browser::extension::ExtensionBridge;
use crate::browser::BrowserBackend;
use crate::app::{detect_app_backend_with_allowlist, AppBackend};

#[derive(Clone)]
pub struct AppState {
    pub paths: Arc<VcuPaths>,
    pub config: Arc<RwLock<UserConfig>>,
    pub sessions: Arc<RwLock<HashMap<String, SessionSlot>>>,
    pub extension_bridge: ExtensionBridge,
    pub app_backend: Arc<RwLock<Box<dyn AppBackend>>>,
    pub started_at: chrono::DateTime<chrono::Utc>,
    pub version: String,
}

pub struct SessionSlot {
    pub session: Session,
    pub backend: Box<dyn BrowserBackend>,
    pub blackboard: Blackboard,
    pub idempotency: HashMap<String, serde_json::Value>,
    pub borrows: HashMap<String, bool>,
}

impl AppState {
    pub fn new(paths: VcuPaths, config: UserConfig) -> Self {
        let allow = if config.app_allowlist.is_empty() {
            None
        } else {
            Some(config.app_allowlist.clone())
        };
        Self {
            paths: Arc::new(paths),
            config: Arc::new(RwLock::new(config)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            extension_bridge: ExtensionBridge::new(),
            app_backend: Arc::new(RwLock::new(detect_app_backend_with_allowlist(allow))),
            started_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

pub async fn create_backend(
    name: &str,
    config: &UserConfig,
    extension_bridge: &ExtensionBridge,
) -> Result<Box<dyn BrowserBackend>, vcu_core::VcuError> {
    match name {
        "mock" => Ok(Box::new(crate::browser::MockBackend::new())),
        "cdp" => {
            let url = config
                .cdp_url
                .clone()
                .unwrap_or_else(|| "http://127.0.0.1:9222".into());
            let backend = crate::browser::cdp::CdpBackend::connect(&url).await?;
            Ok(Box::new(backend))
        }
        "extension" => Ok(Box::new(
            crate::browser::extension::ExtensionBackend::with_bridge(extension_bridge.clone()),
        )),
        other => Err(vcu_core::VcuError::coded(
            vcu_core::ErrorCode::InvalidInput,
            format!("unknown backend: {other}"),
        )),
    }
}
