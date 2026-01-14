use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::CroLensError;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    pub fn error(id: Value, err: CroLensError) -> Self {
        let (code, message, data) = err.to_json_rpc_error();
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_success_response() {
        let id = serde_json::json!(1);
        let resp = JsonRpcResponse::success(id.clone(), serde_json::json!({ "ok": true }));
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, id);
        assert!(resp.error.is_none());
        assert!(resp.result.is_some());
    }

    #[test]
    fn builds_error_response() {
        let id = serde_json::json!("req-1");
        let resp = JsonRpcResponse::error(id.clone(), CroLensError::rate_limit_exceeded(Some(60)));
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, id);
        assert!(resp.result.is_none());
        let err = resp.error.expect("error must exist");
        assert_eq!(err.code, -32003);
    }
}

#[derive(Debug, Deserialize)]
pub struct ToolCallParams {
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

#[derive(Debug, Serialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}
