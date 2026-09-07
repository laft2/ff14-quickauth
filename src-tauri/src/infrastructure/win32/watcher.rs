use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, SetForegroundWindow, ShowWindow, SW_RESTORE,
};

/// Process names that identify the FF14 launcher.
pub const LAUNCHER_PROCESS_NAMES: &[&str] = &[
    "ffxivlauncher64.exe",
    "ffxivlauncher.exe",
    "ffxivboot64.exe",
    "ffxivboot.exe",
    "xivlauncher.exe",
];

/// Returns `true` if the given executable name matches the FF14 launcher.
pub fn is_ff14_launcher(exe_name: &str) -> bool {
    let lower = exe_name.to_lowercase();
    LAUNCHER_PROCESS_NAMES
        .iter()
        .any(|&name| lower.ends_with(name))
}

/// Finds the main visible FF14 Launcher window (ignoring small splash dialogs) and returns its HWND and RECT.
pub fn find_main_launcher_window() -> Option<(HWND, RECT)> {
    struct Context {
        best_hwnd: Option<HWND>,
        best_rect: RECT,
        max_area: i32,
    }
    let mut ctx = Context {
        best_hwnd: None,
        best_rect: RECT::default(),
        max_area: 0,
    };

    unsafe extern "system" fn enum_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let ctx = &mut *(lparam.0 as *mut Context);
        if IsWindowVisible(hwnd).as_bool() {
            if let Some(exe_path) = get_process_exe_path(hwnd) {
                if is_ff14_launcher(&exe_path) {
                    let mut rect = RECT::default();
                    if GetWindowRect(hwnd, &mut rect).is_ok() {
                        let width = rect.right - rect.left;
                        let height = rect.bottom - rect.top;
                        let area = width * height;
                        // Ignore small splash/dialog windows (width < 350 or height < 200)
                        if width >= 350 && height >= 200 && area > ctx.max_area {
                            ctx.max_area = area;
                            ctx.best_hwnd = Some(hwnd);
                            ctx.best_rect = rect;
                        }
                    }
                }
            }
        }
        BOOL(1)
    }

    unsafe {
        let _ = EnumWindows(Some(enum_proc), LPARAM(&mut ctx as *mut Context as isize));
    }

    if let Some(hwnd) = ctx.best_hwnd {
        Some((hwnd, ctx.best_rect))
    } else {
        None
    }
}

/// Finds the HWND of the active FF14 Launcher window.
pub fn find_launcher_hwnd() -> Option<HWND> {
    find_main_launcher_window().map(|(hwnd, _)| hwnd)
}

/// Restores and brings the launcher window to foreground.
pub fn focus_launcher_window(hwnd: HWND) -> Result<(), String> {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let mut pid: u32 = 0;
        let target_thread = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        let current_thread = GetCurrentThreadId();

        if target_thread != 0 && current_thread != target_thread {
            let _ = AttachThreadInput(current_thread, target_thread, true);
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
            let _ = AttachThreadInput(current_thread, target_thread, false);
        } else {
            let _ = BringWindowToTop(hwnd);
            let _ = SetForegroundWindow(hwnd);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    Ok(())
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
