#!/usr/bin/env bash
set -euo pipefail

INTEGRATION_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
API_DIR="$(cd "${INTEGRATION_DIR}/../.." && pwd)"

# shellcheck source=./lib.sh
source "${INTEGRATION_DIR}/lib.sh"

load_pids

kill_pid() {
  local pid="$1"
  local name="$2"
  if [[ -z "${pid}" ]]; then
    return 0
  fi
  if ! kill -0 "${pid}" >/dev/null 2>&1; then
    return 0
  fi
  echo "[integration] Stopping ${name} (pid ${pid})"
  kill "${pid}" >/dev/null 2>&1 || true
  for _ in $(seq 1 20); do
    if ! kill -0 "${pid}" >/dev/null 2>&1; then
      return 0
    fi
    sleep 0.2
  done
  kill -9 "${pid}" >/dev/null 2>&1 || true
}

kill_pid "${WRANGLER_PID:-}" "wrangler"
kill_pid "${RPC_PID:-}" "mock rpc"

if [[ -n "${DEV_VARS_FILE:-}" && -n "${DEV_VARS_BACKUP:-}" ]]; then
  if [[ -f "${DEV_VARS_BACKUP}" ]]; then
    mv "${DEV_VARS_BACKUP}" "${DEV_VARS_FILE}"
  else
    rm -f "${DEV_VARS_FILE}"
  fi
fi

rm -f "${PIDS_FILE}"

echo "[integration] Teardown complete"

