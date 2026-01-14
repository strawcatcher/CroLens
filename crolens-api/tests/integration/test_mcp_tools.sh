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
assert_eq "6" "$(json_get '.result.tools | length')" "tools/list should return 6 tools"

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

echo "[mcp] tools/call free tier calling pro-only tool (expected payment required)"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":5}}}' \
  -H "CF-Connecting-IP: 203.0.113.24" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "402" "${HTTP_STATUS}" "free tier should return 402 for pro-only tool"
assert_eq "-32002" "$(json_get '.error.code')" "expected -32002 payment required"
assert_eq "${TEST_PAYMENT_ADDRESS}" "$(json_get '.error.data.payment_address')" "payment_address missing/mismatch"
assert_eq "25" "$(json_get '.error.data.chain_id')" "expected chain_id 25"
assert_ne "null" "$(json_get '.error.data.price')" "payment price missing"
assert_ne "null" "$(json_get '.error.data.credits')" "payment credits missing"

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

echo "[mcp] OK"
