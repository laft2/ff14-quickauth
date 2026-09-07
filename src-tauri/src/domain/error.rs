/// Domain-level errors.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum DomainError {
    #[error("account name must not be empty")]
    EmptyAccountName,

    #[error("password must not be empty")]
    EmptyPassword,

    #[error("TOTP code must be exactly 6 digits, got: {0}")]
    InvalidTotpCodeLength(usize),

    #[error("remaining seconds must be in range 0..=29, got: {0}")]
    InvalidRemainingSeconds(u8),
}
