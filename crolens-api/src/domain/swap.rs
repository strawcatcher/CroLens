use alloy_primitives::{Address, Bytes, U256};
use alloy_sol_types::SolCall;
use serde::Deserialize;
use serde_json::Value;

use crate::abi;
use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Deserialize)]
struct SwapArgs {
    from: String,
    token_in: String,
    token_out: String,
    amount_in: String,
    slippage_bps: u16,
}

pub async fn construct_swap_tx(services: &infra::Services, args: Value) -> Result<Value> {
    let input: SwapArgs = serde_json::from_value(args)
        .map_err(|err| CroLensError::invalid_params(format!("Invalid input: {err}")))?;

    let from = types::parse_address(&input.from)?;
    let amount_in = types::parse_u256_dec(&input.amount_in)?;
    let rpc = services.rpc()?;

    let tokens = infra::token::list_tokens_cached(&services.db, &services.kv).await?;
    let wcro = infra::token::resolve_token(&tokens, "WCRO").ok();
    let wcro_address = wcro.as_ref().map(|t| t.address);

    let is_native_out = input.token_out.trim().eq_ignore_ascii_case("cro");
    let token_out_address = if is_native_out {
        wcro_address.ok_or_else(|| CroLensError::TokenNotFound("WCRO".to_string()))?
    } else {
        infra::token::resolve_token(&tokens, &input.token_out)?.address
    };

    let is_native_in = input.token_in.trim().eq_ignore_ascii_case("cro");
    if is_native_in && is_native_out {
        return Err(CroLensError::invalid_params(
            "token_in and token_out cannot both be CRO".to_string(),
        ));
    }
    let token_in = if is_native_in {
        None
    } else {
        Some(infra::token::resolve_token(&tokens, &input.token_in)?)
    };

    // 并行获取 router 和 factory
    let (router, factory) = futures_util::future::try_join(
        infra::config::get_protocol_contract(&services.db, "vvs", "router"),
        infra::config::get_protocol_contract(&services.db, "vvs", "factory"),
    )
    .await?;

    let path = build_path(
        factory,
        wcro_address,
        token_in.as_ref().map(|t| t.address),
        token_out_address,
        rpc,
    )
    .await?;
    if is_native_out && path.last().copied() != wcro_address {
        return Err(CroLensError::invalid_params(
            "Swap path must end with WCRO for CRO output".to_string(),
        ));
    }
    let deadline = (types::now_seconds() + 1200) as u64;

    // 并行获取报价和价格影响
    let ((estimated_out, minimum_out), price_impact_bps) = futures_util::future::try_join(
        quote_amounts(router, amount_in, &path, rpc, input.slippage_bps),
        estimate_price_impact_bps(factory, &path, amount_in, rpc),
    )
    .await?;
    let price_impact = format_percent_from_basis_points(price_impact_bps);

    let mut steps: Vec<Value> = Vec::new();
    let mut step_index: u8 = 1;

    if let Some(t_in) = &token_in {
        let allowance = get_allowance(t_in.address, from, router, rpc).await?;
        if allowance < amount_in {
            let approve = abi::approveCall {
                spender: router,
                amount: amount_in,
            }
            .abi_encode();
            steps.push(serde_json::json!({
                "step_index": step_index,
                "type": "approval",
                "description": format!("Approve router to spend {}", t_in.symbol),
                "tx_data": {
                    "to": t_in.address.to_string(),
                    "data": types::bytes_to_hex0x(&approve),
                    "value": "0"
                },
                "status": "pending"
            }));
            step_index = step_index.saturating_add(1);
        }
    }

    let (swap_to, swap_data, swap_value) = build_swap_calldata(SwapCalldataParams {
        router,
        from,
        token_in: token_in.as_ref().map(|t| t.address),
        native_out: is_native_out,
        amount_in,
        amount_out_min: minimum_out,
        path: &path,
        deadline,
    })?;
    let status = if steps.is_empty() {
        "pending"
    } else {
        "blocked"
    };
    steps.push(serde_json::json!({
        "step_index": step_index,
        "type": "swap",
        "description": "Execute swap on VVS router",
        "tx_data": { "to": swap_to.to_string(), "data": types::bytes_to_hex0x(&swap_data), "value": swap_value.to_string() },
        "status": status
    }));

    let mut simulation_verified = false;
    if steps.len() == 1 {
        if let Some(tenderly) = services.tenderly() {
            let data_hex = types::bytes_to_hex0x(&swap_data);
            let sim = tenderly
                .simulate(from, swap_to, &data_hex, swap_value, None)
                .await?;
            if !sim.success {
                return Err(CroLensError::SimulationFailed(
                    sim.error_message
                        .unwrap_or_else(|| "Tenderly simulation failed".to_string()),
                ));
            }
            simulation_verified = true;
        }
    }

    Ok(serde_json::json!({
        "operation_id": format!("swap_{}_{}_{}", input.token_in, input.token_out, types::now_ms()),
        "estimated_out": estimated_out.to_string(),
        "minimum_out": minimum_out.to_string(),
        "price_impact": price_impact,
        "simulation_verified": simulation_verified,
        "steps": steps,
        "meta": services.meta()
    }))
}

