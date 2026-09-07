/// Thread-safe shared application state injected into all Tauri commands.
use crate::application::{
    credential_service::CredentialService, totp_service::TotpService,
};
use std::sync::Arc;

pub struct AppState {
    pub credential_service: Arc<CredentialService>,
    pub totp_service: Arc<TotpService>,
    /// Whether to automatically send Enter after filling credentials.
    pub auto_submit: std::sync::atomic::AtomicBool,
}

impl AppState {
    pub fn new(
        credential_service: Arc<CredentialService>,
        totp_service: Arc<TotpService>,
    ) -> Self {
        Self {
            credential_service,
            totp_service,
            auto_submit: std::sync::atomic::AtomicBool::new(false),
        }
    }
}
