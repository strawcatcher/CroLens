use alloy_primitives::{Address, U256};
use worker::d1::D1Type;
use worker::{D1Database, Env};

use crate::error::{CroLensError, Result};
use crate::infra;
use crate::types;

#[derive(Debug, Clone)]
pub struct X402Config {
    pub payment_address: Address,
    pub price_per_credit_wei: U256,
    pub topup_credits: i64,
}

impl X402Config {
    pub async fn try_load(env: &Env, db: &D1Database) -> Result<Option<Self>> {
        let payment_address = match env.var("X402_PAYMENT_ADDRESS") {
            Ok(v) => v.to_string(),
            Err(_) => return Ok(None),
        };
        if payment_address.trim().is_empty() {
            return Ok(None);
        }

        let payment_address = types::parse_address(&payment_address)?;
        let price_per_credit_wei = load_price_per_credit_wei(db).await?;
        let topup_credits = env
            .var("X402_TOPUP_CREDITS")
            .ok()
            .and_then(|v| v.to_string().parse::<i64>().ok())
            .filter(|v| *v > 0)
            .unwrap_or(1000);

        Ok(Some(Self {
            payment_address,
            price_per_credit_wei,
            topup_credits,
        }))
    }

    pub fn topup_amount_wei(&self) -> U256 {
        self.price_per_credit_wei
            .saturating_mul(U256::from(self.topup_credits as u64))
    }
}

async fn load_price_per_credit_wei(db: &D1Database) -> Result<U256> {
    let key_arg = D1Type::Text("x402.price_per_credit");
    let statement = db
        .prepare("SELECT value FROM system_config WHERE key = ?1 LIMIT 1")
        .bind_refs([&key_arg])
        .map_err(|err| CroLensError::DbError(err.to_string()))?;
    let result = infra::db::run("load_price_per_credit_wei", statement.all()).await?;
    let rows: Vec<serde_json::Value> = result
        .results()
        .map_err(|err| CroLensError::DbError(err.to_string()))?;

    let value = rows
        .first()
        .and_then(|row| row.get("value"))
        .and_then(|v| v.as_str())
        .unwrap_or("10000000000000000");

    types::parse_u256_dec(value).or_else(|_| Ok(U256::from(10_000_000_000_000_000u64)))
}
