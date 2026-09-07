/// Tauri command handlers.
///
/// Each command is a thin wrapper that delegates to application services.
/// Business logic lives in the application layer, not here.
use crate::presentation::state::AppState;
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::State;

// ─── DTOs ────────────────────────────────────────────────────────────────────

/// Credential data transferred between frontend and backend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CredentialDto {
    pub account_name: String,
    pub password: String,
    pub totp_seed: Option<String>,
}

/// TOTP code response returned to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TotpCodeDto {
    pub code: String,
    pub remaining_seconds: u8,
}

// ─── Pure command logic (testable without Tauri runtime) ─────────────────────

/// Load the stored credential. Returns `None` if nothing is saved yet.
pub fn load_credential_inner(
    state: &AppState,
) -> Result<Option<CredentialDto>, String> {
    state
        .credential_service
        .load()
        .map(|opt| {
            opt.map(|cred| CredentialDto {
                account_name: cred.account_name().to_owned(),
                password: cred.password().expose_secret().to_owned(),
                totp_seed: cred
                    .totp_seed()
                    .map(|s| s.expose_secret().to_owned()),
            })
        })
        .map_err(|e| e.to_string())
}

/// Save a credential (overwrites any existing).
pub fn save_credential_inner(
    state: &AppState,
    dto: CredentialDto,
) -> Result<(), String> {
    let credential =
        crate::domain::credential::Credential::new(dto.account_name, dto.password, dto.totp_seed)
            .map_err(|e| e.to_string())?;
    state
        .credential_service
        .save(&credential)
        .map_err(|e| e.to_string())
}

/// Generate a TOTP code from the stored seed.
/// Returns an error if no credential with a TOTP seed is saved.
pub fn generate_totp_inner(state: &AppState) -> Result<TotpCodeDto, String> {
    let cred = state
        .credential_service
        .load()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "認証情報が登録されていません".to_string())?;

    let seed = cred
        .totp_seed()
        .ok_or_else(|| "TOTPシードが登録されていません".to_string())?;

    state
        .totp_service
        .generate(seed.expose_secret())
        .map(|code| TotpCodeDto {
            code: code.code().to_owned(),
            remaining_seconds: code.remaining_seconds(),
        })
        .map_err(|e| e.to_string())
}

/// Delete the stored credential.
pub fn delete_credential_inner(state: &AppState) -> Result<(), String> {
    state
        .credential_service
        .delete()
        .map_err(|e| e.to_string())
}

// ─── Tauri command wrappers ───────────────────────────────────────────────────

#[tauri::command]
pub fn get_credential(state: State<AppState>) -> Result<Option<CredentialDto>, String> {
    load_credential_inner(&state)
}

#[tauri::command]
pub fn save_credential(
    state: State<AppState>,
    dto: CredentialDto,
) -> Result<(), String> {
    save_credential_inner(&state, dto)
}

#[tauri::command]
pub fn generate_totp(state: State<AppState>) -> Result<TotpCodeDto, String> {
    generate_totp_inner(&state)
}

#[tauri::command]
pub fn delete_credential(state: State<AppState>) -> Result<(), String> {
    delete_credential_inner(&state)
}

#[tauri::command]
pub fn get_auto_submit(state: State<AppState>) -> bool {
    state.auto_submit.load(Ordering::Relaxed)
}

