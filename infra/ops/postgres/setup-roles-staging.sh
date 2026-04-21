#!/bin/sh
# One-time bootstrap for the staging slim parallel stack: creates the staging
# DB and the owner / api / worker roles on the shared prod Postgres cluster.
#
# Idempotent; re-running rotates passwords to whatever is in the env vars.
# After roles exist, installs extensions the consolidated migration needs
# (cube, earthdistance, pg_trgm) because the staging owner is intentionally
# NOT a superuser and can't CREATE EXTENSION itself.
#
# Required env vars:
#   POSTGRES_STAGING_OWNER_PASSWORD   Password for poziomki_staging (DB owner).
#   POSTGRES_STAGING_API_PASSWORD     Password for poziomki_staging_api.
#   POSTGRES_STAGING_WORKER_PASSWORD  Password for poziomki_staging_worker.
#
# Optional:
#   POSTGRES_CONTAINER  (default: poziomki-rs-postgres-1)
#   CLUSTER_SUPERUSER   (default: poziomki — the prod superuser)
#
# Example (prod VPS):
#   ssh poziomki 'cd ~/poziomki-rs-staging && \
#     POSTGRES_STAGING_OWNER_PASSWORD=... \
#     POSTGRES_STAGING_API_PASSWORD=... \
#     POSTGRES_STAGING_WORKER_PASSWORD=... \
#     ./infra/ops/postgres/setup-roles-staging.sh'

set -eu

: "${POSTGRES_STAGING_OWNER_PASSWORD:?Set POSTGRES_STAGING_OWNER_PASSWORD}"
: "${POSTGRES_STAGING_API_PASSWORD:?Set POSTGRES_STAGING_API_PASSWORD}"
: "${POSTGRES_STAGING_WORKER_PASSWORD:?Set POSTGRES_STAGING_WORKER_PASSWORD}"

CONTAINER="${POSTGRES_CONTAINER:-poziomki-rs-postgres-1}"
SUPERUSER="${CLUSTER_SUPERUSER:-poziomki}"

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SQL_FILE="${SCRIPT_DIR}/setup-roles-staging.sql"

if [ ! -f "${SQL_FILE}" ]; then
  echo "Missing ${SQL_FILE}" >&2
  exit 1
fi

echo "Bootstrapping staging roles + DB in ${CONTAINER} as ${SUPERUSER}..."

docker exec -i "${CONTAINER}" \
  psql -v ON_ERROR_STOP=1 \
       -v "owner_password=${POSTGRES_STAGING_OWNER_PASSWORD}" \
       -v "api_password=${POSTGRES_STAGING_API_PASSWORD}" \
       -v "worker_password=${POSTGRES_STAGING_WORKER_PASSWORD}" \
       -U "${SUPERUSER}" \
       -d postgres \
  < "${SQL_FILE}"

# Extensions are required by the consolidated migration and must be created
# as a superuser — the staging DB owner deliberately isn't one.
echo "Installing extensions into poziomki_staging..."
docker exec -i "${CONTAINER}" \
  psql -v ON_ERROR_STOP=1 -U "${SUPERUSER}" -d poziomki_staging <<'SQL'
CREATE EXTENSION IF NOT EXISTS cube;
CREATE EXTENSION IF NOT EXISTS earthdistance;
CREATE EXTENSION IF NOT EXISTS pg_trgm;
SQL

echo "Done. Verify with:"
echo "  docker exec ${CONTAINER} psql -U ${SUPERUSER} -d postgres -c '\\du poziomki_staging*'"
echo "  docker exec ${CONTAINER} psql -U ${SUPERUSER} -d poziomki_staging -c '\\dx'"
