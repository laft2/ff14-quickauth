use crate::application::ports::{PortError, TotpProvider};
use crate::domain::totp::TotpCode;
use std::sync::Arc;

/// Use-case: generate TOTP codes for the FF14 one-time password field.
pub struct TotpService {
    provider: Arc<dyn TotpProvider>,
}

impl TotpService {
    pub fn new(provider: Arc<dyn TotpProvider>) -> Self {
        Self { provider }
    }

    /// Generate a current TOTP code from a Base32-encoded seed.
    pub fn generate(&self, seed: &str) -> Result<TotpCode, PortError> {
        self.provider.generate(seed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::ports::MockTotpProvider;

    // --- テストリスト ---
    // [x] provider が正常コードを返すとき generate() は Ok(TotpCode) を返す
    // [x] provider がエラーを返すとき generate() はそのエラーを伝播する
    // [x] 生成されたコードの remaining_seconds が 0〜29 の範囲に収まる

    #[test]
    fn generate_returns_totp_code_from_provider() {
        let mut mock = MockTotpProvider::new();
        mock.expect_generate()
            .withf(|seed| seed == "JBSWY3DPEHPK3PXP")
            .times(1)
            .returning(|_| Ok(TotpCode::new("123456", 15).unwrap()));

        let service = TotpService::new(Arc::new(mock));
        let result = service.generate("JBSWY3DPEHPK3PXP");
        assert!(result.is_ok());
        let code = result.unwrap();
        assert_eq!(code.code(), "123456");
        assert_eq!(code.remaining_seconds(), 15);
    }

    #[test]
    fn generate_propagates_provider_error() {
        let mut mock = MockTotpProvider::new();
        mock.expect_generate()
            .times(1)
            .returning(|_| Err(PortError::Totp("invalid seed".to_string())));

        let service = TotpService::new(Arc::new(mock));
        let result = service.generate("INVALID!!!");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TOTP generation error"));
    }

    #[test]
    fn generated_code_remaining_seconds_is_in_valid_range() {
        let mut mock = MockTotpProvider::new();
        mock.expect_generate()
            .returning(|_| Ok(TotpCode::new("000000", 29).unwrap()));

        let service = TotpService::new(Arc::new(mock));
        let code = service.generate("ANYSEED").unwrap();
        assert!(code.remaining_seconds() <= 29);
    }
}
