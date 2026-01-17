#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

echo "[mcp] tools/list"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' -H "CF-Connecting-IP: 203.0.113.20"
assert_eq "200" "${HTTP_STATUS}" "tools/list should return 200"
assert_eq "null" "$(json_get '.error')" "tools/list should not return error"
assert_eq "30" "$(json_get '.result.tools | length')" "tools/list should return 30 tools"

echo "[mcp] tools/call free tier get_account_summary (expected success)"
http_post_json "${BASE_URL}/" "$(jq -nc --arg address "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_account_summary","arguments":{"address":$address,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 203.0.113.21" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "free tier get_account_summary should return 200"
assert_eq "null" "$(json_get '.error')" "expected no json-rpc error"
assert_contains "$(json_get '.result.text')" "Wallet tokens:" "expected summary text"

echo "[mcp] tools/call get_account_summary invalid address (expected invalid params)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_account_summary","arguments":{"address":"0xabc","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 203.0.113.22" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "400" "${HTTP_STATUS}" "invalid address should return 400"
assert_eq "-32602" "$(json_get '.error.code')" "expected -32602 invalid params"
assert_contains "$(json_get '.error.message')" "Invalid address" "expected invalid address message"

echo "[mcp] tools/call without x-api-key (expected invalid params)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_account_summary","arguments":{}}}' \
  -H "CF-Connecting-IP: 203.0.113.23"
assert_eq "400" "${HTTP_STATUS}" "tools/call without key should return 400"
assert_eq "-32602" "$(json_get '.error.code')" "expected -32602 invalid params"
assert_contains "$(json_get '.error.message')" "Missing API key header" "expected missing api key message"

echo "[mcp] tools/call free tier search_contract (expected success)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":5}}}' \
  -H "CF-Connecting-IP: 203.0.113.24" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "free tier search_contract should return 200"
assert_eq "null" "$(json_get '.error')" "expected no json-rpc error"
assert_ne "null" "$(json_get '.result.results')" "expected results array"
assert_ne "0" "$(json_get '.result.results | length')" "expected non-empty results"

echo "[mcp] tools/call pro tier search_contract (expected success + credit deducted)"
http_get "${BASE_URL}/x402/status" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "200" "${HTTP_STATUS}" "status should return 200"
credits_before="$(json_get '.credits')"

http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":5}}}' \
  -H "CF-Connecting-IP: 203.0.113.25" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "200" "${HTTP_STATUS}" "pro tier search_contract should return 200"
assert_eq "null" "$(json_get '.error')" "expected no json-rpc error"
assert_ne "null" "$(json_get '.result.results')" "expected results array"
assert_ne "0" "$(json_get '.result.results | length')" "expected non-empty results"

http_get "${BASE_URL}/x402/status" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "200" "${HTTP_STATUS}" "status should return 200"
credits_after="$(json_get '.credits')"
expected_after="$((credits_before - 1))"
assert_eq "${expected_after}" "${credits_after}" "expected 1 credit deducted"

echo "[mcp] tools/call pro tier decode_transaction (expected success)"
http_post_json "${BASE_URL}/" "$(jq -nc --arg tx "${TEST_TX_VALID}" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"decode_transaction","arguments":{"tx_hash":$tx,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 203.0.113.26" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "200" "${HTTP_STATUS}" "decode_transaction should return 200"
assert_eq "null" "$(json_get '.error')" "expected no json-rpc error"
assert_contains "$(json_get '.result.text')" "Transfer:" "expected decoded summary"

echo "[mcp] tools/call unknown tool (expected method not found)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"unknown_tool","arguments":{}}}' \
  -H "CF-Connecting-IP: 203.0.113.27" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "404" "${HTTP_STATUS}" "unknown tool should return 404"
assert_eq "-32601" "$(json_get '.error.code')" "expected -32601 method not found"

echo "[mcp] tools/call invalid params payload (expected invalid params)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":123,"arguments":{}}}' \
  -H "CF-Connecting-IP: 203.0.113.28" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "400" "${HTTP_STATUS}" "invalid params should return 400"
assert_eq "-32602" "$(json_get '.error.code')" "expected -32602 invalid params"
assert_contains "$(json_get '.error.message')" "Invalid tools/call params" "expected invalid params message"

echo "[mcp] tools/call search_contract with too-long query (expected invalid params)"
long_query="$(printf 'a%.0s' {1..201})"
long_query_payload="$(jq -nc --arg q "${long_query}" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":$q,"limit":1}}}')"
http_post_json "${BASE_URL}/" "${long_query_payload}" -H "CF-Connecting-IP: 203.0.113.29" -H "x-api-key: ${TEST_PRO_KEY}"
assert_eq "400" "${HTTP_STATUS}" "too-long query should return 400"
assert_eq "-32602" "$(json_get '.error.code')" "expected -32602 invalid params"
assert_contains "$(json_get '.error.message')" "query too long" "expected query too long message"

