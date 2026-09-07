use crate::application::ports::{CredentialRepository, PortError};
use crate::domain::credential::Credential;
use std::sync::Arc;

/// Use-case: manage the single FF14 login credential.
pub struct CredentialService {
    repository: Arc<dyn CredentialRepository>,
}

impl CredentialService {
    pub fn new(repository: Arc<dyn CredentialRepository>) -> Self {
        Self { repository }
    }

    /// Load the stored credential.
    pub fn load(&self) -> Result<Option<Credential>, PortError> {
        self.repository.load()
    }

    /// Save a credential (overwrites any existing).
    pub fn save(&self, credential: &Credential) -> Result<(), PortError> {
        self.repository.save(credential)
    }

    /// Delete the stored credential.
    pub fn delete(&self) -> Result<(), PortError> {
        self.repository.delete()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockCredentialRepository;

    // --- テストリスト ---
    // [x] リポジトリが空のとき load() は Ok(None) を返す
    // [x] save() して load() すると Ok(Some(credential)) が返る
    // [x] save() を2回呼ぶと2回目も成功する（上書き）
    // [x] delete() 後に load() すると Ok(None) が返る
    // [x] リポジトリがエラーを返すとき load() はそのエラーを伝播する

    #[test]
    fn load_returns_none_when_repository_is_empty() {
        let mut mock = MockCredentialRepository::new();
        mock.expect_load()
            .times(1)
            .returning(|| Ok(None));

        let service = CredentialService::new(Arc::new(mock));
        let result = service.load();
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn load_returns_credential_after_save() {
        let credential = Credential::new("FF14 Main", "p@ssw0rd", None::<String>).unwrap();
        let mut mock = MockCredentialRepository::new();
        mock.expect_save()
            .times(1)
            .returning(|_| Ok(()));
        mock.expect_load()
            .times(1)
            .returning(|| {
                Ok(Some(
                    Credential::new("FF14 Main", "p@ssw0rd", None::<String>).unwrap(),
                ))
            });

        let service = CredentialService::new(Arc::new(mock));
        service.save(&credential).unwrap();
        let loaded = service.load().unwrap().unwrap();
        assert_eq!(loaded.account_name(), "FF14 Main");
    }

    #[test]
    fn save_twice_succeeds() {
        let cred1 = Credential::new("FF14 Main", "old_pass", None::<String>).unwrap();
        let cred2 = Credential::new("FF14 Main", "new_pass", None::<String>).unwrap();

        let mut mock = MockCredentialRepository::new();
        mock.expect_save().times(2).returning(|_| Ok(()));

        let service = CredentialService::new(Arc::new(mock));
        assert!(service.save(&cred1).is_ok());
        assert!(service.save(&cred2).is_ok());
    }

    #[test]
    fn load_returns_none_after_delete() {
        let mut mock = MockCredentialRepository::new();
        mock.expect_delete().times(1).returning(|| Ok(()));
        mock.expect_load().times(1).returning(|| Ok(None));

        let service = CredentialService::new(Arc::new(mock));
        service.delete().unwrap();
        let result = service.load().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn load_propagates_repository_error() {
        let mut mock = MockCredentialRepository::new();
        mock.expect_load()
            .times(1)
            .returning(|| Err(PortError::Storage("disk error".to_string())));

        let service = CredentialService::new(Arc::new(mock));
        let result = service.load();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("storage error"));
    }
}
