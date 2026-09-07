/// Output port traits (Dependency Inversion Principle).
/// Infrastructure implements these; Application depends on them.
use crate::domain::{credential::Credential, totp::TotpCode};
use std::result::Result;

/// Error type for port operations.
#[derive(Debug, thiserror::Error)]
pub enum PortError {
    #[error("storage error: {0}")]
    Storage(String),

    #[error("TOTP generation error: {0}")]
    Totp(String),
}

/// Persistent storage for the single FF14 credential.
/// Implementations: [`crate::infrastructure::keyring_repository::KeyringRepository`]
#[cfg_attr(test, mockall::automock)]
pub trait CredentialRepository: Send + Sync {
    /// Load the stored credential, or `None` if not set.
    fn load(&self) -> Result<Option<Credential>, PortError>;

    /// Persist the credential (overwrites any existing).
    fn save(&self, credential: &Credential) -> Result<(), PortError>;

    /// Remove the stored credential.
    fn delete(&self) -> Result<(), PortError>;
}

/// Generates TOTP codes from a Base32-encoded seed.
/// Implementations: [`crate::infrastructure::totp_generator::TotpGenerator`]
#[cfg_attr(test, mockall::automock)]
pub trait TotpProvider: Send + Sync {
    /// Generate a current TOTP code from the given Base32 seed.
    fn generate(&self, seed: &str) -> Result<TotpCode, PortError>;
}
