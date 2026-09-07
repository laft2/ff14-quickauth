// Domain layer: business entities, value objects, and domain errors.
// No external infrastructure dependencies.
pub mod domain;

// Application layer: use-cases and service interfaces (traits).
pub mod application;

// Infrastructure layer: concrete implementations (keyring, totp-rs, win32).
// pub mod infrastructure;

// Presentation layer: Tauri commands wiring everything together.
// pub mod presentation;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
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
