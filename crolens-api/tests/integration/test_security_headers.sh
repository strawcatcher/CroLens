#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

expect_security_headers() {
  local headers_lc
  headers_lc="$(echo "${HTTP_HEADERS}" | tr '[:upper:]' '[:lower:]')"

  assert_contains "${headers_lc}" "x-content-type-options: nosniff" "missing X-Content-Type-Options"
  assert_contains "${headers_lc}" "x-frame-options: deny" "missing X-Frame-Options"
  assert_contains "${headers_lc}" "x-xss-protection: 1; mode=block" "missing X-XSS-Protection"
  assert_contains "${headers_lc}" "strict-transport-security: max-age=31536000; includesubdomains" "missing HSTS"
  assert_contains "${headers_lc}" "content-security-policy: default-src 'none'; frame-ancestors 'none'" "missing CSP"
}

echo "[security] GET /health includes security headers"
http_get "${BASE_URL}/health"
assert_eq "200" "${HTTP_STATUS}" "health should return 200"
expect_security_headers

echo "[security] POST / (JSON-RPC tools/list) includes security headers"
http_post_json "${BASE_URL}/" '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' -H "CF-Connecting-IP: 203.0.113.10"
assert_eq "200" "${HTTP_STATUS}" "tools/list should return 200"
expect_security_headers

echo "[security] OK"

