/// Parser for TOTP secrets, supporting standard Base32, `otpauth://` URLs,
/// and Google Authenticator export QR codes (`otpauth-migration://`).

use base64::Engine as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTotpAccount {
    pub secret_base32: String,
    pub name: Option<String>,
    pub issuer: Option<String>,
}

/// Parses any TOTP input (migration URL, standard `otpauth://` URL, or plain Base32).
pub fn parse_totp_input(input: &str) -> Result<Vec<ParsedTotpAccount>, String> {
    let trimmed = input.trim();
    if trimmed.starts_with("otpauth-migration://") {
        parse_migration_url(trimmed)
    } else if trimmed.starts_with("otpauth://") {
        parse_otpauth_url(trimmed).map(|acc| vec![acc])
    } else {
        parse_plain_base32(trimmed).map(|acc| vec![acc])
    }
}

fn parse_migration_url(url_str: &str) -> Result<Vec<ParsedTotpAccount>, String> {
    let query_start = url_str.find('?').ok_or("URLパラメータが見つかりません")?;
    let query = &url_str[query_start + 1..];
    let data_param = query
        .split('&')
        .find_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?;
            let val = parts.next()?;
            if key == "data" {
                Some(val)
            } else {
                None
            }
        })
        .ok_or("dataパラメータが見つかりません")?;

    let decoded_data = percent_encoding_decode(data_param);
    let raw_bytes = decode_base64_flexible(&decoded_data)
        .map_err(|e| format!("Base64デコードエラー: {e}"))?;

    let accounts = parse_protobuf_migration_payload(&raw_bytes)?;
    if accounts.is_empty() {
        return Err("エクスポートデータ内に有効なTOTPアカウントが見つかりませんでした".to_string());
    }
    Ok(accounts)
}

fn percent_encoding_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(val) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                result.push(val as char);
                i += 3;
                continue;
            }
        }
        result.push(bytes[i] as char);
        i += 1;
    }
    result
}

fn decode_base64_flexible(s: &str) -> Result<Vec<u8>, String> {
    let mut cleaned = s.replace('-', "+").replace('_', "/");
    while cleaned.len() % 4 != 0 {
        cleaned.push('=');
    }
    use base64::engine::general_purpose::STANDARD;
    STANDARD.decode(&cleaned).map_err(|e| e.to_string())
}

fn parse_protobuf_migration_payload(bytes: &[u8]) -> Result<Vec<ParsedTotpAccount>, String> {
    let mut offset = 0;
    let mut accounts = Vec::new();

    while offset < bytes.len() {
        let (field_num, wire_type, len) = read_tag(bytes, &mut offset)?;
        if field_num == 1 && wire_type == 2 {
            let end = offset + len;
            if end > bytes.len() {
                return Err("Protobufフォーマットエラー: 長さが超過しています".to_string());
            }
            if let Ok(acc) = parse_otp_parameters(&bytes[offset..end]) {
                accounts.push(acc);
            }
            offset = end;
        } else {
            skip_field(bytes, &mut offset, wire_type, len)?;
        }
    }

    Ok(accounts)
}

fn parse_otp_parameters(bytes: &[u8]) -> Result<ParsedTotpAccount, String> {
    let mut offset = 0;
    let mut secret_bytes = None;
    let mut name = None;
    let mut issuer = None;

    while offset < bytes.len() {
        let (field_num, wire_type, len) = read_tag(bytes, &mut offset)?;
        let end = offset + len;
        match (field_num, wire_type) {
            (1, 2) => {
                if end <= bytes.len() {
                    secret_bytes = Some(bytes[offset..end].to_vec());
                }
                offset = end;
            }
            (2, 2) => {
                if end <= bytes.len() {
                    name = String::from_utf8(bytes[offset..end].to_vec()).ok();
                }
                offset = end;
            }
            (3, 2) => {
                if end <= bytes.len() {
                    issuer = String::from_utf8(bytes[offset..end].to_vec()).ok();
                }
                offset = end;
            }
            _ => {
                skip_field(bytes, &mut offset, wire_type, len)?;
            }
        }
    }

    let raw_secret = secret_bytes.ok_or("シードデータが存在しません")?;
    let secret_base32 = base32::encode(base32::Alphabet::Rfc4648 { padding: false }, &raw_secret);

    Ok(ParsedTotpAccount {
        secret_base32,
        name,
        issuer,
    })
}

fn read_tag(bytes: &[u8], offset: &mut usize) -> Result<(u32, u8, usize), String> {
    let tag = read_varint(bytes, offset)?;
    let field_num = (tag >> 3) as u32;
    let wire_type = (tag & 0x07) as u8;
    let len = if wire_type == 2 {
        read_varint(bytes, offset)? as usize
    } else {
        0
    };
    Ok((field_num, wire_type, len))
}

