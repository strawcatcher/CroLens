use crolens_api::error::CroLensError;
use crolens_api::mcp::protocol::JsonRpcResponse;

#[test]
fn json_rpc_error_payload_is_well_formed() {
    let resp = JsonRpcResponse::error(
        serde_json::json!(1),
        CroLensError::rate_limit_exceeded(Some(60)),
    );
    let value = serde_json::to_value(&resp).expect("must serialize");

    assert_eq!(value.get("jsonrpc").and_then(|v| v.as_str()), Some("2.0"));
    assert_eq!(value.get("id").and_then(|v| v.as_i64()), Some(1));
    assert!(value.get("result").is_none());

    let err = value.get("error").expect("error must exist");
    assert_eq!(err.get("code").and_then(|v| v.as_i64()), Some(-32003));
    assert!(err.get("message").and_then(|v| v.as_str()).is_some());
}
