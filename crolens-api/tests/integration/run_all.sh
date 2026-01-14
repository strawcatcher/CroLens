#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

cleanup() {
  "${INTEGRATION_DIR}/teardown.sh" || true
}

trap cleanup EXIT

"${INTEGRATION_DIR}/setup.sh"

echo "[integration] Running test_health.sh"
"${INTEGRATION_DIR}/test_health.sh"

echo "[integration] Running test_security_headers.sh"
"${INTEGRATION_DIR}/test_security_headers.sh"

echo "[integration] Running test_auth.sh"
"${INTEGRATION_DIR}/test_auth.sh"

echo "[integration] Running test_mcp_tools.sh"
"${INTEGRATION_DIR}/test_mcp_tools.sh"

echo "[integration] Running test_x402_flow.sh"
"${INTEGRATION_DIR}/test_x402_flow.sh"

echo "[integration] Running test_rate_limit.sh"
"${INTEGRATION_DIR}/test_rate_limit.sh"

echo "[integration] All integration tests passed"
