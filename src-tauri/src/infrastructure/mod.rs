// Infrastructure layer — concrete implementations of application ports.
// Depends on external crates: keyring, totp-rs, windows-rs.
pub mod keyring_repository;
pub mod totp_generator;

#[cfg(target_os = "windows")]
pub mod win32;