fn read_varint(bytes: &[u8], offset: &mut usize) -> Result<u64, String> {
    let mut result: u64 = 0;
    let mut shift = 0;
    while *offset < bytes.len() {
        let byte = bytes[*offset];
        *offset += 1;
        result |= ((byte & 0x7f) as u64) << shift;
        if (byte & 0x80) == 0 {
            return Ok(result);
        }
        shift += 7;
        if shift >= 64 {
            return Err("Varintオーバーフロー".to_string());
        }
    }
    Err("Varintデコード中にデータの終端に達しました".to_string())
}

fn skip_field(bytes: &[u8], offset: &mut usize, wire_type: u8, len: usize) -> Result<(), String> {
    match wire_type {
        0 => {
            read_varint(bytes, offset)?;
        }
        1 => {
            *offset += 8;
        }
        2 => {
            *offset += len;
        }
        5 => {
            *offset += 4;
        }
        _ => return Err(format!("未サポートのProtobuf wire type: {wire_type}")),
    }
    if *offset > bytes.len() {
        return Err("Protobufフィールドスキップ失敗: 境界外参照".to_string());
    }
    Ok(())
}

fn parse_otpauth_url(url_str: &str) -> Result<ParsedTotpAccount, String> {
    let query_start = url_str.find('?').ok_or("otpauth URLにパラメータがありません")?;
    let query = &url_str[query_start + 1..];

    let mut secret = None;
    let mut issuer = None;

    for pair in query.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let val = parts.next().unwrap_or("");
        if key.eq_ignore_ascii_case("secret") {
            secret = Some(val.to_uppercase().replace(' ', "").replace('-', ""));
        } else if key.eq_ignore_ascii_case("issuer") {
            issuer = Some(percent_encoding_decode(val));
        }
    }

    let secret_base32 = secret.ok_or("secretパラメータが見つかりません")?;
    let path_part = &url_str[..query_start];
    let name = path_part
        .split('/')
        .last()
        .map(|s| percent_encoding_decode(s));

    Ok(ParsedTotpAccount {
        secret_base32,
        name,
        issuer,
    })
}

fn parse_plain_base32(input: &str) -> Result<ParsedTotpAccount, String> {
    let cleaned = input.to_uppercase().replace(' ', "").replace('-', "");
    if cleaned.is_empty() {
        return Err("TOTPシードが空です".to_string());
    }
    if base32::decode(base32::Alphabet::Rfc4648 { padding: false }, &cleaned).is_none()
        && base32::decode(base32::Alphabet::Rfc4648 { padding: true }, &cleaned).is_none()
    {
        return Err("不正なBase32文字列です".to_string());
    }
    Ok(ParsedTotpAccount {
        secret_base32: cleaned,
        name: None,
        issuer: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_base32_seed() {
        let input = "GEZD GNBV GY3T QOJQ GEZD GNBV GY3T QOJQ";
        let res = parse_totp_input(input).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].secret_base32, "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
    }

    #[test]
    fn parses_standard_otpauth_url() {
        let input = "otpauth://totp/SquareEnix:FF14?secret=JBSWY3DPEHPK3PXP&issuer=SquareEnix";
        let res = parse_totp_input(input).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].secret_base32, "JBSWY3DPEHPK3PXP");
        assert_eq!(res[0].issuer.as_deref(), Some("SquareEnix"));
    }

    #[test]
    fn parses_google_authenticator_migration_payload() {
        // Construct protobuf payload with secret = b"12345678901234567890" (Base32: GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ)
        // name = "FF14 Main"
        let mut otp_params = vec![
            0x0a, 20, b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'0',
            0x12, 9, b'F', b'F', b'1', b'4', b' ', b'M', b'a', b'i', b'n'
        ];
        let mut payload = vec![0x0a, otp_params.len() as u8];
        payload.append(&mut otp_params);

        use base64::engine::general_purpose::STANDARD;
        let b64 = STANDARD.encode(&payload);
        let migration_url = format!("otpauth-migration://offline?data={b64}");

        let res = parse_totp_input(&migration_url).unwrap();
        assert_eq!(res.len(), 1);
        assert_eq!(res[0].secret_base32, "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ");
        assert_eq!(res[0].name.as_deref(), Some("FF14 Main"));
    }

    #[test]
    fn returns_error_on_invalid_input() {
        assert!(parse_totp_input("!!!INVALID_BASE32!!!").is_err());
    }
}
