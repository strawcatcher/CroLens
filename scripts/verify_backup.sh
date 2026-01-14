#!/usr/bin/env bash
set -euo pipefail

FILE="${1:-}"
if [[ -z "${FILE}" ]]; then
  echo "Usage: $(basename "$0") path/to/backup.sql" >&2
  exit 2
fi

if [[ ! -f "${FILE}" ]]; then
  echo "Missing file: ${FILE}" >&2
  exit 2
fi

if [[ ! -s "${FILE}" ]]; then
  echo "Empty backup file: ${FILE}" >&2
  exit 1
fi

missing=0
for needle in "CREATE TABLE" "request_logs" "api_keys" "tokens"; do
  if ! grep -q "${needle}" "${FILE}"; then
    echo "[verify] missing marker: ${needle}" >&2
    missing=1
  fi
done

if [[ "${missing}" -eq 1 ]]; then
  echo "[verify] FAILED: backup sanity check failed" >&2
  exit 1
fi

echo "[verify] OK"

