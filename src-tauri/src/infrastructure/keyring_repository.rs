use crate::application::ports::{CredentialRepository, PortError};
use crate::domain::credential::Credential;
use keyring::Entry;
use secrecy::ExposeSecret;

/// Windows Credential Manager (DPAPI) backed credential storage.
/// Each field (password, totp_seed) is stored as a separate keyring entry
/// under the service name "ff14-companion".
pub struct KeyringRepository {
    service: String,
}

const KEY_PASSWORD: &str = "password";
const KEY_TOTP_SEED: &str = "totp_seed";
const KEY_ACCOUNT_NAME: &str = "account_name";
const TOTP_ABSENT_SENTINEL: &str = "__NONE__";

impl KeyringRepository {
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }

    fn entry(&self, key: &str) -> Result<Entry, PortError> {
        Entry::new(&self.service, key)
            .map_err(|e| PortError::Storage(format!("failed to open keyring entry '{key}': {e}")))
    }
}

impl CredentialRepository for KeyringRepository {
    fn load(&self) -> Result<Option<Credential>, PortError> {
        let account_name = match self.entry(KEY_ACCOUNT_NAME)?.get_password() {
            Ok(v) => v,
            Err(keyring::Error::NoEntry) => {
                // If primary service has no entry and we are using "ff14-quickauth", check legacy "ff14-companion"
                if self.service == "ff14-quickauth" {
                    let legacy_repo = KeyringRepository::new("ff14-companion");
                    if let Ok(Some(legacy_cred)) = legacy_repo.load() {
                        // Migrate credential to new service automatically
                        let _ = self.save(&legacy_cred);
                        return Ok(Some(legacy_cred));
                    }
                }
                return Ok(None);
            }
            Err(e) => return Err(PortError::Storage(format!("failed to read account_name: {e}"))),
        };

        let password = self
            .entry(KEY_PASSWORD)?
            .get_password()
            .map_err(|e| PortError::Storage(format!("failed to read password: {e}")))?;

        let totp_seed_raw = self
            .entry(KEY_TOTP_SEED)?
            .get_password()
            .map_err(|e| PortError::Storage(format!("failed to read totp_seed: {e}")))?;

        let totp_seed: Option<String> = if totp_seed_raw == TOTP_ABSENT_SENTINEL {
            None
        } else {
            Some(totp_seed_raw)
        };

        Credential::new(account_name, password, totp_seed)
            .map(Some)
            .map_err(|e| PortError::Storage(format!("invalid stored credential: {e}")))
    }

    fn save(&self, credential: &Credential) -> Result<(), PortError> {
        self.entry(KEY_ACCOUNT_NAME)?
            .set_password(credential.account_name())
            .map_err(|e| PortError::Storage(format!("failed to write account_name: {e}")))?;

        self.entry(KEY_PASSWORD)?
            .set_password(credential.password().expose_secret())
            .map_err(|e| PortError::Storage(format!("failed to write password: {e}")))?;

        let totp_seed_val = credential
            .totp_seed()
            .map(|s| s.expose_secret().to_owned())
            .unwrap_or_else(|| TOTP_ABSENT_SENTINEL.to_string());

        self.entry(KEY_TOTP_SEED)?
            .set_password(&totp_seed_val)
            .map_err(|e| PortError::Storage(format!("failed to write totp_seed: {e}")))?;

        Ok(())
    }

    fn delete(&self) -> Result<(), PortError> {
        for key in &[KEY_ACCOUNT_NAME, KEY_PASSWORD, KEY_TOTP_SEED] {
            match self.entry(key)?.delete_credential() {
                Ok(()) | Err(keyring::Error::NoEntry) => {}
                Err(e) => {
                    return Err(PortError::Storage(format!(
                        "failed to delete '{key}': {e}"
                    )))
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- テストリスト ---
    // These are integration tests that write to the real Windows Credential Manager.
    // They use a unique test-scoped service name to avoid polluting production entries.
    //
    // [x] save() + load() → 同じ Credential が返る
    // [x] save() + load() → totp_seed あり の場合も正しく返る
    // [x] load() → エントリ未存在のとき Ok(None) を返す
    // [x] save() を2回呼ぶと load() が2回目の値を返す（上書き）
    // [x] delete() 後に load() すると Ok(None) を返す

    fn test_repo(suffix: &str) -> KeyringRepository {
        KeyringRepository::new(format!("ff14-companion-test-{suffix}"))
    }

    fn cleanup(repo: &KeyringRepository) {
        let _ = repo.delete();
    }

    #[test]
    fn save_and_load_returns_same_credential() {
        let repo = test_repo("save-load");
        cleanup(&repo);

        let cred = Credential::new("TestAccount", "TestPass123", None::<String>).unwrap();
        repo.save(&cred).unwrap();

        let loaded = repo.load().unwrap().unwrap();
        assert_eq!(loaded.account_name(), "TestAccount");

        cleanup(&repo);
    }

    #[test]
    fn save_and_load_preserves_totp_seed() {
        let repo = test_repo("save-load-totp");
        cleanup(&repo);

        let cred =
            Credential::new("TestAccount", "TestPass123", Some("JBSWY3DPEHPK3PXP")).unwrap();
        repo.save(&cred).unwrap();

        let loaded = repo.load().unwrap().unwrap();
        assert!(loaded.has_totp());

        cleanup(&repo);
    }

    #[test]
    fn load_returns_none_when_no_entry_exists() {
        let repo = test_repo("load-none");
        cleanup(&repo);

        let result = repo.load().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn second_save_overwrites_first() {
        let repo = test_repo("overwrite");
        cleanup(&repo);

        let cred1 = Credential::new("TestAccount", "OldPass", None::<String>).unwrap();
        let cred2 = Credential::new("TestAccount", "NewPass", None::<String>).unwrap();
        repo.save(&cred1).unwrap();
        repo.save(&cred2).unwrap();

        let loaded = repo.load().unwrap().unwrap();
        use secrecy::ExposeSecret;
        assert_eq!(loaded.password().expose_secret(), "NewPass");

        cleanup(&repo);
    }

    #[test]
    fn load_returns_none_after_delete() {
        let repo = test_repo("delete");
        cleanup(&repo);

        let cred = Credential::new("TestAccount", "TestPass123", None::<String>).unwrap();
        repo.save(&cred).unwrap();
        repo.delete().unwrap();

        let result = repo.load().unwrap();
        assert!(result.is_none());
    }
}
