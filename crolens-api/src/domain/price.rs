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

/// Get prices for multiple tokens
pub async fn get_token_price(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetTokenPriceArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    if input.tokens.is_empty() {
        return Err(CroLensError::invalid_params(
            "tokens array must not be empty".to_string(),
        ));
    }

    if input.tokens.len() > 20 {
        return Err(CroLensError::invalid_params(
            "Maximum 20 tokens per request".to_string(),
        ));
    }

    // 获取所有代币列表
    let all_tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;

    // 解析请求的代币
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

    // 批量获取价格
    let price_map = infra::price::get_prices_usd_batch(services, &requested_tokens).await?;

    // 构建结果
    let mut prices = Vec::new();
    for token in &requested_tokens {
        let price_usd = price_map.get(&token.address).copied().unwrap_or(0.0);

        // 判断价格来源和置信度
        let (source, confidence) = if token.is_stablecoin {
            ("pegged", "high")
        } else if price_usd > 0.0 {
            // 简化判断：有价格就是 high，否则是 low
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

    // 返回结果
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

    // 如果有未找到的代币，添加警告
    if !not_found.is_empty() {
        result["warnings"] = serde_json::json!([{
            "type": "tokens_not_found",
            "tokens": not_found
        }]);
    }

    Ok(result)
}