#[tauri::command]
pub fn set_auto_submit(state: State<AppState>, enabled: bool) {
    state.auto_submit.store(enabled, Ordering::Relaxed);
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{
        credential_service::CredentialService,
        ports::{MockCredentialRepository, MockTotpProvider},
        totp_service::TotpService,
    };
    use crate::domain::totp::TotpCode;
    use crate::presentation::state::AppState;
    use std::sync::Arc;

    // --- テストリスト ---
    // [x] load_credential_inner: 未登録のとき Ok(None) を返す
    // [x] load_credential_inner: 登録済みのとき Ok(Some(CredentialDto)) を返す
    // [x] load_credential_inner: totp_seed あり の場合も正しく返す
    // [x] save_credential_inner: 正常な DTO で Ok(()) を返す
    // [x] save_credential_inner: 空のアカウント名で Err を返す
    // [x] save_credential_inner: 空のパスワードで Err を返す
    // [x] generate_totp_inner: 登録済みシードから TotpCodeDto を返す
    // [x] generate_totp_inner: 未登録のとき Err を返す
    // [x] generate_totp_inner: totp_seed なしの登録では Err を返す
    // [x] delete_credential_inner: Ok(()) を返す

    fn make_state(
        mock_repo: MockCredentialRepository,
        mock_totp: MockTotpProvider,
    ) -> AppState {
        let cred_service = Arc::new(CredentialService::new(Arc::new(mock_repo)));
        let totp_service = Arc::new(TotpService::new(Arc::new(mock_totp)));
        AppState::new(cred_service, totp_service)
    }

    #[test]
    fn load_credential_inner_returns_none_when_empty() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| Ok(None));
        let state = make_state(repo, MockTotpProvider::new());

        let result = load_credential_inner(&state);
        assert_eq!(result, Ok(None));
    }

    #[test]
    fn load_credential_inner_returns_dto_when_saved() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| {
            Ok(Some(
                crate::domain::credential::Credential::new(
                    "FF14 Main",
                    "p@ssw0rd",
                    None::<String>,
                )
                .unwrap(),
            ))
        });
        let state = make_state(repo, MockTotpProvider::new());

        let result = load_credential_inner(&state).unwrap().unwrap();
        assert_eq!(result.account_name, "FF14 Main");
        assert_eq!(result.password, "p@ssw0rd");
        assert!(result.totp_seed.is_none());
    }

    #[test]
    fn load_credential_inner_returns_totp_seed_when_present() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| {
            Ok(Some(
                crate::domain::credential::Credential::new(
                    "FF14 Main",
                    "p@ssw0rd",
                    Some("TESTSEED"),
                )
                .unwrap(),
            ))
        });
        let state = make_state(repo, MockTotpProvider::new());

        let result = load_credential_inner(&state).unwrap().unwrap();
        assert_eq!(result.totp_seed, Some("TESTSEED".to_string()));
    }

    #[test]
    fn save_credential_inner_succeeds_with_valid_dto() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_save().returning(|_| Ok(()));
        let state = make_state(repo, MockTotpProvider::new());

        let dto = CredentialDto {
            account_name: "FF14 Main".to_string(),
            password: "p@ssw0rd".to_string(),
            totp_seed: None,
        };
        let result = save_credential_inner(&state, dto);
        assert!(result.is_ok());
    }

    #[test]
    fn save_credential_inner_errors_on_empty_account_name() {
        let state = make_state(MockCredentialRepository::new(), MockTotpProvider::new());

        let dto = CredentialDto {
            account_name: "".to_string(),
            password: "p@ssw0rd".to_string(),
            totp_seed: None,
        };
        let result = save_credential_inner(&state, dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("account name"));
    }

    #[test]
    fn save_credential_inner_errors_on_empty_password() {
        let state = make_state(MockCredentialRepository::new(), MockTotpProvider::new());

        let dto = CredentialDto {
            account_name: "FF14 Main".to_string(),
            password: "".to_string(),
            totp_seed: None,
        };
        let result = save_credential_inner(&state, dto);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("password"));
    }

    #[test]
    fn generate_totp_inner_returns_code_from_seed() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| {
            Ok(Some(
                crate::domain::credential::Credential::new(
                    "FF14 Main",
                    "p@ssw0rd",
                    Some("GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ"),
                )
                .unwrap(),
            ))
        });
        let mut totp = MockTotpProvider::new();
        totp.expect_generate()
            .returning(|_| Ok(TotpCode::new("123456", 20).unwrap()));
        let state = make_state(repo, totp);

        let result = generate_totp_inner(&state).unwrap();
        assert_eq!(result.code, "123456");
        assert_eq!(result.remaining_seconds, 20);
    }

    #[test]
    fn generate_totp_inner_errors_when_no_credential() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| Ok(None));
        let state = make_state(repo, MockTotpProvider::new());

        let result = generate_totp_inner(&state);
        assert!(result.is_err());
    }

    #[test]
    fn generate_totp_inner_errors_when_no_totp_seed() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| {
            Ok(Some(
                crate::domain::credential::Credential::new(
                    "FF14 Main",
                    "p@ssw0rd",
                    None::<String>,
                )
                .unwrap(),
            ))
        });
        let state = make_state(repo, MockTotpProvider::new());

        let result = generate_totp_inner(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("TOTP"));
    }

    #[test]
    fn delete_credential_inner_succeeds() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_delete().returning(|| Ok(()));
        let state = make_state(repo, MockTotpProvider::new());

        let result = delete_credential_inner(&state);
        assert!(result.is_ok());
    }
}
