// Domain layer: business entities, value objects, and domain errors.
// No external infrastructure dependencies.
pub mod domain;

// Application layer: use-cases and service interfaces (traits).
pub mod application;

// Infrastructure layer: concrete implementations (keyring, totp-rs, win32).
pub mod infrastructure;

// Presentation layer: Tauri commands wiring everything together.
pub mod presentation;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    use crate::application::{
        credential_service::CredentialService, totp_service::TotpService,
    };
    use crate::infrastructure::{
        keyring_repository::KeyringRepository, totp_generator::TotpGenerator,
    };
    use crate::presentation::{
        commands::{
            delete_credential, generate_totp, get_auto_submit, get_credential, save_credential,
            set_auto_submit,
        },
        state::AppState,
    };
    use std::sync::Arc;

    let repo = Arc::new(KeyringRepository::new("ff14-companion"));
    let totp_gen = Arc::new(TotpGenerator::new());
    let cred_service = Arc::new(CredentialService::new(repo));
    let totp_service = Arc::new(TotpService::new(totp_gen));
    let app_state = AppState::new(cred_service, totp_service);

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            get_credential,
            save_credential,
            delete_credential,
            generate_totp,
            get_auto_submit,
            set_auto_submit,
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
