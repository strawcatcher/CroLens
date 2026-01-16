use alloy_primitives::U256;
use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct GetBlockInfoArgs {
    block: Option<String>,
    #[serde(default)]
    simple_mode: bool,
}

/// Get block information
pub async fn get_block_info(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetBlockInfoArgs = serde_json::from_value(args).unwrap_or(GetBlockInfoArgs {
        block: None,
        simple_mode: false,
    });

    let rpc = services.rpc()?;

    // Parse block identifier: "latest", block number, or block hash
    let block_param = input.block.as_deref().unwrap_or("latest");
    let block_id = parse_block_id(block_param);

    // Fetch block
    let block = rpc.eth_get_block_by_number(&block_id, false).await?;

    // Extract fields
    let number = block
        .get("number")
        .and_then(|v| v.as_str())
        .and_then(|s| parse_hex_u64(s))
        .unwrap_or(0);

    let hash = block
        .get("hash")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0")
        .to_string();

    let timestamp = block
        .get("timestamp")
        .and_then(|v| v.as_str())
        .and_then(|s| parse_hex_u64(s))
        .unwrap_or(0);

    let transactions_count = block
        .get("transactions")
        .and_then(|v| v.as_array())
        .map(|a| a.len())
        .unwrap_or(0);

    let gas_used = block
        .get("gasUsed")
        .and_then(|v| v.as_str())
        .and_then(|s| parse_hex_u64(s))
        .unwrap_or(0);

    let gas_limit = block
        .get("gasLimit")
        .and_then(|v| v.as_str())
        .and_then(|s| parse_hex_u64(s))
        .unwrap_or(1); // Avoid division by zero

    let gas_used_percent = if gas_limit > 0 {
        (gas_used as f64 / gas_limit as f64) * 100.0
    } else {
        0.0
    };

    let base_fee = block
        .get("baseFeePerGas")
        .and_then(|v| v.as_str())
        .and_then(|s| types::parse_u256_hex(s).ok())
        .unwrap_or(U256::ZERO);
    let base_fee_gwei = types::format_units(&base_fee, 9);

    let miner = block
        .get("miner")
        .and_then(|v| v.as_str())
        .unwrap_or("0x0")
        .to_string();

    // Calculate relative time
    let now_secs = types::now_ms() / 1000;
    let relative_time = format_relative_time(now_secs as u64, timestamp);

    if input.simple_mode {
        let text = format!(
            "Block #{} | {} | {} txs | Gas: {:.1}% used | Base fee: {} gwei",
            number, relative_time, transactions_count, gas_used_percent, base_fee_gwei
        );
        return Ok(serde_json::json!({
            "text": text,
            "meta": services.meta()
        }));
    }

    Ok(serde_json::json!({
        "number": number,
        "hash": hash,
        "timestamp": timestamp,
        "timestamp_relative": relative_time,
        "transactions_count": transactions_count,
        "gas_used": gas_used.to_string(),
        "gas_limit": gas_limit.to_string(),
        "gas_used_percent": format!("{:.2}", gas_used_percent),
        "base_fee_gwei": base_fee_gwei,
        "miner": miner,
        "meta": services.meta()
    }))
}

/// Parse block identifier
fn parse_block_id(s: &str) -> String {
    let trimmed = s.trim().to_lowercase();
    if trimmed == "latest" || trimmed == "pending" || trimmed == "earliest" {
        return trimmed;
    }
    // If it starts with 0x, assume it's already hex (could be block number or hash)
    if trimmed.starts_with("0x") {
        return trimmed;
    }
    // Otherwise, try to parse as decimal number
    if let Ok(num) = trimmed.parse::<u64>() {
        return format!("0x{:x}", num);
    }
    // Default to latest
    "latest".to_string()
}

/// Parse hex string to u64
fn parse_hex_u64(s: &str) -> Option<u64> {
    let trimmed = s.trim_start_matches("0x");
    u64::from_str_radix(trimmed, 16).ok()
}

