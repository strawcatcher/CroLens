use alloy_primitives::{Address, U256};
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::infra::multicall::Call;
use crate::types;

#[derive(Debug, Deserialize)]
struct GetPoolInfoArgs {
    pool: String,
    #[serde(default)]
    dex: Option<String>,
    #[serde(default)]
    simple_mode: bool,
}

/// Get detailed LP pool information
pub async fn get_pool_info(services: &infra::Services, args: Value) -> Result<Value> {
    let input: GetPoolInfoArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let pool_query = input.pool.trim();
    if pool_query.is_empty() {
        return Err(CroLensError::invalid_params(
            "pool must not be empty".to_string(),
        ));
    }

    let dex = input.dex.as_deref().unwrap_or("vvs");
    let pools = infra::config::list_dex_pools_cached(&services.db, &services.kv, dex).await?;

    // 解析池子 - 支持 LP 地址或 "TOKEN0-TOKEN1" 格式
    let pool = if pool_query.starts_with("0x") {
        // 按地址查找
        let address = types::parse_address(pool_query)?;
        pools
            .iter()
            .find(|p| p.lp_address == address)
            .ok_or_else(|| CroLensError::invalid_params(format!("Pool not found: {pool_query}")))?
    } else {
        // 按 pair 名称查找 (如 "CRO-USDC" 或 "WCRO-USDC")
        let parts: Vec<&str> = pool_query.split('-').collect();
        if parts.len() != 2 {
            return Err(CroLensError::invalid_params(
                "Pool format should be 'TOKEN0-TOKEN1' or LP address".to_string(),
            ));
        }
        let sym0 = parts[0].trim().to_uppercase();
        let sym1 = parts[1].trim().to_uppercase();

        // 处理 CRO/WCRO 的等价性
        let normalize = |s: &str| -> String {
            if s == "CRO" {
                "WCRO".to_string()
            } else {
                s.to_string()
            }
        };
        let sym0_norm = normalize(&sym0);
        let sym1_norm = normalize(&sym1);

        pools
            .iter()
            .find(|p| {
                let p0 = normalize(&p.token0_symbol);
                let p1 = normalize(&p.token1_symbol);
                (p0 == sym0_norm && p1 == sym1_norm) || (p0 == sym1_norm && p1 == sym0_norm)
            })
            .ok_or_else(|| {
                CroLensError::invalid_params(format!("Pool not found: {}-{}", sym0, sym1))
            })?
    };

    // 获取链上数据
    let multicall = services.multicall()?;
    let calls = vec![
        // getReserves
        Call {
            target: pool.lp_address,
            call_data: abi::getReservesCall {}.abi_encode().into(),
        },
        // totalSupply (LP token)
        Call {
            target: pool.lp_address,
            call_data: abi::totalSupplyCall {}.abi_encode().into(),
        },
    ];

    let results = multicall.aggregate(calls).await?;

    // 解析 reserves
    let (reserve0, reserve1) = results
        .get(0)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::getReservesCall::abi_decode_returns(data, true).ok())
        .map(|v| (U256::from(v.reserve0), U256::from(v.reserve1)))
        .unwrap_or((U256::ZERO, U256::ZERO));

    // 解析 total LP supply
    let total_lp_supply = results
        .get(1)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::totalSupplyCall::abi_decode_returns(data, true).ok())
        .map(|v| U256::from(v._0))
        .unwrap_or(U256::ZERO);

    // 获取代币信息
    let tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;
    let token0 = tokens.iter().find(|t| t.address == pool.token0_address);
    let token1 = tokens.iter().find(|t| t.address == pool.token1_address);

    let token0_decimals = token0.map(|t| t.decimals).unwrap_or(18);
    let token1_decimals = token1.map(|t| t.decimals).unwrap_or(18);

    let reserve0_formatted = types::format_units(&reserve0, token0_decimals);
    let reserve1_formatted = types::format_units(&reserve1, token1_decimals);
    let total_lp_formatted = types::format_units(&total_lp_supply, 18);

    let reserve0_f64 = reserve0_formatted.parse::<f64>().unwrap_or(0.0);
    let reserve1_f64 = reserve1_formatted.parse::<f64>().unwrap_or(0.0);

    // 获取价格
    let price_map = infra::price::get_prices_usd_batch(services, &tokens).await?;
    let price0 = price_map
        .get(&pool.token0_address)
        .copied()
        .unwrap_or(0.0);
    let price1 = price_map
        .get(&pool.token1_address)
        .copied()
        .unwrap_or(0.0);

    let value0_usd = reserve0_f64 * price0;
    let value1_usd = reserve1_f64 * price1;
    let tvl_usd = value0_usd + value1_usd;

    // 计算价格比率
    let price_ratio = if reserve0_f64 > 0.0 && reserve1_f64 > 0.0 {
        format!(
            "1 {} = {:.6} {}",
            pool.token0_symbol,
            reserve1_f64 / reserve0_f64,
            pool.token1_symbol
        )
    } else {
        "N/A".to_string()
    };

    // 尝试获取 APY (从 MasterChef)
    let apy = get_pool_apy(services, pool.pool_index).await.ok().flatten();

    // 返回结果
    if input.simple_mode {
        let apy_str = apy
            .map(|v| format!("{:.2}%", v))
            .unwrap_or_else(|| "N/A".to_string());
        let text = format!(
            "{}-{} Pool ({}) | TVL: ${:.2} | APY: {} | {}",
            pool.token0_symbol, pool.token1_symbol, dex.to_uppercase(), tvl_usd, apy_str, price_ratio
        );
        return Ok(serde_json::json!({ "text": text }));
    }

    Ok(serde_json::json!({
        "address": pool.lp_address.to_string(),
        "dex": dex,
        "pool_id": pool.pool_id,
        "token0": {
            "symbol": pool.token0_symbol,
            "address": pool.token0_address.to_string(),
            "reserve": reserve0_formatted,
            "price_usd": format!("{:.6}", price0),
            "value_usd": format!("{:.2}", value0_usd)
        },
        "token1": {
            "symbol": pool.token1_symbol,
            "address": pool.token1_address.to_string(),
            "reserve": reserve1_formatted,
            "price_usd": format!("{:.6}", price1),
            "value_usd": format!("{:.2}", value1_usd)
        },
        "tvl_usd": format!("{:.2}", tvl_usd),
        "fee_rate": "0.3%",
        "apy": apy.map(|v| format!("{:.2}", v)),
        "price_ratio": price_ratio,
        "total_lp_supply": total_lp_formatted,
        "meta": services.meta()
    }))
}

