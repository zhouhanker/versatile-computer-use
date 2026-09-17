//! VCU core protocol, errors, and user configuration.
pub mod config;
pub mod error;
pub mod id;
pub mod protocol;
pub mod result;

pub use config::{ModelConfig, UserConfig, VisionPolicy, VcuPaths};
pub use error::{ErrorCode, VcuError, VcuResult};
pub use id::new_id;
pub use protocol::*;
pub use result::{Envelope, RepairHint};
