/// Win32 `SendInput` based credential injector.
///
/// Sends keystrokes to the currently focused window by simulating
/// hardware keyboard events. This is the same mechanism used by
/// KeePassXC Auto-Type and 1Password.
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_KEYBOARD, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE, VK_BACK, VK_CONTROL,
    VK_RETURN, VK_TAB, VIRTUAL_KEY,
};

/// Delay between each character input (milliseconds).
/// WebView2 / Chromium needs ≥ 15 ms to process each key event reliably.
const INTER_KEY_DELAY_MS: u64 = 20;

/// Send a string as simulated keystrokes to the foreground window.
pub fn send_string(text: &str, delay_ms: Option<u64>) -> Result<(), String> {
    let delay = delay_ms.unwrap_or(INTER_KEY_DELAY_MS);
    for ch in text.chars() {
        send_unicode_char(ch)?;
        std::thread::sleep(std::time::Duration::from_millis(delay));
    }
    Ok(())
}

/// Send a single Tab key press.
pub fn send_tab(delay_ms: Option<u64>) -> Result<(), String> {
    let delay = delay_ms.unwrap_or(INTER_KEY_DELAY_MS);
    send_virtual_key(VK_TAB)?;
    std::thread::sleep(std::time::Duration::from_millis(delay));
    Ok(())
}

/// Send Ctrl+A to select all text in the current field.
pub fn send_select_all(delay_ms: Option<u64>) -> Result<(), String> {
    let delay = delay_ms.unwrap_or(INTER_KEY_DELAY_MS);
    let inputs = [
        make_vk_input(VK_CONTROL, false),
        make_vk_input(VIRTUAL_KEY(b'A' as u16), false),
        make_vk_input(VIRTUAL_KEY(b'A' as u16), true),
        make_vk_input(VK_CONTROL, true),
    ];
    send_inputs(&inputs)?;
    std::thread::sleep(std::time::Duration::from_millis(delay));
    Ok(())
}

/// Send a Backspace key press.
pub fn send_backspace(delay_ms: Option<u64>) -> Result<(), String> {
    let delay = delay_ms.unwrap_or(INTER_KEY_DELAY_MS);
    send_virtual_key(VK_BACK)?;
    std::thread::sleep(std::time::Duration::from_millis(delay));
    Ok(())
}

/// Clears any existing text in the current input box (Ctrl+A -> Backspace) and types new text.
pub fn send_clear_and_type(text: &str, delay_ms: Option<u64>) -> Result<(), String> {
    let _ = send_select_all(delay_ms);
    let _ = send_backspace(delay_ms);
    send_string(text, delay_ms)
}

/// Send a single Enter key press.
pub fn send_enter(delay_ms: Option<u64>) -> Result<(), String> {
    let delay = delay_ms.unwrap_or(INTER_KEY_DELAY_MS);
    send_virtual_key(VK_RETURN)?;
    std::thread::sleep(std::time::Duration::from_millis(delay));
    Ok(())
}

fn send_unicode_char(ch: char) -> Result<(), String> {
    let scalar = ch as u16;
    let inputs = [
        make_unicode_input(scalar, false),
        make_unicode_input(scalar, true),
    ];
    send_inputs(&inputs)
}

fn send_virtual_key(vk: VIRTUAL_KEY) -> Result<(), String> {
    let inputs = [
        make_vk_input(vk, false),
        make_vk_input(vk, true),
    ];
    send_inputs(&inputs)
}

fn send_inputs(inputs: &[INPUT]) -> Result<(), String> {
    let sent = unsafe {
        SendInput(inputs, std::mem::size_of::<INPUT>() as i32)
    };
    if sent == inputs.len() as u32 {
        Ok(())
    } else {
        Err(format!(
            "SendInput: sent {sent}/{} events",
            inputs.len()
        ))
    }
}

fn make_unicode_input(scalar: u16, key_up: bool) -> INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::KEYBDINPUT;
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0),
                wScan: scalar,
                dwFlags: if key_up {
                    KEYEVENTF_UNICODE | KEYEVENTF_KEYUP
                } else {
                    KEYEVENTF_UNICODE
                },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

fn make_vk_input(vk: VIRTUAL_KEY, key_up: bool) -> INPUT {
    use windows::Win32::UI::Input::KeyboardAndMouse::KEYBDINPUT;
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: windows::Win32::UI::Input::KeyboardAndMouse::INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { Default::default() },
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

#[cfg(test)]
mod tests {
    // SendInput sends keystrokes to the foreground window, so unit tests
    // that call the real API would require a focused text field to observe output.
    // We verify the builder functions compile and produce valid INPUT structs.

    use super::*;

    // --- テストリスト ---
    // [x] make_unicode_input でキーダウンの dwFlags が KEYEVENTF_UNICODE を含む
    // [x] make_unicode_input でキーアップの dwFlags が KEYEVENTF_UNICODE | KEYEVENTF_KEYUP を含む
    // [x] make_vk_input でキーアップの dwFlags が KEYEVENTF_KEYUP を含む
    // [x] make_vk_input でキーダウンの dwFlags が 0

    #[test]
    fn unicode_keydown_has_correct_flags() {
        let input = make_unicode_input(b'A' as u16, false);
        let flags = unsafe { input.Anonymous.ki.dwFlags };
        assert_eq!(flags, KEYEVENTF_UNICODE);
    }

    #[test]
    fn unicode_keyup_has_correct_flags() {
        let input = make_unicode_input(b'A' as u16, true);
        let flags = unsafe { input.Anonymous.ki.dwFlags };
        assert_eq!(flags, KEYEVENTF_UNICODE | KEYEVENTF_KEYUP);
    }

    #[test]
    fn vk_keydown_has_zero_flags() {
        let input = make_vk_input(VK_TAB, false);
        let flags = unsafe { input.Anonymous.ki.dwFlags };
        assert_eq!(flags.0, 0);
    }

    #[test]
    fn vk_keyup_has_keyup_flag() {
        let input = make_vk_input(VK_RETURN, true);
        let flags = unsafe { input.Anonymous.ki.dwFlags };
        assert_eq!(flags, KEYEVENTF_KEYUP);
    }
}
