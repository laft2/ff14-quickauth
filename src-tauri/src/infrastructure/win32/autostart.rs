use std::path::PathBuf;

fn get_startup_folder() -> Option<PathBuf> {
    let app_data = std::env::var("APPDATA").ok()?;
    Some(PathBuf::from(app_data).join(r"Microsoft\Windows\Start Menu\Programs\Startup"))
}

fn get_shortcut_path() -> Option<PathBuf> {
    get_startup_folder().map(|folder| folder.join("FF14 QuickAuth.lnk"))
}

pub fn is_autostart_enabled() -> bool {
    get_shortcut_path().map(|p| p.exists()).unwrap_or(false)
}

pub fn set_autostart_enabled(enabled: bool) -> Result<(), String> {
    let shortcut_path = get_shortcut_path()
        .ok_or_else(|| "APPDATA フォルダの取得に失敗しました".to_string())?;

    // Cleanup legacy shortcut name if present
    if let Some(folder) = get_startup_folder() {
        let old_path = folder.join("FF14 Companion.lnk");
        if old_path.exists() {
            let _ = std::fs::remove_file(&old_path);
        }
    }

    if enabled {
        let exe_path = std::env::current_exe()
            .map_err(|e| format!("実行ファイルのパス取得に失敗しました: {e}"))?;
        let exe_dir = exe_path
            .parent()
            .ok_or_else(|| "実行ファイルディレクトリの取得に失敗しました".to_string())?;

        let script = format!(
            "$wsh = New-Object -ComObject WScript.Shell; $s = $wsh.CreateShortcut('{0}'); $s.TargetPath = '{1}'; $s.WorkingDirectory = '{2}'; $s.Description = 'FF14 QuickAuth 認証補完マネージャー'; $s.Save()",
            shortcut_path.to_string_lossy().replace("'", "''"),
            exe_path.to_string_lossy().replace("'", "''"),
            exe_dir.to_string_lossy().replace("'", "''")
        );

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            let output = std::process::Command::new("powershell")
                .args(&["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| format!("PowerShellの実行に失敗しました: {e}"))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                return Err(format!("スタートアップショートカットの作成に失敗しました: {stderr}"));
            }
        }
    } else {
        if shortcut_path.exists() {
            std::fs::remove_file(&shortcut_path)
                .map_err(|e| format!("スタートアップショートカットの削除に失敗しました: {e}"))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_shortcut_path_returns_some() {
        let path = get_shortcut_path();
        assert!(path.is_some());
        assert!(path.unwrap().to_string_lossy().contains("Startup"));
    }
}
