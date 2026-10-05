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

#[tauri::command]
pub fn get_autostart() -> bool {
    #[cfg(target_os = "windows")]
    {
        crate::infrastructure::win32::autostart::is_autostart_enabled()
    }
    #[cfg(not(target_os = "windows"))]
    {
        false
    }
}

#[tauri::command]
pub fn set_autostart(enabled: bool) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        crate::infrastructure::win32::autostart::set_autostart_enabled(enabled)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateCheckResultDto {
    pub should_update: bool,
    pub version: Option<String>,
    pub body: Option<String>,
}

#[tauri::command]
pub async fn check_for_update(app: tauri::AppHandle) -> Result<UpdateCheckResultDto, String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    match updater.check().await {
        Ok(Some(update)) => Ok(UpdateCheckResultDto {
            should_update: true,
            version: Some(update.version),
            body: update.body,
        }),
        Ok(None) => Ok(UpdateCheckResultDto {
            should_update: false,
            version: None,
            body: None,
        }),
        Err(e) => Err(format!("更新チェックエラー: {e}")),
    }
}

#[tauri::command]
pub async fn install_update(app: tauri::AppHandle) -> Result<(), String> {
    use tauri_plugin_updater::UpdaterExt;
    let updater = app.updater().map_err(|e| e.to_string())?;
    if let Some(update) = updater.check().await.map_err(|e| e.to_string())? {
        let mut downloaded = 0;
        update
            .download_and_install(
                |chunk_length, content_length| {
                    downloaded += chunk_length;
                    let _ = (downloaded, content_length);
                },
                || {},
            )
            .await
            .map_err(|e| format!("アップデートインストール失敗: {e}"))?;

        app.restart();
    }
    Ok(())
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ParsedTotpAccountDto {
    pub secret_base32: String,
    pub name: Option<String>,
    pub issuer: Option<String>,
}

#[tauri::command]
pub fn parse_totp_input(input: String) -> Result<Vec<ParsedTotpAccountDto>, String> {
    use crate::domain::totp_parser;
    totp_parser::parse_totp_input(&input).map(|list| {
        list.into_iter()
            .map(|acc| ParsedTotpAccountDto {
                secret_base32: acc.secret_base32,
                name: acc.name,
                issuer: acc.issuer,
            })
            .collect()
    })
}

#[tauri::command]
pub fn trigger_autofill(state: State<AppState>) -> Result<(), String> {
    trigger_autofill_inner(&state)
}

pub fn trigger_autofill_inner(state: &AppState) -> Result<(), String> {
    use crate::infrastructure::win32::{input, watcher};
    use secrecy::ExposeSecret;

    // 0. Find and focus FF14 launcher window
    if let Some(hwnd) = watcher::find_launcher_hwnd() {
        let _ = watcher::focus_launcher_window(hwnd);
    } else {
        return Err("FF14ランチャーのウィンドウが見つかりません。ランチャーを起動してからお試しください。".to_string());
    }

    let cred = state
        .credential_service
        .load()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "認証情報が登録されていません".to_string())?;

    // 1. Clear any existing text in Password field (Ctrl+A -> Backspace) and send password
    input::send_clear_and_type(cred.password().expose_secret(), None)
        .map_err(|e| format!("パスワード入力失敗: {e}。ランチャーが管理者権限で起動している場合は本アプリも「管理者として実行」してください。"))?;

    // 2. Tab to OTP field
    input::send_tab(None).map_err(|e| format!("Tab送信失敗: {e}"))?;

    // 3. Clear any existing text in OTP field and send TOTP if seed is registered
    if let Some(seed) = cred.totp_seed() {
        let totp = state
            .totp_service
            .generate(seed.expose_secret())
            .map_err(|e| e.to_string())?;
        input::send_clear_and_type(totp.code(), None)
            .map_err(|e| format!("OTP入力失敗: {e}"))?;
    }

    // 4. Auto-submit Enter if enabled
    if state.auto_submit.load(Ordering::Relaxed) {
        input::send_enter(None).map_err(|e| format!("Enter送信失敗: {e}"))?;
    }

    Ok(())
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

    #[test]
    fn trigger_autofill_inner_errors_when_no_credential() {
        let mut repo = MockCredentialRepository::new();
        repo.expect_load().returning(|| Ok(None));
        let state = make_state(repo, MockTotpProvider::new());

        let result = trigger_autofill_inner(&state);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("FF14ランチャー"));
    }
}
