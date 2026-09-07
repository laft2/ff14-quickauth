use crate::domain::error::DomainError;

/// A generated TOTP code snapshot.
///
/// Invariants:
/// - `code` is always exactly 6 decimal digits (zero-padded)
/// - `remaining_seconds` is in range 0..=29
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TotpCode {
    code: String,
    remaining_seconds: u8,
}

impl TotpCode {
    /// Creates a new `TotpCode`.
    ///
    /// # Errors
    /// - [`DomainError::InvalidTotpCodeLength`] if `code` is not exactly 6 digits
    /// - [`DomainError::InvalidRemainingSeconds`] if `remaining_seconds` > 29
    pub fn new(code: impl Into<String>, remaining_seconds: u8) -> Result<Self, DomainError> {
        let code = code.into();
        if code.len() != 6 || !code.chars().all(|c| c.is_ascii_digit()) {
            return Err(DomainError::InvalidTotpCodeLength(code.len()));
        }
        if remaining_seconds > 29 {
            return Err(DomainError::InvalidRemainingSeconds(remaining_seconds));
        }
        Ok(Self {
            code,
            remaining_seconds,
        })
    }

    /// Returns the 6-digit zero-padded TOTP code string.
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns seconds remaining until the next code (0..=29).
    pub fn remaining_seconds(&self) -> u8 {
        self.remaining_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- テストリスト ---
    // [x] TotpCode は 6桁の数字文字列を持つ
    // [x] code() は常に 6桁ゼロパディングされた文字列を返す（"042918"等）
    // [x] remaining_seconds は 0 を受け付ける
    // [x] remaining_seconds は 29 を受け付ける
    // [x] remaining_seconds が 30 以上のときエラーになる
    // [x] code が 5桁のときエラーになる
    // [x] code が 7桁のときエラーになる
    // [x] code に数字以外が含まれるときエラーになる

    #[test]
    fn new_with_valid_code_and_seconds_creates_totp_code() {
        let totp = TotpCode::new("123456", 15);
        assert!(totp.is_ok());
        let totp = totp.unwrap();
        assert_eq!(totp.code(), "123456");
        assert_eq!(totp.remaining_seconds(), 15);
    }

    #[test]
    fn code_preserves_zero_padding() {
        // "042918" must stay as-is (not parsed as integer)
        let totp = TotpCode::new("042918", 0).unwrap();
        assert_eq!(totp.code(), "042918");
    }

    #[test]
    fn remaining_seconds_zero_is_valid() {
        let totp = TotpCode::new("000000", 0);
        assert!(totp.is_ok());
    }

    #[test]
    fn remaining_seconds_29_is_valid() {
        let totp = TotpCode::new("999999", 29);
        assert!(totp.is_ok());
    }

    #[test]
    fn remaining_seconds_30_returns_error() {
        let result = TotpCode::new("123456", 30);
        assert_eq!(
            result.unwrap_err(),
            DomainError::InvalidRemainingSeconds(30)
        );
    }

    #[test]
    fn code_with_five_digits_returns_error() {
        let result = TotpCode::new("12345", 10);
        assert_eq!(
            result.unwrap_err(),
            DomainError::InvalidTotpCodeLength(5)
        );
    }

    #[test]
    fn code_with_seven_digits_returns_error() {
        let result = TotpCode::new("1234567", 10);
        assert_eq!(
            result.unwrap_err(),
            DomainError::InvalidTotpCodeLength(7)
        );
    }

    #[test]
    fn code_with_non_digit_chars_returns_error() {
        let result = TotpCode::new("12345a", 10);
        // length is 6 but not all digits — we report length error with len=6
        // (alternative design: dedicated NonDigitCode error, revisit in refactor)
        assert!(result.is_err());
    }
}
