use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ErrorCode {
    DaemonNotRunning,
    DaemonAlreadyRunning,
    DaemonAuthFailed,
    ExtensionDisconnected,
    SessionNotFound,
    SessionClosed,
    BorrowRequired,
    TabNotFound,
    TabAlreadyBorrowed,
    FocusPolicyViolation,
    OsCursorDenied,
    VisionProviderRequired,
    VisionCallFailed,
    ModelNotFound,
    BudgetExceeded,
    InvalidInput,
    BrowserUnavailable,
    CdpConnectFailed,
    ActionFailed,
    IdempotencyConflict,
    Internal,
    NotImplemented,
    AccessibilityDenied,
    AppDenied,
}

impl ErrorCode {
    pub fn default_hint(self) -> &'static str {
        match self {
            Self::DaemonNotRunning => "Run `vcu daemon start` then retry.",
            Self::DaemonAlreadyRunning => "A vcu-daemon is already running. Use `vcu daemon status` or `vcu daemon stop`.",
            Self::DaemonAuthFailed => "Pairing token mismatch. Re-run `vcu init` or restart daemon.",
            Self::ExtensionDisconnected => "Load the VCU extension in Chrome/Edge and complete pairing, or use --backend mock/cdp.",
            Self::SessionNotFound => "Pass a valid --session id from `vcu session start --json`.",
            Self::SessionClosed => "Start a new session with `vcu session start`.",
            Self::BorrowRequired => "User tabs require explicit borrow: `vcu tabs borrow --session <id> --tab <tab_id>`.",
            Self::TabNotFound => "Refresh tabs with `vcu tabs list --session <id>`.",
            Self::TabAlreadyBorrowed => "Return the tab first or wait until the other session releases it.",
            Self::FocusPolicyViolation => "Browser adapter forbids stealing user focus; operate in Agent Window.",
            Self::OsCursorDenied => "OS cursor warp is denied. On desktop, use Scene refs + AX press/set; Guide is overlay-only.",
            Self::VisionProviderRequired => "Configure vision: `vcu init model` or `vcu model set vision ...`.",
            Self::VisionCallFailed => "Check vision provider base-url/model/api-key-env and run `vcu model test vision`.",
            Self::ModelNotFound => "Run `vcu model list` and `vcu model set`.",
            Self::BudgetExceeded => "Raise --budget or use a smaller snapshot mode (a11y/text).",
            Self::InvalidInput => "Check required flags and JSON shapes; see `vcu <cmd> --help`.",
            Self::BrowserUnavailable => "Install Chrome/Edge or use `--backend mock` for local POC.",
            Self::CdpConnectFailed => "Start Chrome/Edge with remote debugging or enable chrome://inspect/#remote-debugging.",
            Self::ActionFailed => "Inspect result.error.detail and retry after snapshot.",
            Self::IdempotencyConflict => "Reuse the same idempotency_key only with identical action payloads.",
            Self::Internal => "See daemon logs; restart daemon if state is corrupt.",
            Self::NotImplemented => "This capability is not in the current MVP build.",
            Self::AccessibilityDenied => "Grant Accessibility once: 系统设置 → 隐私与安全 → 辅助功能, then retry. Do not click Edge Allow debugging for the desktop surface.",
            Self::AppDenied => "This app is blocked by VCU policy (WeChat/微信) or is outside the desktop allowlist.",
        }
    }
}

#[derive(Debug, Error)]
pub enum VcuError {
    #[error("{code:?}: {message}")]
    Coded {
        code: ErrorCode,
        message: String,
        detail: Option<String>,
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

impl VcuError {
    pub fn coded(code: ErrorCode, message: impl Into<String>) -> Self {
        Self::Coded {
            code,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(code: ErrorCode, message: impl Into<String>, detail: impl Into<String>) -> Self {
        Self::Coded {
            code,
            message: message.into(),
            detail: Some(detail.into()),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Coded { code, .. } => *code,
            Self::Io(_) => ErrorCode::Internal,
            Self::Json(_) => ErrorCode::InvalidInput,
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Coded { message, .. } => message.clone(),
            other => other.to_string(),
        }
    }

    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Coded { detail, .. } => detail.clone(),
            Self::Io(e) => Some(e.to_string()),
            Self::Json(e) => Some(e.to_string()),
        }
    }

    pub fn repair_hint(&self) -> String {
        self.code().default_hint().to_string()
    }
}

pub type VcuResult<T> = Result<T, VcuError>;
