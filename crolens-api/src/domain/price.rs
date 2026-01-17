use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;

#[derive(Debug, Deserialize)]
struct GetTokenPriceArgs {
    tokens: Vec<String>,
    #[serde(default)]
    simple_mode: bool,
}

const MAX_TOKENS_PER_REQUEST: usize = 20;

fn validate_token_price_request(tokens: &[String]) -> Result<()> {
    if tokens.is_empty() {
        return Err(CroLensError::invalid_params(
            "tokens array must not be empty".to_string(),
        ));
    }
    if tokens.len() > MAX_TOKENS_PER_REQUEST {
        return Err(CroLensError::invalid_params(format!(
            "Maximum {MAX_TOKENS_PER_REQUEST} tokens per request"
        )));
    }
    Ok(())
}

/// Get prices for multiple tokens
pub async fn get_token_price(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetTokenPriceArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    validate_token_price_request(&input.tokens)?;

    // Load token list.
    let all_tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;

    // Resolve requested tokens.
    let mut requested_tokens = Vec::new();
    let mut not_found = Vec::new();

    for query in &input.tokens {
        match infra::token::resolve_token(&all_tokens, query) {
            Ok(token) => requested_tokens.push(token),
            Err(_) => not_found.push(query.clone()),
        }
    }

    if requested_tokens.is_empty() {
        return Err(CroLensError::invalid_params(format!(
            "No valid tokens found. Unknown: {:?}",
            not_found
        )));
    }

    // Fetch prices in batch.
    let price_map = infra::price::get_prices_usd_batch(services, &requested_tokens).await?;

    // Build result.
    let mut prices = Vec::new();
    for token in &requested_tokens {
        let price_usd = price_map.get(&token.address).copied().unwrap_or(0.0);

        // Determine source/confidence.
        let (source, confidence) = if token.is_stablecoin {
            ("pegged", "high")
        } else if price_usd > 0.0 {
            // Simplified heuristic: if we have a price, mark it as high confidence.
            ("derived", "high")
        } else {
            ("unknown", "low")
        };

        prices.push(serde_json::json!({
            "symbol": token.symbol,
            "address": token.address.to_string(),
            "price_usd": format!("{:.8}", price_usd),
            "source": source,
            "confidence": confidence
        }));
    }

    // Build response.
    if input.simple_mode {
        let text_parts: Vec<String> = prices
            .iter()
            .map(|p| {
                let symbol = p.get("symbol").and_then(|v| v.as_str()).unwrap_or("?");
                let price = p.get("price_usd").and_then(|v| v.as_str()).unwrap_or("0");
                let price_f64: f64 = price.parse().unwrap_or(0.0);
                format!("{}: ${:.6}", symbol, price_f64)
            })
            .collect();
        let text = text_parts.join(" | ");
        return Ok(serde_json::json!({ "text": text }));
    }

    let mut result = serde_json::json!({
        "prices": prices,
        "meta": services.meta()
    });

    // Add warnings for unknown tokens.
    if !not_found.is_empty() {
        result["warnings"] = serde_json::json!([{
            "type": "tokens_not_found",
            "tokens": not_found
        }]);
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_request_rejects_empty() {
        let tokens: Vec<String> = Vec::new();
        let err = validate_token_price_request(&tokens).unwrap_err();
        assert!(matches!(err, CroLensError::InvalidParams(_)));
    }

    #[test]
    fn validate_request_rejects_over_limit() {
        let tokens = vec!["CRO".to_string(); MAX_TOKENS_PER_REQUEST + 1];
        let err = validate_token_price_request(&tokens).unwrap_err();
        assert!(matches!(err, CroLensError::InvalidParams(_)));
    }

    #[test]
    fn validate_request_allows_limit() {
        let tokens = vec!["CRO".to_string(); MAX_TOKENS_PER_REQUEST];
        validate_token_price_request(&tokens).expect("should allow max tokens");
    }

    #[test]
    fn args_deserialize_defaults() {
        let json = serde_json::json!({ "tokens": ["CRO"] });
        let args: GetTokenPriceArgs = serde_json::from_value(json).expect("args should parse");
        assert_eq!(args.tokens, vec!["CRO".to_string()]);
        assert!(!args.simple_mode);
    }

    #[test]
    fn args_deserialize_simple_mode_true() {
        let json = serde_json::json!({ "tokens": ["CRO"], "simple_mode": true });
        let args: GetTokenPriceArgs = serde_json::from_value(json).expect("args should parse");
        assert!(args.simple_mode);
    }
}
