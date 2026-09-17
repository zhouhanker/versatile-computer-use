//! VCU daemon library: sessions, browser backends, HTTP IPC, vision.
pub mod api;
pub mod audit;
pub mod app;
pub mod browser;
pub mod doctor;
pub mod runtime;
pub mod state;
pub mod vision;

pub use runtime::{start_daemon, DaemonHandle};
pub use state::AppState;