async fn estimate_price_impact_bps(
    factory: Address,
    path: &[Address],
    amount_in: U256,
    rpc: &infra::rpc::RpcClient,
) -> Result<U256> {
    if amount_in.is_zero() {
        return Ok(U256::ZERO);
    }
    if path.len() < 2 {
        return Ok(U256::ZERO);
    }

    let mut ideal_amount = amount_in;
    let mut actual_amount = amount_in;

    for hop in path.windows(2) {
        let (reserve_in, reserve_out) = get_pair_reserves(factory, hop[0], hop[1], rpc).await?;
        ideal_amount = compute_ideal_out(ideal_amount, reserve_in, reserve_out);
        actual_amount = compute_actual_out(actual_amount, reserve_in, reserve_out);
    }

    if ideal_amount.is_zero() {
        return Ok(U256::ZERO);
    }

    let diff = ideal_amount.saturating_sub(actual_amount);
    Ok(diff.saturating_mul(U256::from(10_000u64)) / ideal_amount)
}

async fn get_pair_reserves(
    factory: Address,
    token_in: Address,
    token_out: Address,
    rpc: &infra::rpc::RpcClient,
) -> Result<(U256, U256)> {
    let call = abi::getPairCall {
        tokenA: token_in,
        tokenB: token_out,
    }
    .abi_encode();
    let data = rpc.eth_call(factory, Bytes::from(call)).await?;
    let decoded = abi::getPairCall::abi_decode_returns(&data, true)
        .map_err(|err| CroLensError::RpcError(format!("getPair decode failed: {err}")))?;

    if decoded.pair == Address::ZERO {
        return Err(CroLensError::RpcError(
            "Pair not found for price impact calculation".to_string(),
        ));
    }

    let reserves_call = abi::getReservesCall {}.abi_encode();
    let reserves_data = rpc
        .eth_call(decoded.pair, Bytes::from(reserves_call))
        .await?;
    let reserves_ret = abi::getReservesCall::abi_decode_returns(&reserves_data, true)
        .map_err(|err| CroLensError::RpcError(format!("getReserves decode failed: {err}")))?;

    let reserve0 = U256::from(reserves_ret.reserve0);
    let reserve1 = U256::from(reserves_ret.reserve1);
    if token_in.as_slice() < token_out.as_slice() {
        Ok((reserve0, reserve1))
    } else {
        Ok((reserve1, reserve0))
    }
}

fn compute_ideal_out(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
    if reserve_in.is_zero() {
        return U256::ZERO;
    }
    amount_in.saturating_mul(reserve_out) / reserve_in
}

