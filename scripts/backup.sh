#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
API_DIR="${ROOT_DIR}/crolens-api"

DB_NAME="${CROLENS_D1_NAME:-}"
BACKUP_DIR="${CROLENS_BACKUP_DIR:-${ROOT_DIR}/backups/d1}"
KEEP_DAYS="${CROLENS_BACKUP_KEEP_DAYS:-30}"
WRANGLER_CONFIG="${WRANGLER_CONFIG:-${API_DIR}/wrangler.production.toml}"

if [[ ! -f "${WRANGLER_CONFIG}" ]]; then
  WRANGLER_CONFIG="${API_DIR}/wrangler.toml"
fi

if [[ -z "${DB_NAME}" ]]; then
  DB_NAME="$(grep -E '^[[:space:]]*database_name[[:space:]]*=' "${WRANGLER_CONFIG}" | head -n 1 | sed -E 's/^[[:space:]]*database_name[[:space:]]*=[[:space:]]*\"([^\"]+)\".*/\\1/')"
fi
if [[ -z "${DB_NAME}" ]]; then
  DB_NAME="crolens-db"
fi

WRANGLER="${API_DIR}/node_modules/.bin/wrangler"
if [[ ! -x "${WRANGLER}" ]]; then
  WRANGLER="wrangler"
fi

mkdir -p "${BACKUP_DIR}"

TS="$(date -u +%Y%m%dT%H%M%SZ)"
OUT_FILE="${BACKUP_DIR}/${DB_NAME}_${TS}.sql"

echo "[backup] exporting ${DB_NAME} -> ${OUT_FILE}"
"${WRANGLER}" d1 export "${DB_NAME}" --remote --output "${OUT_FILE}" -c "${WRANGLER_CONFIG}"

if [[ ! -s "${OUT_FILE}" ]]; then
  echo "[backup] ERROR: empty export file: ${OUT_FILE}" >&2
  exit 1
fi

if grep -qE "CREATE TABLE|INSERT INTO" "${OUT_FILE}"; then
  echo "[backup] verify: basic SQL markers found"
else
  echo "[backup] WARN: export file does not contain expected SQL markers" >&2
fi

echo "[backup] retention: keeping ${KEEP_DAYS} day(s) in ${BACKUP_DIR}"
find "${BACKUP_DIR}" -type f -name "${DB_NAME}_*.sql" -mtime +"${KEEP_DAYS}" -print -delete || true

echo "[backup] done"