echo "[mcp] tools/call with zero credits (expected payment required)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_account_summary","arguments":{}}}' \
  -H "CF-Connecting-IP: 203.0.113.99" -H "x-api-key: ${TEST_FREE_ZERO_KEY}"
assert_eq "402" "${HTTP_STATUS}" "zero credits should return 402"
assert_eq "-32002" "$(json_get '.error.code')" "expected -32002 payment required"
assert_eq "${TEST_PAYMENT_ADDRESS}" "$(json_get '.error.data.payment_address')" "payment data should exist"

# ============================================================
# Phase 1 Tools Tests
# ============================================================

echo "[mcp] get_token_info"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_token_info","arguments":{"token":"VVS","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.30" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_token_info should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_pool_info"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_pool_info","arguments":{"pool":"CRO-USDC","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.31" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_pool_info should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_gas_price"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_gas_price","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.32" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_gas_price should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_token_price"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_token_price","arguments":{"tokens":["CRO","VVS"],"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.33" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_token_price should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_approval_status"
http_post_json "${BASE_URL}/" "$(jq -nc --arg addr "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_approval_status","arguments":{"address":$addr,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.34" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_approval_status should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_block_info"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_block_info","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.35" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_block_info should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] estimate_gas"
http_post_json "${BASE_URL}/" "$(jq -nc --arg from "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" --arg to "0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"estimate_gas","arguments":{"from":$from,"to":$to,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.36" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "estimate_gas should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] decode_calldata"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"decode_calldata","arguments":{"data":"0xa9059cbb0000000000000000000000001234567890123456789012345678901234567890000000000000000000000000000000000000000000000000000000000000000a","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.37" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "decode_calldata should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

# ============================================================
# Phase 2 Tools Tests
# ============================================================

echo "[mcp] get_vvs_farms"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_vvs_farms","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.40" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_vvs_farms should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_vvs_rewards"
http_post_json "${BASE_URL}/" "$(jq -nc --arg addr "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_vvs_rewards","arguments":{"address":$addr,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.41" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_vvs_rewards should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_tectonic_markets"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_tectonic_markets","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.42" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_tectonic_markets should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_tectonic_rates"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_tectonic_rates","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.43" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_tectonic_rates should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] construct_revoke_approval"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"construct_revoke_approval","arguments":{"token":"0x2D03bece6747ADC00E1a131BBA1469C15fD11e03","spender":"0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.44" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "construct_revoke_approval should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_lending_rates"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_lending_rates","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.45" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_lending_rates should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_cro_overview"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_cro_overview","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.46" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_cro_overview should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_protocol_stats"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_protocol_stats","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.47" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_protocol_stats should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_contract_info"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_contract_info","arguments":{"address":"0x145863Eb42Cf62847A6Ca784e6416C1682b1b2Ae","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.48" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_contract_info should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_health_alerts"
http_post_json "${BASE_URL}/" "$(jq -nc --arg addr "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_health_alerts","arguments":{"address":$addr,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.49" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_health_alerts should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

# ============================================================
# Placeholder Tools Tests
# ============================================================

echo "[mcp] get_portfolio_analysis (placeholder)"
http_post_json "${BASE_URL}/" "$(jq -nc --arg addr "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_portfolio_analysis","arguments":{"address":$addr,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.50" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_portfolio_analysis should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"
assert_contains "$(json_get '.result.text')" "placeholder" "expected placeholder message"

echo "[mcp] get_whale_activity (placeholder)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_whale_activity","arguments":{"simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.51" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_whale_activity should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_best_swap_route"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_best_swap_route","arguments":{"token_in":"CRO","token_out":"USDC","amount_in":"1000000000000000000","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.52" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_best_swap_route should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] resolve_cronos_id (placeholder)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"resolve_cronos_id","arguments":{"query":"test.cro","simple_mode":true}}}' \
  -H "CF-Connecting-IP: 192.0.2.53" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "resolve_cronos_id should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_token_approvals"
http_post_json "${BASE_URL}/" "$(jq -nc --arg addr "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_token_approvals","arguments":{"address":$addr,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.54" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_token_approvals should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] get_liquidation_risk"
http_post_json "${BASE_URL}/" "$(jq -nc --arg addr "0x5C7F8A570d578ED84E63fdFA7b1eE72dEae1AE23" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"get_liquidation_risk","arguments":{"address":$addr,"simple_mode":true}}}')" \
  -H "CF-Connecting-IP: 192.0.2.55" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "200" "${HTTP_STATUS}" "get_liquidation_risk should return 200"
assert_eq "null" "$(json_get '.error')" "expected no error"

echo "[mcp] OK"
