#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

ip_header="CF-Connecting-IP: 203.0.113.10"
api_key="${TEST_TOPUP_KEY}"

echo "[x402] GET /x402/quote"
http_get "${BASE_URL}/x402/quote" -H "${ip_header}"
assert_eq "200" "${HTTP_STATUS}" "quote should return 200"
assert_eq "${TEST_PAYMENT_ADDRESS}" "$(json_get '.payment_address')" "quote.payment_address mismatch"
assert_ne "null" "$(json_get '.amount_wei')" "quote.amount_wei missing"
assert_ne "null" "$(json_get '.credits')" "quote.credits missing"

echo "[x402] GET /x402/status (initial)"
http_get "${BASE_URL}/x402/status" -H "x-api-key: ${api_key}"
assert_eq "200" "${HTTP_STATUS}" "status should return 200"
assert_eq "${api_key}" "$(json_get '.api_key')" "status.api_key mismatch"
assert_eq "free" "$(json_get '.tier')" "initial tier should be free"
initial_credits="$(json_get '.credits')"

echo "[x402] POST /x402/verify (pending tx)"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_PENDING}\"}" -H "${ip_header}" -H "x-api-key: ${api_key}"
assert_eq "200" "${HTTP_STATUS}" "pending verify should return 200"
assert_eq "pending" "$(json_get '.status')" "expected pending status"

echo "[x402] POST /x402/verify (failed tx)"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_FAILED}\"}" -H "${ip_header}" -H "x-api-key: ${api_key}"
assert_eq "400" "${HTTP_STATUS}" "failed tx should return 400"
assert_eq "failed" "$(json_get '.status')" "expected failed status"
assert_eq "Transaction failed" "$(json_get '.error.message')" "expected Transaction failed error"

echo "[x402] POST /x402/verify (wrong recipient)"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_WRONG_RECIPIENT}\"}" -H "${ip_header}" -H "x-api-key: ${api_key}"
assert_eq "400" "${HTTP_STATUS}" "wrong recipient should return 400"
assert_eq "rejected" "$(json_get '.status')" "expected rejected status"
assert_eq "Transaction recipient mismatch" "$(json_get '.error.message')" "expected recipient mismatch error"

echo "[x402] POST /x402/verify (amount too low)"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_LOW_AMOUNT}\"}" -H "${ip_header}" -H "x-api-key: ${api_key}"
assert_eq "400" "${HTTP_STATUS}" "low amount should return 400"
assert_eq "rejected" "$(json_get '.status')" "expected rejected status"
assert_eq "Payment amount too low" "$(json_get '.error.message')" "expected low amount error"

echo "[x402] POST /x402/verify (valid tx)"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_VALID}\"}" -H "${ip_header}" -H "x-api-key: ${api_key}"
assert_eq "200" "${HTTP_STATUS}" "valid verify should return 200"
assert_eq "credited" "$(json_get '.status')" "expected credited status"
assert_eq "1000" "$(json_get '.credits_added')" "credits_added mismatch"
assert_eq "pro" "$(json_get '.tier')" "tier should be pro after credit"
credited_credits="$(json_get '.credits')"

echo "[x402] GET /x402/status (after credit)"
http_get "${BASE_URL}/x402/status" -H "x-api-key: ${api_key}"
assert_eq "200" "${HTTP_STATUS}" "status should return 200"
assert_eq "pro" "$(json_get '.tier')" "tier should remain pro"
assert_eq "${credited_credits}" "$(json_get '.credits')" "status credits should match verify credits"

echo "[x402] POST /x402/verify (duplicate tx)"
http_post_json "${BASE_URL}/x402/verify" "{\"tx_hash\":\"${TEST_TX_VALID}\"}" -H "${ip_header}" -H "x-api-key: ${api_key}"
assert_eq "200" "${HTTP_STATUS}" "duplicate verify should return 200"
assert_eq "already_credited" "$(json_get '.status')" "expected already_credited status"
assert_eq "0" "$(json_get '.credits_added')" "duplicate should not add credits"
assert_eq "${credited_credits}" "$(json_get '.credits')" "credits should not change on duplicate"

echo "[x402] OK"