/// Format relative time
fn format_relative_time(now_secs: u64, timestamp: u64) -> String {
    if timestamp > now_secs {
        return "just now".to_string();
    }

    let diff = now_secs - timestamp;

    if diff < 60 {
        format!("{} seconds ago", diff)
    } else if diff < 3600 {
        let mins = diff / 60;
        if mins == 1 {
            "1 minute ago".to_string()
        } else {
            format!("{} minutes ago", mins)
        }
    } else if diff < 86400 {
        let hours = diff / 3600;
        if hours == 1 {
            "1 hour ago".to_string()
        } else {
            format!("{} hours ago", hours)
        }
    } else {
        let days = diff / 86400;
        if days == 1 {
            "1 day ago".to_string()
        } else {
            format!("{} days ago", days)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_block_id() {
        assert_eq!(parse_block_id("latest"), "latest");
        assert_eq!(parse_block_id("LATEST"), "latest");
        assert_eq!(parse_block_id("pending"), "pending");
        assert_eq!(parse_block_id("0x123"), "0x123");
        assert_eq!(parse_block_id("12345"), "0x3039");
        assert_eq!(parse_block_id("invalid"), "latest");
    }

    #[test]
    fn test_parse_block_id_earliest() {
        assert_eq!(parse_block_id("earliest"), "earliest");
        assert_eq!(parse_block_id("EARLIEST"), "earliest");
    }

    #[test]
    fn test_parse_block_id_large_number() {
        assert_eq!(parse_block_id("1000000"), "0xf4240");
        assert_eq!(parse_block_id("15000000"), "0xe4e1c0");
    }

    #[test]
    fn test_parse_block_id_hex_hash() {
        let hash = "0xabc123def456789012345678901234567890123456789012345678901234abcd";
        assert_eq!(parse_block_id(hash), hash);
    }

    #[test]
    fn test_parse_hex_u64() {
        assert_eq!(parse_hex_u64("0x10"), Some(16));
        assert_eq!(parse_hex_u64("0xff"), Some(255));
        assert_eq!(parse_hex_u64("10"), Some(16));
        assert_eq!(parse_hex_u64("abc"), Some(2748));
    }

    #[test]
    fn test_parse_hex_u64_large() {
        assert_eq!(parse_hex_u64("0xffffffff"), Some(4294967295));
        assert_eq!(parse_hex_u64("0x1"), Some(1));
        assert_eq!(parse_hex_u64("0x0"), Some(0));
    }

    #[test]
    fn test_parse_hex_u64_edge_cases() {
        assert_eq!(parse_hex_u64(""), None);
        assert_eq!(parse_hex_u64("xyz"), None);
    }

    #[test]
    fn test_format_relative_time() {
        let now = 1000000u64;
        assert_eq!(format_relative_time(now, now), "0 seconds ago");
        assert_eq!(format_relative_time(now, now - 30), "30 seconds ago");
        assert_eq!(format_relative_time(now, now - 60), "1 minute ago");
        assert_eq!(format_relative_time(now, now - 120), "2 minutes ago");
        assert_eq!(format_relative_time(now, now - 3600), "1 hour ago");
        assert_eq!(format_relative_time(now, now - 7200), "2 hours ago");
        assert_eq!(format_relative_time(now, now - 86400), "1 day ago");
        assert_eq!(format_relative_time(now, now - 172800), "2 days ago");
        // Future timestamp
        assert_eq!(format_relative_time(now, now + 100), "just now");
    }

    #[test]
    fn test_format_relative_time_boundary() {
        let now = 1000000u64;
        // 59 seconds - still "seconds ago"
        assert_eq!(format_relative_time(now, now - 59), "59 seconds ago");
        // 59 minutes
        assert_eq!(format_relative_time(now, now - 3540), "59 minutes ago");
        // 23 hours
        assert_eq!(format_relative_time(now, now - 82800), "23 hours ago");
    }

    #[test]
    fn args_deserialize_default() {
        let json = serde_json::json!({});
        let args: GetBlockInfoArgs = serde_json::from_value(json).unwrap_or(GetBlockInfoArgs {
            block: None,
            simple_mode: false,
        });
        assert!(args.block.is_none());
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_with_block() {
        let json = serde_json::json!({
            "block": "12345",
            "simple_mode": true
        });
        let args: std::result::Result<GetBlockInfoArgs, _> = serde_json::from_value(json);
        assert!(args.is_ok());
        let args = args.unwrap();
        assert_eq!(args.block, Some("12345".to_string()));
        assert!(args.simple_mode);
    }
}
