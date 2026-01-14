#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

jsonrpc_limit="${CROLENS_TEST_JSONRPC_LIMIT:-5}"
jsonrpc_window_secs="${CROLENS_TEST_JSONRPC_WINDOW_SECS:-60}"

echo "[rate] JSON-RPC per-IP limit (limit=${jsonrpc_limit}, window=${jsonrpc_window_secs}s)"
ip_rl="CF-Connecting-IP: 203.0.113.30"

for _ in $(seq 1 "${jsonrpc_limit}"); do
  http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' -H "${ip_rl}"
  assert_eq "200" "${HTTP_STATUS}" "request should be allowed before limit"
done

http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' -H "${ip_rl}"
assert_eq "429" "${HTTP_STATUS}" "request should be rate limited"
assert_contains "${HTTP_HEADERS}" "Retry-After: ${jsonrpc_window_secs}" "Retry-After header missing"
assert_eq "-32003" "$(json_get '.error.code')" "expected -32003 rate limit"
assert_eq "${jsonrpc_window_secs}" "$(json_get '.error.data.retry_after')" "expected retry_after in json-rpc error data"

sleep "$((jsonrpc_window_secs + 1))"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' -H "${ip_rl}"
assert_eq "200" "${HTTP_STATUS}" "request should be allowed after window reset"

echo "[rate] tools/call per-api-key limit (free tier: 50/hour)"
for i in $(seq 1 50); do
  ip="CF-Connecting-IP: 10.0.0.${i}"
  http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":1}}}' \
    -H "${ip}" -H "x-api-key: ${TEST_FREE_RL_KEY}"
  assert_eq "402" "${HTTP_STATUS}" "expected payment required before tool rate limit"
  assert_eq "-32002" "$(json_get '.error.code')" "expected -32002 payment required"
done

http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":1}}}' \
  -H "CF-Connecting-IP: 10.0.1.51" -H "x-api-key: ${TEST_FREE_RL_KEY}"
assert_eq "429" "${HTTP_STATUS}" "expected rate limit on 51st tools/call"
assert_eq "-32003" "$(json_get '.error.code')" "expected -32003 rate limit"
assert_contains "${HTTP_HEADERS}" "Retry-After: 3600" "expected Retry-After: 3600 for tools/call"
assert_eq "3600" "$(json_get '.error.data.retry_after')" "expected retry_after=3600 in json-rpc error data"

echo "[rate] tools/call pro tier should not hit per-key limit at 51 requests"
for i in $(seq 1 51); do
  ip="CF-Connecting-IP: 10.0.2.${i}"
  http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"search_contract","arguments":{"query":"VVS","limit":1}}}' \
    -H "${ip}" -H "x-api-key: ${TEST_PRO_KEY}"
  assert_eq "200" "${HTTP_STATUS}" "pro tier request should be allowed"
  assert_eq "null" "$(json_get '.error')" "expected no json-rpc error"
done

echo "[rate] /x402/quote per-IP limit (30/min)"
ip_quote="CF-Connecting-IP: 203.0.113.40"
for _ in $(seq 1 30); do
  http_get "${BASE_URL}/x402/quote" -H "${ip_quote}"
  assert_eq "200" "${HTTP_STATUS}" "quote should be allowed before limit"
done
http_get "${BASE_URL}/x402/quote" -H "${ip_quote}"
assert_eq "429" "${HTTP_STATUS}" "quote should be rate limited"
assert_contains "${HTTP_HEADERS}" "Retry-After: 60" "expected Retry-After: 60 for /x402/quote"

echo "[rate] /x402/verify per-IP limit (10/min)"
ip_verify="CF-Connecting-IP: 203.0.113.41"
for _ in $(seq 1 10); do
  http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_PENDING}\"}" -H "${ip_verify}" -H "x-api-key: ${TEST_FREE_KEY}"
  assert_eq "200" "${HTTP_STATUS}" "verify should be allowed before limit"
done
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_PENDING}\"}" -H "${ip_verify}" -H "x-api-key: ${TEST_FREE_KEY}"
assert_eq "429" "${HTTP_STATUS}" "verify should be rate limited"
assert_contains "${HTTP_HEADERS}" "Retry-After: 60" "expected Retry-After: 60 for /x402/verify"

echo "[rate] OK"
