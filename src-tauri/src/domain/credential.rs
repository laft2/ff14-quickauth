use crate::domain::error::DomainError;
use secrecy::SecretString;

/// FF14 launcher login credential entity.
///
/// Invariants:
/// - `account_name` must not be empty
/// - `password` must not be empty
/// - `totp_seed` is optional (Some if TOTP is enabled)
#[derive(Debug)]
pub struct Credential {
    account_name: String,
    password: SecretString,
    totp_seed: Option<SecretString>,
}

impl Credential {
    /// Creates a new `Credential`.
    ///
    /// # Errors
    /// - [`DomainError::EmptyAccountName`] if `account_name` is empty or blank
    /// - [`DomainError::EmptyPassword`] if `password` is empty or blank
    pub fn new(
        account_name: impl Into<String>,
        password: impl Into<String>,
        totp_seed: Option<impl Into<String>>,
    ) -> Result<Self, DomainError> {
        let account_name = account_name.into();
        if account_name.trim().is_empty() {
            return Err(DomainError::EmptyAccountName);
        }

        let password = password.into();
        if password.trim().is_empty() {
            return Err(DomainError::EmptyPassword);
        }

        Ok(Self {
            account_name,
            password: SecretString::from(password),
            totp_seed: totp_seed.map(|s| SecretString::from(s.into())),
        })
    }

    pub fn account_name(&self) -> &str {
        &self.account_name
    }

    pub fn password(&self) -> &SecretString {
        &self.password
    }

    pub fn totp_seed(&self) -> Option<&SecretString> {
        self.totp_seed.as_ref()
    }

    pub fn has_totp(&self) -> bool {
        self.totp_seed.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;

    // --- テストリスト ---
    // [x] Credential::new でアカウント名・パスワードを持つ値が作れる
    // [x] Credential::new でアカウント名が空のときエラーになる
    // [x] Credential::new でアカウント名が空白のみのときエラーになる
    // [x] Credential::new でパスワードが空のときエラーになる
    // [x] Credential::new でパスワードが空白のみのときエラーになる
    // [x] Credential は totp_seed を持てる（Some）
    // [x] Credential は totp_seed を持たなくてもよい（None）
    // [x] has_totp は totp_seed が Some のとき true を返す
    // [x] has_totp は totp_seed が None のとき false を返す

    #[test]
    fn new_with_valid_args_creates_credential() {
        let cred = Credential::new("My Account", "s3cr3t", None::<String>);
        assert!(cred.is_ok());
        let cred = cred.unwrap();
        assert_eq!(cred.account_name(), "My Account");
        assert_eq!(cred.password().expose_secret(), "s3cr3t");
    }

    #[test]
    fn new_with_empty_account_name_returns_error() {
        let result = Credential::new("", "s3cr3t", None::<String>);
        assert_eq!(result.unwrap_err(), DomainError::EmptyAccountName);
    }

    #[test]
    fn new_with_blank_account_name_returns_error() {
        let result = Credential::new("   ", "s3cr3t", None::<String>);
        assert_eq!(result.unwrap_err(), DomainError::EmptyAccountName);
    }

    #[test]
    fn new_with_empty_password_returns_error() {
        let result = Credential::new("My Account", "", None::<String>);
        assert_eq!(result.unwrap_err(), DomainError::EmptyPassword);
    }

    #[test]
    fn new_with_blank_password_returns_error() {
        let result = Credential::new("My Account", "   ", None::<String>);
        assert_eq!(result.unwrap_err(), DomainError::EmptyPassword);
    }

    #[test]
    fn new_with_totp_seed_stores_seed() {
        let cred = Credential::new("My Account", "s3cr3t", Some("JBSWY3DPEHPK3PXP")).unwrap();
        assert!(cred.totp_seed().is_some());
        assert_eq!(
            cred.totp_seed().unwrap().expose_secret(),
            "JBSWY3DPEHPK3PXP"
        );
    }

    #[test]
    fn new_without_totp_seed_has_none() {
        let cred = Credential::new("My Account", "s3cr3t", None::<String>).unwrap();
        assert!(cred.totp_seed().is_none());
    }

    #[test]
    fn has_totp_returns_true_when_seed_is_some() {
        let cred = Credential::new("My Account", "s3cr3t", Some("JBSWY3DPEHPK3PXP")).unwrap();
        assert!(cred.has_totp());
    }

    #[test]
    fn has_totp_returns_false_when_seed_is_none() {
        let cred = Credential::new("My Account", "s3cr3t", None::<String>).unwrap();
        assert!(!cred.has_totp());
    }
}
