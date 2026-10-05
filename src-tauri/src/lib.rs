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
            delete_credential, generate_totp, get_auto_submit, get_autostart, get_credential,
            parse_totp_input, save_credential, set_auto_submit, set_autostart, trigger_autofill,
        },
        state::AppState,
    };
    use std::sync::Arc;
    use tauri::{
        menu::{MenuBuilder, MenuItemBuilder},
        tray::{TrayIconBuilder, TrayIconEvent},
        Emitter, Manager, WindowEvent,
    };

    let repo = Arc::new(KeyringRepository::new("ff14-quickauth"));
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
            get_autostart,
            set_autostart,
            trigger_autofill,
            parse_totp_input,
        ])
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // System Tray Menu Setup
            let show_item = MenuItemBuilder::with_id("show", "設定を開く").build(app)?;
            let fill_item = MenuItemBuilder::with_id("autofill", "今すぐ自動入力").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "終了").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show_item, &fill_item, &quit_item])
                .build()?;

            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "autofill" => {
                        let state = app.state::<AppState>();
                        let _ = crate::presentation::commands::trigger_autofill_inner(&state);
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: tauri::tray::MouseButton::Left,
                        button_state: tauri::tray::MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                })
                .build(app)?;

            // Background Launcher Process Watcher (Windows only)
            #[cfg(target_os = "windows")]
            {
                let handle = app.handle().clone();
                std::thread::spawn(move || {
                    use crate::infrastructure::win32::watcher::{
                        find_main_launcher_window, get_process_exe_path, is_ff14_launcher,
                    };
                    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

                    let mut was_detected = false;
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(1000));
                        let hwnd = unsafe { GetForegroundWindow() };
                        let is_detected = get_process_exe_path(hwnd)
                            .map(|path| is_ff14_launcher(&path))
                            .unwrap_or(false);

                        if is_detected {
                            if !was_detected {
                                was_detected = true;
                                let _ = handle.emit("launcher-detected", ());
                            }

                            if let Some((_l_hwnd, rect)) = find_main_launcher_window() {
                                let l_width = rect.right - rect.left;
                                let overlay_x = rect.left + (l_width - 260) / 2;
                                let overlay_y = rect.top + 35;

                                let handle_clone = handle.clone();
                                let _ = handle.run_on_main_thread(move || {
                                    use tauri::{PhysicalPosition, WebviewWindowBuilder};
                                    let pos = PhysicalPosition::new(overlay_x, overlay_y);

                                    if let Some(overlay) = handle_clone.get_webview_window("overlay") {
                                        let _ = overlay.set_position(pos);
                                        let _ = overlay.show();
                                        let _ = overlay.set_always_on_top(true);
                                    } else {
                                        if let Ok(overlay) = WebviewWindowBuilder::new(
                                            &handle_clone,
                                            "overlay",
                                            tauri::WebviewUrl::App("overlay.html".into()),
                                        )
                                        .title("FF14 QuickAuth Overlay")
                                        .inner_size(260.0, 70.0)
                                        .position(overlay_x as f64, overlay_y as f64)
                                        .decorations(false)
                                        .transparent(true)
                                        .always_on_top(true)
                                        .resizable(false)
                                        .build()
                                        {
                                            let _ = overlay.set_position(pos);
                                        }
                                    }
                                });
                            }
                        } else if was_detected {
                            was_detected = false;
                            let _ = handle.emit("launcher-closed", ());

                            let handle_clone = handle.clone();
                            let _ = handle.run_on_main_thread(move || {
                                if let Some(overlay) = handle_clone.get_webview_window("overlay") {
                                    let _ = overlay.hide();
                                }
                            });
                        }
                    }
                });
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
