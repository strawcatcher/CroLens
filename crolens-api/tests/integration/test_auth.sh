#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

new_key="cl_sk_test_new_${RANDOM}${RANDOM}"

echo "[auth] Missing x-api-key should return 400"
http_get "${BASE_URL}/x402/status"
assert_eq "400" "${HTTP_STATUS}" "status without key should return 400"
assert_eq "Missing x-api-key" "$(json_get '.error.message')" "expected missing key message"

echo "[auth] /x402/verify missing x-api-key should return 400"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_PENDING}\"}" -H "CF-Connecting-IP: 203.0.113.51"
assert_eq "400" "${HTTP_STATUS}" "verify without key should return 400"
assert_eq "Missing x-api-key" "$(json_get '.error.message')" "expected missing key message"

echo "[auth] New API key should be created on first use"
http_get "${BASE_URL}/x402/status" -H "x-api-key: ${new_key}"
assert_eq "200" "${HTTP_STATUS}" "status should return 200"
assert_eq "${new_key}" "$(json_get '.api_key')" "api_key mismatch"
assert_eq "free" "$(json_get '.tier')" "new key tier should be free"
assert_eq "50" "$(json_get '.credits')" "new key credits should default to 50"

echo "[auth] Existing API key should be stable"
http_get "${BASE_URL}/x402/status" -H "x-api-key: ${new_key}"
assert_eq "200" "${HTTP_STATUS}" "status should return 200"
assert_eq "${new_key}" "$(json_get '.api_key')" "api_key mismatch"

echo "[auth] OK"
