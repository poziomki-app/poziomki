#!/usr/bin/env bash
# Render the Garage config file from its template using values in the
# chosen env file. The rendered file is gitignored — it contains live
# secrets.
#
# Usage:
#   ./scripts/render-garage-toml.sh prod     → garage/garage.toml
#   ./scripts/render-garage-toml.sh staging  → garage/garage-staging.toml
set -euo pipefail

cd "$(dirname "$0")/.."

ENV="${1:-prod}"

case "$ENV" in
  prod)
    ENV_FILE=".env"
    TEMPLATE="garage/garage.toml.tpl"
    OUTPUT="garage/garage.toml"
    ;;
  staging)
    ENV_FILE=".env.staging"
    TEMPLATE="garage/garage-staging.toml.tpl"
    OUTPUT="garage/garage-staging.toml"
    ;;
  *)
    echo "error: unknown env '$ENV' (expected: prod|staging)" >&2
    exit 1
    ;;
esac

if [[ ! -f "$ENV_FILE" ]]; then
  echo "error: $ENV_FILE not found." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

: "${GARAGE_RPC_SECRET:?GARAGE_RPC_SECRET must be set in $ENV_FILE}"
: "${GARAGE_ADMIN_TOKEN:?GARAGE_ADMIN_TOKEN must be set in $ENV_FILE}"

envsubst '${GARAGE_RPC_SECRET} ${GARAGE_ADMIN_TOKEN}' \
  < "$TEMPLATE" \
  > "$OUTPUT"

chmod 600 "$OUTPUT"
echo "rendered $OUTPUT"
