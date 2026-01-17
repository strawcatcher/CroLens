use serde::Deserialize;
use serde_json::Value;

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct SimpleModeArgs {
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_vvs_farms(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SimpleModeArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let pools = infra::config::list_dex_pools_cached(&services.db, &services.kv, "vvs").await?;
    let farms: Vec<Value> = pools
        .into_iter()
        .map(|p| {
            serde_json::json!({
                "pool_id": p.pool_id,
                "lp_address": p.lp_address.to_string(),
                "token0_symbol": p.token0_symbol,
                "token1_symbol": p.token1_symbol,
                "tvl_usd": Value::Null,
                "apy": Value::Null,
            })
        })
        .collect();

    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": format!("VVS farms: {}", farms.len()),
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({ "farms": farms, "meta": services.meta() }))
}

#[derive(Debug, Deserialize)]
struct VvsRewardsArgs {
    address: String,
    #[serde(default)]
    simple_mode: bool,
}

pub async fn get_vvs_rewards(services: &infra::Services, args: Value) -> Result<Value> {
    let input: VvsRewardsArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let _ = types::parse_address(&input.address)?;

    // Rewards require protocol-specific on-chain calls. Return an empty placeholder for now.
    if input.simple_mode {
        return Ok(serde_json::json!({
            "text": "VVS pending rewards: 0 (placeholder).",
            "meta": services.meta(),
        }));
    }

    Ok(serde_json::json!({
        "address": input.address,
        "rewards": [],
        "total_pending_vvs": "0",
        "meta": services.meta(),
    }))
}

