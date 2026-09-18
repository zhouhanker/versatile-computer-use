use vcu_core::{ErrorCode, Envelope, SessionPolicy, SnapshotMode, VcuError, VisionPolicy};

#[test]
fn every_error_code_has_hint() {
    for code in [
        ErrorCode::DaemonNotRunning,
        ErrorCode::DaemonAlreadyRunning,
        ErrorCode::DaemonAuthFailed,
        ErrorCode::ExtensionDisconnected,
        ErrorCode::SessionNotFound,
        ErrorCode::SessionClosed,
        ErrorCode::BorrowRequired,
        ErrorCode::TabNotFound,
        ErrorCode::TabAlreadyBorrowed,
        ErrorCode::FocusPolicyViolation,
        ErrorCode::OsCursorDenied,
        ErrorCode::VisionProviderRequired,
        ErrorCode::VisionCallFailed,
        ErrorCode::ModelNotFound,
        ErrorCode::BudgetExceeded,
        ErrorCode::InvalidInput,
        ErrorCode::BrowserUnavailable,
        ErrorCode::CdpConnectFailed,
        ErrorCode::ActionFailed,
        ErrorCode::IdempotencyConflict,
        ErrorCode::Internal,
        ErrorCode::NotImplemented,
        ErrorCode::AccessibilityDenied,
        ErrorCode::AppDenied,
    ] {
        assert!(!code.default_hint().is_empty(), "{code:?}");
        let e = VcuError::coded(code, "x");
        let env = Envelope::<serde_json::Value>::from_error(&e);
        assert_eq!(env.ok, false);
        assert!(env.error.is_some());
    }
}

#[test]
fn session_policy_defaults() {
    let p = SessionPolicy::default();
    assert!(matches!(p.os_cursor, vcu_core::OsCursorPolicy::Deny));
    assert!(p.borrow_required_for_user_tabs);
    assert_eq!(VisionPolicy::default(), VisionPolicy::DomFirst);
    assert_eq!(SnapshotMode::parse("full"), Some(SnapshotMode::Full));
}
