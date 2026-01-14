use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CroLensError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Token not found: {0}")]
    TokenNotFound(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Service unavailable: {message}")]
    ServiceUnavailable {
        message: String,
        retry_after_secs: Option<u32>,
    },

    #[error("Simulation failed: {0}")]
    #[allow(dead_code)]
    SimulationFailed(String),

    #[error("Rate limit exceeded")]
    #[allow(dead_code)]
    RateLimitExceeded { retry_after_secs: Option<u32> },

    #[error("Unauthorized: {0}")]
    #[allow(dead_code)]
    Unauthorized(String),

    #[error("Payment required")]
    PaymentRequired {
        #[source]
        detail: Option<Box<CroLensError>>,
        data: Option<Value>,
    },

    #[error("Database error: {0}")]
    DbError(String),

    #[error("KV error: {0}")]
    KvError(String),
}

pub type Result<T> = std::result::Result<T, CroLensError>;

impl CroLensError {
    pub fn invalid_request(message: String) -> Self {
        Self::InvalidRequest(message)
    }

    pub fn method_not_found(message: String) -> Self {
        Self::MethodNotFound(message)
    }

    pub fn invalid_params(message: String) -> Self {
        Self::InvalidParams(message)
    }

    pub fn payment_required(data: Option<Value>) -> Self {
        Self::PaymentRequired { detail: None, data }
    }

    pub fn rate_limit_exceeded(retry_after_secs: Option<u32>) -> Self {
        Self::RateLimitExceeded { retry_after_secs }
    }

    pub fn unauthorized(message: String) -> Self {
        Self::Unauthorized(message)
    }

    pub fn service_unavailable(message: String, retry_after_secs: Option<u32>) -> Self {
        Self::ServiceUnavailable {
            message,
            retry_after_secs,
        }
    }

    pub fn to_json_rpc_error(&self) -> (i32, String, Option<Value>) {
        match self {
            Self::InvalidRequest(_) => (-32600, self.to_string(), None),
            Self::MethodNotFound(_) => (-32601, self.to_string(), None),
            Self::InvalidParams(_) => (-32602, self.to_string(), None),
            Self::InvalidAddress(_) => (-32602, self.to_string(), None),
            Self::TokenNotFound(_) => (-32602, self.to_string(), None),
            Self::RpcError(_) => (-32500, self.to_string(), None),
            Self::ServiceUnavailable {
                retry_after_secs, ..
            } => (
                -32501,
                self.to_string(),
                retry_after_secs.map(|v| serde_json::json!({ "retry_after": v })),
            ),
            Self::SimulationFailed(_) => (-32500, self.to_string(), None),
            Self::RateLimitExceeded { retry_after_secs } => (
                -32003,
                self.to_string(),
                retry_after_secs.map(|v| serde_json::json!({ "retry_after": v })),
            ),
            Self::Unauthorized(_) => (-32001, self.to_string(), None),
            Self::PaymentRequired { data, .. } => (-32002, self.to_string(), data.clone()),
            Self::DbError(_) => (-32500, self.to_string(), None),
            Self::KvError(_) => (-32500, self.to_string(), None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_invalid_request_code() {
        let err = CroLensError::invalid_request("bad".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32600);
    }

    #[test]
    fn maps_method_not_found_code() {
        let err = CroLensError::method_not_found("nope".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32601);
    }

    #[test]
    fn maps_invalid_params_code() {
        let err = CroLensError::invalid_params("bad params".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32602);
    }

    #[test]
    fn maps_invalid_address_as_invalid_params() {
        let err = CroLensError::InvalidAddress("0x1234".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32602);
    }

    #[test]
    fn maps_rpc_error_code() {
        let err = CroLensError::RpcError("rpc".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32500);
    }

    #[test]
    fn maps_rate_limit_code() {
        let err = CroLensError::rate_limit_exceeded(Some(60));
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32003);
    }

    #[test]
    fn maps_unauthorized_code() {
        let err = CroLensError::unauthorized("bad key".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32001);
    }

    #[test]
    fn maps_payment_required_code_and_data() {
        let data = serde_json::json!({ "payment_address": "0xabc" });
        let err = CroLensError::payment_required(Some(data.clone()));
        let (code, _, out) = err.to_json_rpc_error();
        assert_eq!(code, -32002);
        assert_eq!(out, Some(data));
    }

    #[test]
    fn rate_limit_includes_retry_after_data() {
        let err = CroLensError::rate_limit_exceeded(Some(123));
        let (code, _, out) = err.to_json_rpc_error();
        assert_eq!(code, -32003);
        assert_eq!(out, Some(serde_json::json!({ "retry_after": 123 })));
    }

    #[test]
    fn maps_db_error_code() {
        let err = CroLensError::DbError("db".to_string());
        let (code, _, _) = err.to_json_rpc_error();
        assert_eq!(code, -32500);
    }
}
