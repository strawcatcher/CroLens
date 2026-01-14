#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
API_DIR="$(cd "${INTEGRATION_DIR}/../.." && pwd)"

PIDS_FILE="${API_DIR}/tests/integration/.pids"
STATE_DIR="${API_DIR}/.wrangler/test-state"

_base_url_env="${CROLENS_TEST_BASE_URL:-}"
_mock_rpc_url_env="${CROLENS_TEST_MOCK_RPC_URL:-}"

if [[ -f "${PIDS_FILE}" ]]; then
  # shellcheck disable=SC1090
  source "${PIDS_FILE}"
fi

BASE_URL="${_base_url_env:-${BASE_URL:-http://127.0.0.1:8787}}"
MOCK_RPC_URL="${_mock_rpc_url_env:-${MOCK_RPC_URL:-http://127.0.0.1:19000}}"

TEST_FREE_KEY="${CROLENS_TEST_FREE_KEY:-cl_sk_test_free_001}"
TEST_FREE_RL_KEY="${CROLENS_TEST_FREE_RL_KEY:-cl_sk_test_free_rl}"
TEST_FREE_ZERO_KEY="${CROLENS_TEST_FREE_ZERO_KEY:-cl_sk_test_free_zero}"
TEST_PRO_KEY="${CROLENS_TEST_PRO_KEY:-cl_sk_test_pro_001}"
TEST_TOPUP_KEY="${CROLENS_TEST_TOPUP_KEY:-cl_sk_test_free_topup}"

TEST_PAYMENT_ADDRESS="${CROLENS_TEST_PAYMENT_ADDRESS:-0x1111111111111111111111111111111111111111}"

TEST_TX_VALID="${CROLENS_TEST_TX_VALID:-0x1111111111111111111111111111111111111111111111111111111111111111}"
TEST_TX_PENDING="${CROLENS_TEST_TX_PENDING:-0x2222222222222222222222222222222222222222222222222222222222222222}"
TEST_TX_WRONG_RECIPIENT="${CROLENS_TEST_TX_WRONG_RECIPIENT:-0x3333333333333333333333333333333333333333333333333333333333333333}"
TEST_TX_LOW_AMOUNT="${CROLENS_TEST_TX_LOW_AMOUNT:-0x4444444444444444444444444444444444444444444444444444444444444444}"
TEST_TX_FAILED="${CROLENS_TEST_TX_FAILED:-0x5555555555555555555555555555555555555555555555555555555555555555}"

load_pids() {
  if [[ -f "${PIDS_FILE}" ]]; then
    # shellcheck disable=SC1090
    source "${PIDS_FILE}"
  fi
}

fail() {
  echo "FAIL: $*" >&2
  exit 1
}

assert_eq() {
  local expected="$1"
  local actual="$2"
  local msg="${3:-}"
  if [[ "${expected}" != "${actual}" ]]; then
    fail "${msg:-expected '${expected}', got '${actual}'}"
  fi
}

assert_ne() {
  local not_expected="$1"
  local actual="$2"
  local msg="${3:-}"
  if [[ "${not_expected}" == "${actual}" ]]; then
    fail "${msg:-did not expect '${not_expected}'}"
  fi
}

assert_contains() {
  local haystack="$1"
  local needle="$2"
  local msg="${3:-}"
  if [[ "${haystack}" != *"${needle}"* ]]; then
    fail "${msg:-expected output to contain '${needle}'}"
  fi
}

http_get() {
  local url="$1"
  shift
  local headers_file body_file
  headers_file="$(mktemp)"
  body_file="$(mktemp)"
  local status
  status="$(
    curl -sS -D "${headers_file}" -o "${body_file}" -w "%{http_code}" "$@" "${url}" || true
  )"
  HTTP_STATUS="${status}"
  HTTP_HEADERS="$(cat "${headers_file}")"
  HTTP_BODY="$(cat "${body_file}")"
  rm -f "${headers_file}" "${body_file}"
}

http_post_json() {
  local url="$1"
  local json="$2"
  shift 2
  local headers_file body_file
  headers_file="$(mktemp)"
  body_file="$(mktemp)"
  local status
  status="$(
    curl -sS -D "${headers_file}" -o "${body_file}" -w "%{http_code}" \
      -H "Content-Type: application/json" \
      "$@" \
      --data "${json}" \
      "${url}" || true
  )"
  HTTP_STATUS="${status}"
  HTTP_HEADERS="$(cat "${headers_file}")"
  HTTP_BODY="$(cat "${body_file}")"
  rm -f "${headers_file}" "${body_file}"
}

json_get() {
  local jq_expr="$1"
  echo "${HTTP_BODY}" | jq -r "${jq_expr}"
}
