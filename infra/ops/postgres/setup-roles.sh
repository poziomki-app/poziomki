#!/bin/sh
# One-time bootstrap: create least-privilege Postgres roles for the API and worker.
#
# Run this once per environment. It's idempotent, but re-running rotates the
# role passwords to whatever is currently in the env vars below.
#
# Required env vars:
#   POSTGRES_API_PASSWORD      Password for the poziomki_api role.
#   POSTGRES_WORKER_PASSWORD   Password for the poziomki_worker role.
#
# Optional (all default to values already used by docker-compose.prod.yml):
#   POSTGRES_CONTAINER  Container name (default: poziomki-rs-postgres-1).
#   POSTGRES_USER       Owner role to connect as (default: poziomki).
#   POSTGRES_DB         Database to bootstrap in (default: poziomki-rs).
#
# Example (local):
#   POSTGRES_API_PASSWORD=... POSTGRES_WORKER_PASSWORD=... \
#     ./infra/ops/postgres/setup-roles.sh
#
# Example (prod, via SSH):
#   ssh poziomki 'cd /home/ubuntu/poziomki-rs && \
#     POSTGRES_API_PASSWORD=... POSTGRES_WORKER_PASSWORD=... \
#     ./infra/ops/postgres/setup-roles.sh'

set -eu

: "${POSTGRES_API_PASSWORD:?Set POSTGRES_API_PASSWORD}"
: "${POSTGRES_WORKER_PASSWORD:?Set POSTGRES_WORKER_PASSWORD}"

CONTAINER="${POSTGRES_CONTAINER:-poziomki-rs-postgres-1}"
OWNER_USER="${POSTGRES_USER:-poziomki}"
DB_NAME="${POSTGRES_DB:-poziomki-rs}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SQL_FILE="${SCRIPT_DIR}/setup-roles.sql"

if [ ! -f "${SQL_FILE}" ]; then
  echo "Missing ${SQL_FILE}" >&2
  exit 1
fi

echo "Bootstrapping roles in ${CONTAINER} / ${DB_NAME} as ${OWNER_USER}..."

docker exec -i \
  -e PGPASSWORD \
  "${CONTAINER}" \
  psql -v ON_ERROR_STOP=1 \
       -v "api_password=${POSTGRES_API_PASSWORD}" \
       -v "worker_password=${POSTGRES_WORKER_PASSWORD}" \
       -U "${OWNER_USER}" \
       -d "${DB_NAME}" \
  < "${SQL_FILE}"

echo "Done. Verify with:"
echo "  docker exec ${CONTAINER} psql -U ${OWNER_USER} -d ${DB_NAME} -c '\\du'"