fn compute_actual_out(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
    if reserve_in.is_zero() || reserve_out.is_zero() {
        return U256::ZERO;
    }

    let amount_in_with_fee = amount_in.saturating_mul(U256::from(997u64));
    let numerator = amount_in_with_fee.saturating_mul(reserve_out);
    let denominator = reserve_in
        .saturating_mul(U256::from(1000u64))
        .saturating_add(amount_in_with_fee);
    if denominator.is_zero() {
        return U256::ZERO;
    }
    numerator / denominator
}

#[cfg(test)]
fn calculate_price_impact_bps_single_pair(
    amount_in: U256,
    reserve_in: U256,
    reserve_out: U256,
) -> U256 {
    let ideal_out = compute_ideal_out(amount_in, reserve_in, reserve_out);
    if ideal_out.is_zero() {
        return U256::ZERO;
    }
    let actual_out = compute_actual_out(amount_in, reserve_in, reserve_out);
    let diff = ideal_out.saturating_sub(actual_out);
    diff.saturating_mul(U256::from(10_000u64)) / ideal_out
}

fn format_percent_from_basis_points(bp: U256) -> String {
    let hundred = U256::from(100u64);
    let int_part = bp / hundred;
    let mut frac = (bp % hundred).to_string();
    if frac.len() == 1 {
        frac.insert(0, '0');
    }
    if frac.len() > 2 {
        frac.truncate(2);
    }
    format!("{}.{}", int_part, frac)
}

async fn build_path(
    factory: Address,
    wcro: Option<Address>,
    token_in: Option<Address>,
    token_out: Address,
    rpc: &infra::rpc::RpcClient,
) -> Result<Vec<Address>> {
    let mut direct = Vec::new();
    match token_in {
        Some(addr_in) => {
            direct.push(addr_in);
            direct.push(token_out);
        }
        None => {
            let Some(wcro_addr) = wcro else {
                return Err(CroLensError::TokenNotFound("WCRO".to_string()));
            };
            direct.push(wcro_addr);
            direct.push(token_out);
        }
    }

    if is_pair_available(factory, direct[0], direct[1], rpc).await? {
        return Ok(direct);
    }

    let Some(wcro_addr) = wcro else {
        return Err(CroLensError::TokenNotFound("WCRO".to_string()));
    };

    if token_in.is_some() && direct[0] != wcro_addr && token_out != wcro_addr {
        return Ok(vec![direct[0], wcro_addr, token_out]);
    }

    Ok(direct)
}

async fn is_pair_available(
    factory: Address,
    a: Address,
    b: Address,
    rpc: &infra::rpc::RpcClient,
) -> Result<bool> {
    let call = abi::getPairCall {
        tokenA: a,
        tokenB: b,
    }
    .abi_encode();
    let data = rpc.eth_call(factory, Bytes::from(call)).await?;
    let decoded = abi::getPairCall::abi_decode_returns(&data, true)
        .map_err(|err| CroLensError::RpcError(format!("getPair decode failed: {err}")))?;
    Ok(decoded.pair != Address::ZERO)
}

async fn quote_amounts(
    router: Address,
    amount_in: U256,
    path: &[Address],
    rpc: &infra::rpc::RpcClient,
    slippage_bps: u16,
) -> Result<(U256, U256)> {
    let call = abi::getAmountsOutCall {
        amountIn: amount_in,
        path: path.to_vec(),
    }
    .abi_encode();
    let data = rpc.eth_call(router, Bytes::from(call)).await?;
    let decoded = abi::getAmountsOutCall::abi_decode_returns(&data, true)
        .map_err(|err| CroLensError::RpcError(format!("getAmountsOut decode failed: {err}")))?;
    let last = decoded.amounts.last().cloned().unwrap_or(U256::ZERO);
    let minimum =
        last.saturating_mul(U256::from(10_000u64 - slippage_bps as u64)) / U256::from(10_000u64);
    Ok((last, minimum))
}

