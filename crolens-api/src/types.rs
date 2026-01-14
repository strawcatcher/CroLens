use std::str::FromStr;

use alloy_primitives::{Address, U256};
use uuid::Uuid;
use worker::Request;

use crate::error::{CroLensError, Result};

pub fn now_ms() -> i64 {
    worker::Date::now().as_millis() as i64
}

pub fn now_seconds() -> i64 {
    now_ms() / 1000
}

pub fn get_trace_id(req: &Request) -> String {
    req.headers()
        .get("x-request-id")
        .ok()
        .flatten()
        .filter(|v| !v.trim().is_empty())
        .unwrap_or_else(|| Uuid::new_v4().to_string())
}

pub fn get_header(req: &Request, name: &str) -> Option<String> {
    req.headers().get(name).ok().flatten()
}

pub fn normalize_symbol(symbol: &str) -> String {
    symbol.trim().to_lowercase()
}

pub fn parse_address(address: &str) -> Result<Address> {
    let trimmed = address.trim();
    Address::from_str(trimmed).map_err(|_| CroLensError::InvalidAddress(trimmed.to_string()))
}

pub fn parse_u256_dec(value: &str) -> Result<U256> {
    let trimmed = value.trim();
    U256::from_str_radix(trimmed, 10)
        .map_err(|_| CroLensError::invalid_params(format!("Invalid U256: {trimmed}")))
}

pub fn parse_u256_hex(value: &str) -> Result<U256> {
    let trimmed = value.trim().trim_start_matches("0x");
    if trimmed.is_empty() {
        return Ok(U256::ZERO);
    }
    U256::from_str_radix(trimmed, 16)
        .map_err(|_| CroLensError::invalid_params(format!("Invalid hex U256: {value}")))
}

pub fn validate_hex_string(value: &str, expected_len: usize) -> Result<()> {
    let trimmed = value.trim();
    if !trimmed.starts_with("0x") {
        return Err(CroLensError::invalid_params(
            "hex string must be 0x-prefixed".to_string(),
        ));
    }

    let hex = trimmed.trim_start_matches("0x");
    if hex.len() != expected_len {
        return Err(CroLensError::invalid_params(format!(
            "hex string must be {expected_len} hex chars"
        )));
    }

    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(CroLensError::invalid_params(
            "hex string contains non-hex characters".to_string(),
        ));
    }

    Ok(())
}

pub fn get_client_ip(req: &Request) -> String {
    if let Some(ip) = get_header(req, "CF-Connecting-IP") {
        let trimmed = ip.trim();
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }

    if let Some(ip) = get_header(req, "x-forwarded-for") {
        let first = ip.split(',').next().map(|v| v.trim()).unwrap_or_default();
        if !first.is_empty() {
            return first.to_string();
        }
    }

    "unknown".to_string()
}

pub fn bytes_to_hex0x<B: AsRef<[u8]>>(bytes: B) -> String {
    format!("0x{}", hex::encode(bytes.as_ref()))
}

pub fn hex0x_to_bytes(value: &str) -> Result<Vec<u8>> {
    let trimmed = value.trim().trim_start_matches("0x");
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    hex::decode(trimmed).map_err(|err| CroLensError::invalid_params(format!("Invalid hex: {err}")))
}

pub fn format_units(value: &U256, decimals: u8) -> String {
    if decimals == 0 {
        return value.to_string();
    }

    let raw = value.to_string();
    let decimals_usize = decimals as usize;
    if raw == "0" {
        return "0".to_string();
    }

    if raw.len() <= decimals_usize {
        let mut padded = String::with_capacity(decimals_usize + 2);
        padded.push_str("0.");
        for _ in 0..(decimals_usize - raw.len()) {
            padded.push('0');
        }
        padded.push_str(&raw);
        return trim_trailing_zeros(&padded);
    }

    let split = raw.len() - decimals_usize;
    let (int_part, frac_part) = raw.split_at(split);
    let formatted = format!("{int_part}.{frac_part}");
    trim_trailing_zeros(&formatted)
}

fn trim_trailing_zeros(value: &str) -> String {
    if let Some((int_part, frac_part)) = value.split_once('.') {
        let trimmed_frac = frac_part.trim_end_matches('0');
        if trimmed_frac.is_empty() {
            return int_part.to_string();
        }
        return format!("{int_part}.{trimmed_frac}");
    }
    value.to_string()
}

#[allow(dead_code)]
pub mod u256_as_string {
    use alloy_primitives::U256;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &U256, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<U256, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        U256::from_str_radix(&s, 10).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_units_with_decimals() {
        let value = U256::from(1234500u64);
        assert_eq!(format_units(&value, 6), "1.2345");
    }

    #[test]
    fn formats_units_small_values() {
        let value = U256::from(1u64);
        assert_eq!(format_units(&value, 18), "0.000000000000000001");
    }

    #[test]
    fn validates_hex_string_accepts_valid() {
        assert!(validate_hex_string("0x00ff", 4).is_ok());
        assert!(validate_hex_string("0xA0b1", 4).is_ok());
    }

    #[test]
    fn validates_hex_string_rejects_missing_prefix() {
        let err = validate_hex_string("00ff", 4).unwrap_err();
        assert!(err.to_string().contains("0x"));
    }

    #[test]
    fn validates_hex_string_rejects_wrong_length() {
        let err = validate_hex_string("0x00ff", 6).unwrap_err();
        assert!(err.to_string().contains("6"));
    }

    #[test]
    fn validates_hex_string_rejects_invalid_chars() {
        let err = validate_hex_string("0x00gg", 4).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("non-hex"));
    }

    #[test]
    fn parses_valid_address() {
        let addr = parse_address("0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23").unwrap();
        assert_ne!(addr, Address::ZERO);
    }

    #[test]
    fn rejects_invalid_address() {
        let err = parse_address("0x1234").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid address"));
    }

    #[test]
    fn parses_u256_decimal() {
        let v = parse_u256_dec("42").unwrap();
        assert_eq!(v, U256::from(42u64));
    }

    #[test]
    fn rejects_invalid_u256_decimal() {
        let err = parse_u256_dec("not-a-number").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid u256"));
    }

    #[test]
    fn parses_u256_hex() {
        let v = parse_u256_hex("0x2a").unwrap();
        assert_eq!(v, U256::from(42u64));
    }

    #[test]
    fn rejects_invalid_u256_hex() {
        let err = parse_u256_hex("0xzz").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid hex"));
    }

    #[test]
    fn hex_roundtrip() {
        let bytes = vec![0xde, 0xad, 0xbe, 0xef];
        let encoded = bytes_to_hex0x(&bytes);
        assert_eq!(encoded, "0xdeadbeef");
        let decoded = hex0x_to_bytes(&encoded).unwrap();
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn hex_decoder_rejects_invalid() {
        let err = hex0x_to_bytes("0x00zz").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid hex"));
    }
}