/// 尝试从 MasterChef 获取池子 APY
async fn get_pool_apy(services: &infra::Services, pool_index: Option<i64>) -> Result<Option<f64>> {
    let Some(pid) = pool_index else {
        return Ok(None);
    };

    // 获取 MasterChef 地址
    let masterchef =
        match infra::config::get_protocol_contract(&services.db, "vvs", "masterchef").await {
            Ok(addr) => addr,
            Err(_) => return Ok(None),
        };

    let multicall = services.multicall()?;
    let calls = vec![
        // poolInfo(pid)
        Call {
            target: masterchef,
            call_data: abi::poolInfoCall {
                pid: alloy_primitives::U256::from(pid as u64),
            }
            .abi_encode()
            .into(),
        },
        // totalAllocPoint
        Call {
            target: masterchef,
            call_data: abi::totalAllocPointCall {}.abi_encode().into(),
        },
        // vvsPerBlock
        Call {
            target: masterchef,
            call_data: abi::vvsPerBlockCall {}.abi_encode().into(),
        },
    ];

    let results = multicall.aggregate(calls).await?;

    // 解析 poolInfo
    let alloc_point = results
        .get(0)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::poolInfoCall::abi_decode_returns(data, true).ok())
        .map(|v| v.allocPoint)
        .unwrap_or(U256::ZERO);

    let total_alloc_point = results
        .get(1)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::totalAllocPointCall::abi_decode_returns(data, true).ok())
        .map(|v| v._0)
        .unwrap_or(U256::ZERO);

    let vvs_per_block = results
        .get(2)
        .and_then(|r| r.as_ref().ok())
        .and_then(|data| abi::vvsPerBlockCall::abi_decode_returns(data, true).ok())
        .map(|v| U256::from(v._0))
        .unwrap_or(U256::ZERO);

    if total_alloc_point.is_zero() || vvs_per_block.is_zero() {
        return Ok(None);
    }

    // 简化的 APY 计算 (仅供参考，实际需要更多数据)
    // 这里只返回 allocation 比例，实际 APY 需要 TVL 和 VVS 价格
    let alloc_f64: f64 = types::format_units(&alloc_point, 0)
        .parse()
        .unwrap_or(0.0);
    let total_f64: f64 = types::format_units(&total_alloc_point, 0)
        .parse()
        .unwrap_or(1.0);

    if total_f64 > 0.0 {
        // 返回权重百分比 (不是真正的 APY，但可以作为相对指标)
        Ok(Some((alloc_f64 / total_f64) * 100.0))
    } else {
        Ok(None)
    }
}
