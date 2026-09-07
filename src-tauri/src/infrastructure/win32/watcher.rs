/// Win32 `SetWinEventHook` based launcher process watcher.
///
/// Monitors `EVENT_SYSTEM_FOREGROUND` events to detect when the FF14
/// launcher (`ffxivlauncher64.exe`) becomes the foreground window.
/// Uses `WINEVENT_OUTOFCONTEXT` so no DLL injection is needed.
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_NAME_WIN32,
    PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

/// Process names that identify the FF14 launcher.
pub const LAUNCHER_PROCESS_NAMES: &[&str] = &[
    "ffxivlauncher64.exe",
    "ffxivlauncher.exe",
    "ffxivboot64.exe",
    "ffxivboot.exe",
];

/// Returns `true` if the given executable name matches the FF14 launcher.
pub fn is_ff14_launcher(exe_name: &str) -> bool {
    let lower = exe_name.to_lowercase();
    LAUNCHER_PROCESS_NAMES
        .iter()
        .any(|&name| lower.ends_with(name))
}

/// Retrieves the full executable path for the process owning `hwnd`.
///
/// Returns `None` if the process cannot be opened (e.g., elevated process).
pub fn get_process_exe_path(hwnd: HWND) -> Option<String> {
    let mut pid: u32 = 0;
    unsafe { GetWindowThreadProcessId(hwnd, Some(&mut pid)) };
    if pid == 0 {
        return None;
    }

    let handle = unsafe {
        OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?
    };

    let mut buf = vec![0u16; 1024];
    let mut len = buf.len() as u32;

    let ok = unsafe {
        QueryFullProcessImageNameW(handle, PROCESS_NAME_WIN32, windows::core::PWSTR(buf.as_mut_ptr()), &mut len)
    };

    if ok.is_ok() {
        Some(String::from_utf16_lossy(&buf[..len as usize]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- テストリスト ---
    // [x] is_ff14_launcher: "ffxivlauncher64.exe" → true
    // [x] is_ff14_launcher: "ffxivlauncher.exe" → true
    // [x] is_ff14_launcher: "ffxivboot64.exe" → true
    // [x] is_ff14_launcher: "ffxivboot.exe" → true
    // [x] is_ff14_launcher: フルパス "C:\...\ffxivlauncher64.exe" → true
    // [x] is_ff14_launcher: 大文字混じり "FFXIVLAUNCHER64.EXE" → true
    // [x] is_ff14_launcher: "notepad.exe" → false
    // [x] is_ff14_launcher: "" → false

    #[test]
    fn detects_ffxivlauncher64() {
        assert!(is_ff14_launcher("ffxivlauncher64.exe"));
    }

    #[test]
    fn detects_ffxivlauncher() {
        assert!(is_ff14_launcher("ffxivlauncher.exe"));
    }

    #[test]
    fn detects_ffxivboot64() {
        assert!(is_ff14_launcher("ffxivboot64.exe"));
    }

    #[test]
    fn detects_ffxivboot() {
        assert!(is_ff14_launcher("ffxivboot.exe"));
    }

    #[test]
    fn detects_full_path() {
        assert!(is_ff14_launcher(
            r"C:\Program Files (x86)\SquareEnix\FINAL FANTASY XIV - A Realm Reborn\boot\ffxivlauncher64.exe"
        ));
    }

    #[test]
    fn detects_uppercase() {
        assert!(is_ff14_launcher("FFXIVLAUNCHER64.EXE"));
    }

    #[test]
    fn rejects_notepad() {
        assert!(!is_ff14_launcher("notepad.exe"));
    }

    #[test]
    fn rejects_empty_string() {
        assert!(!is_ff14_launcher(""));
    }
}