async fn get_allowance(
    token: Address,
    owner: Address,
    spender: Address,
    rpc: &infra::rpc::RpcClient,
) -> Result<U256> {
    let call = abi::allowanceCall { owner, spender }.abi_encode();
    let data = rpc.eth_call(token, Bytes::from(call)).await?;
    let decoded = abi::allowanceCall::abi_decode_returns(&data, true)
        .map_err(|err| CroLensError::RpcError(format!("allowance decode failed: {err}")))?;
    Ok(decoded._0)
}

fn build_swap_calldata(params: SwapCalldataParams<'_>) -> Result<(Address, Vec<u8>, U256)> {
    if params.token_in.is_none() {
        let call = abi::swapExactETHForTokensCall {
            amountOutMin: params.amount_out_min,
            path: params.path.to_vec(),
            to: params.from,
            deadline: U256::from(params.deadline),
        };
        return Ok((params.router, call.abi_encode(), params.amount_in));
    }

    if params.native_out {
        let call = abi::swapExactTokensForETHCall {
            amountIn: params.amount_in,
            amountOutMin: params.amount_out_min,
            path: params.path.to_vec(),
            to: params.from,
            deadline: U256::from(params.deadline),
        };
        return Ok((params.router, call.abi_encode(), U256::ZERO));
    }

    let call = abi::swapExactTokensForTokensCall {
        amountIn: params.amount_in,
        amountOutMin: params.amount_out_min,
        path: params.path.to_vec(),
        to: params.from,
        deadline: U256::from(params.deadline),
    };
    Ok((params.router, call.abi_encode(), U256::ZERO))
}

struct SwapCalldataParams<'a> {
    router: Address,
    from: Address,
    token_in: Option<Address>,
    native_out: bool,
    amount_in: U256,
    amount_out_min: U256,
    path: &'a [Address],
    deadline: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_swap_exact_tokens_for_eth_when_native_out() {
        let router = types::parse_address("0x1111111111111111111111111111111111111111").unwrap();
        let from = types::parse_address("0x2222222222222222222222222222222222222222").unwrap();
        let token_in = types::parse_address("0x3333333333333333333333333333333333333333").unwrap();
        let wcro = types::parse_address("0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23").unwrap();
        let path = vec![token_in, wcro];

        let (to, data, value) = build_swap_calldata(SwapCalldataParams {
            router,
            from,
            token_in: Some(token_in),
            native_out: true,
            amount_in: U256::from(1234u64),
            amount_out_min: U256::from(1200u64),
            path: &path,
            deadline: 1_700_000_000,
        })
        .expect("build calldata");

        assert_eq!(to, router);
        assert_eq!(value, U256::ZERO);

        let expected = abi::swapExactTokensForETHCall {
            amountIn: U256::from(1234u64),
            amountOutMin: U256::from(1200u64),
            path,
            to: from,
            deadline: U256::from(1_700_000_000u64),
        }
        .abi_encode();

        assert_eq!(data, expected);
    }

    #[test]
    fn calculates_price_impact_bps_sane_ranges() {
        let reserve_in = U256::from(1_000_000u64);
        let reserve_out = U256::from(1_000_000u64);

        let small =
            calculate_price_impact_bps_single_pair(U256::from(1_000u64), reserve_in, reserve_out);
        assert!(
            small < U256::from(100u64),
            "expected <1% for small trade, got {small}"
        );

        let large =
            calculate_price_impact_bps_single_pair(U256::from(200_000u64), reserve_in, reserve_out);
        assert!(
            large > U256::from(1000u64),
            "expected >10% for large trade, got {large}"
        );
    }

    #[test]
    fn formats_basis_points_as_percent_string() {
        assert_eq!(format_percent_from_basis_points(U256::ZERO), "0.00");
        assert_eq!(format_percent_from_basis_points(U256::from(5u64)), "0.05");
        assert_eq!(format_percent_from_basis_points(U256::from(123u64)), "1.23");
    }
}
