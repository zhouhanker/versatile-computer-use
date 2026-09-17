use crate::error::{ErrorCode, VcuError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepairHint {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    pub repair_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorBody>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_revision: Option<u64>,
}

impl<T: Serialize> Envelope<T> {
    pub fn ok(data: T) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            session_revision: None,
        }
    }

    pub fn ok_rev(data: T, rev: u64) -> Self {
        Self {
            ok: true,
            data: Some(data),
            error: None,
            session_revision: Some(rev),
        }
    }

    pub fn from_error(err: &VcuError) -> Envelope<serde_json::Value> {
        Envelope {
            ok: false,
            data: None,
            error: Some(ErrorBody {
                code: err.code(),
                message: err.message(),
                detail: err.detail(),
                repair_hint: err.repair_hint(),
            }),
            session_revision: None,
        }
    }
}
