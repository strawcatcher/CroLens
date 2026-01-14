use std::future::Future;
use std::time::Duration;

use futures_util::future::{select, Either, FutureExt};
use futures_util::pin_mut;
use worker::{console_warn, Delay};

use crate::error::{CroLensError, Result};
use crate::types;

const DB_TIMEOUT: Duration = Duration::from_secs(5);
const SLOW_QUERY_THRESHOLD_MS: i64 = 500;

pub async fn run<T>(label: &str, fut: impl Future<Output = worker::Result<T>>) -> Result<T> {
    let started = types::now_ms();

    let fut = fut.fuse();
    let timeout = Delay::from(DB_TIMEOUT).fuse();
    pin_mut!(fut, timeout);

    match select(fut, timeout).await {
        Either::Left((result, _)) => {
            let elapsed_ms = types::now_ms().saturating_sub(started);
            if elapsed_ms > SLOW_QUERY_THRESHOLD_MS {
                console_warn!("[WARN] Slow DB query: {} ({}ms)", label, elapsed_ms);
            }
            result.map_err(|err| CroLensError::DbError(err.to_string()))
        }
        Either::Right((_elapsed, _)) => Err(CroLensError::DbError(format!(
            "DB query timeout after {}ms: {}",
            DB_TIMEOUT.as_millis(),
            label
        ))),
    }
}
