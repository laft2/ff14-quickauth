use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Threading::{
    AttachThreadInput, GetCurrentThreadId, OpenProcess, QueryFullProcessImageNameW,
    PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, EnumWindows, GetWindowRect, GetWindowThreadProcessId, IsIconic,
    IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow, HWND_TOPMOST,
    SWP_NOACTIVATE, SWP_NOSIZE, SW_RESTORE,
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

/// Moves an overlay window as topmost without stealing focus or altering visibility flags.
pub fn move_window_no_activate(raw_hwnd: isize, x: i32, y: i32) {
    let hwnd = HWND(raw_hwnd as *mut core::ffi::c_void);
    unsafe {
        let _ = SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            x,
            y,
            0,
            0,
            SWP_NOSIZE | SWP_NOACTIVATE,
        );
    }
}

/// Number of consecutive non-detected polls before hiding the overlay.
/// At 300ms per poll, 3 ticks ≈ 0.9 seconds of debounce against brief focus glitches.
pub const HIDE_DEBOUNCE_TICKS: u32 = 3;

/// Overlay display decision computed by [`OverlayTracker`] on each poll tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayAction {
    /// No window state change needed.
    None,
    /// Show the overlay (creating it if needed) at `(x, y)` without stealing focus.
    Show { x: i32, y: i32, emit_detected: bool },
    /// Move the already-shown overlay to a new `(x, y)` without re-showing or stealing focus.
    Move { x: i32, y: i32 },
    /// Hide the overlay and emit `launcher-closed`.
    Hide,
}

/// Pure state machine that decides when to show, move, or hide the overlay window.
///
/// Key guarantees:
/// - Never hides the overlay while the overlay itself has focus (prevents self-hide on click).
/// - Never repeatedly calls `show()` once already shown (prevents focus-steal flicker loop).
/// - Debounces brief focus interruptions before hiding.
#[derive(Debug, Default)]
pub struct OverlayTracker {
    was_detected: bool,
    overlay_shown: bool,
    last_pos: Option<(i32, i32)>,
    miss_count: u32,
}

impl OverlayTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Computes the next overlay action given the current foreground state and launcher rect.
    ///
    /// - `launcher_foreground`: foreground window belongs to an FF14 launcher process.
    /// - `overlay_foreground`: foreground window is our own overlay window.
    /// - `launcher_rect`: `(left, top, right, bottom)` of the main visible launcher window, if any.
    pub fn update(
        &mut self,
        launcher_foreground: bool,
        overlay_foreground: bool,
        launcher_rect: Option<(i32, i32, i32, i32)>,
    ) -> OverlayAction {
        let active = launcher_foreground || (overlay_foreground && launcher_rect.is_some());

        if active {
            self.miss_count = 0;

            let emit_detected = !self.was_detected;
            self.was_detected = true;

            if let Some((left, top, right, _bottom)) = launcher_rect {
                let l_width = right - left;
                let pos = (left + (l_width - 260) / 2, top + 35);

                if !self.overlay_shown {
                    self.overlay_shown = true;
                    self.last_pos = Some(pos);
                    return OverlayAction::Show {
                        x: pos.0,
                        y: pos.1,
                        emit_detected,
                    };
                }

                if self.last_pos != Some(pos) {
                    self.last_pos = Some(pos);
                    return OverlayAction::Move { x: pos.0, y: pos.1 };
                }
            }

            return OverlayAction::None;
        }

        if self.was_detected {
            self.miss_count += 1;
            if self.miss_count >= HIDE_DEBOUNCE_TICKS {
                self.was_detected = false;
                self.overlay_shown = false;
                self.last_pos = None;
                self.miss_count = 0;
                return OverlayAction::Hide;
            }
        }

        OverlayAction::None
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

    #[test]
    fn overlay_tracker_shows_once_and_does_not_repeat_show_at_same_position() {
        let mut tracker = OverlayTracker::new();
        let rect = Some((100, 100, 1100, 700)); // width=1000 -> x=100+(1000-260)/2=470, y=135

        assert_eq!(
            tracker.update(true, false, rect),
            OverlayAction::Show {
                x: 470,
                y: 135,
                emit_detected: true
            }
        );
        // Subsequent ticks at same position must NOT re-trigger Show
        assert_eq!(tracker.update(true, false, rect), OverlayAction::None);
        assert_eq!(tracker.update(true, false, rect), OverlayAction::None);
    }

    #[test]
    fn overlay_tracker_moves_only_when_launcher_position_changes() {
        let mut tracker = OverlayTracker::new();
        let rect1 = Some((100, 100, 1100, 700));
        let rect2 = Some((200, 150, 1200, 750));

        assert!(matches!(
            tracker.update(true, false, rect1),
            OverlayAction::Show { .. }
        ));
        assert_eq!(
            tracker.update(true, false, rect2),
            OverlayAction::Move { x: 570, y: 185 }
        );
        assert_eq!(tracker.update(true, false, rect2), OverlayAction::None);
    }

    #[test]
    fn overlay_tracker_does_not_hide_when_overlay_itself_is_foreground() {
        let mut tracker = OverlayTracker::new();
        let rect = Some((100, 100, 1100, 700));

        assert!(matches!(
            tracker.update(true, false, rect),
            OverlayAction::Show { .. }
        ));
        // Clicking the overlay makes overlay foreground instead of launcher
        for _ in 0..10 {
            assert_eq!(tracker.update(false, true, rect), OverlayAction::None);
        }
    }

    #[test]
    fn overlay_tracker_debounces_brief_focus_loss_and_hides_after_threshold() {
        let mut tracker = OverlayTracker::new();
        let rect = Some((100, 100, 1100, 700));

        assert!(matches!(
            tracker.update(true, false, rect),
            OverlayAction::Show { .. }
        ));

        // Brief focus loss (< HIDE_DEBOUNCE_TICKS) should not hide
        for _ in 0..(HIDE_DEBOUNCE_TICKS - 1) {
            assert_eq!(tracker.update(false, false, rect), OverlayAction::None);
        }
        // Returning focus resets miss_count
        assert_eq!(tracker.update(true, false, rect), OverlayAction::None);

        // Sustained focus loss hides on the Nth tick
        for _ in 0..(HIDE_DEBOUNCE_TICKS - 1) {
            assert_eq!(tracker.update(false, false, rect), OverlayAction::None);
        }
        assert_eq!(tracker.update(false, false, rect), OverlayAction::Hide);

        // Re-focusing launcher later shows overlay again
        assert_eq!(
            tracker.update(true, false, rect),
            OverlayAction::Show {
                x: 470,
                y: 135,
                emit_detected: true
            }
        );
    }
}
