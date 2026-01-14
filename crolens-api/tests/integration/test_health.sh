#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

echo "[health] GET /health (expected ok)"
http_get "${BASE_URL}/health"
assert_eq "200" "${HTTP_STATUS}" "health should return 200"
assert_eq "ok" "$(json_get '.status')" "health.status should be ok"
assert_eq "ok" "$(json_get '.checks.db.status')" "health.checks.db.status should be ok"
assert_eq "ok" "$(json_get '.checks.kv.status')" "health.checks.kv.status should be ok"
assert_eq "ok" "$(json_get '.checks.rpc.status')" "health.checks.rpc.status should be ok"

echo "[health] Simulate RPC down"
http_post_json "${MOCK_RPC_URL}/__mode" '{"mode":"down"}'
assert_eq "200" "${HTTP_STATUS}" "mock rpc control should return 200"

echo "[health] GET /health (expected degraded)"
http_get "${BASE_URL}/health"
assert_eq "503" "${HTTP_STATUS}" "health should return 503 when degraded"
assert_eq "degraded" "$(json_get '.status')" "health.status should be degraded"
assert_eq "ok" "$(json_get '.checks.db.status')" "db should remain ok"
assert_eq "ok" "$(json_get '.checks.kv.status')" "kv should remain ok"
assert_eq "error" "$(json_get '.checks.rpc.status')" "rpc should be error"

echo "[health] Restore RPC"
http_post_json "${MOCK_RPC_URL}/__mode" '{"mode":"up"}'
assert_eq "200" "${HTTP_STATUS}" "mock rpc control should return 200"

echo "[health] GET /health (expected ok again)"
http_get "${BASE_URL}/health"
assert_eq "200" "${HTTP_STATUS}" "health should recover to 200"
assert_eq "ok" "$(json_get '.status')" "health.status should be ok"

echo "[health] Simulate DB binding missing"
"${INTEGRATION_DIR}/teardown.sh"
CROLENS_TEST_WRANGLER_CONFIG="wrangler.integration.no-db.toml" CROLENS_TEST_SKIP_D1_INIT=1 "${INTEGRATION_DIR}/setup.sh"
load_pids

echo "[health] GET /health (expected unhealthy)"
http_get "${BASE_URL}/health"
assert_eq "503" "${HTTP_STATUS}" "health should return 503 when DB is unhealthy"
assert_eq "unhealthy" "$(json_get '.status')" "health.status should be unhealthy"
assert_eq "error" "$(json_get '.checks.db.status')" "db should be error"
assert_eq "ok" "$(json_get '.checks.kv.status')" "kv should remain ok"
assert_eq "ok" "$(json_get '.checks.rpc.status')" "rpc should remain ok"

echo "[health] Restore normal config"
"${INTEGRATION_DIR}/teardown.sh"
"${INTEGRATION_DIR}/setup.sh"
load_pids

echo "[health] OK"
