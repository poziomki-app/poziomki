#!/usr/bin/env bash
# Render garage/garage.toml from garage.toml.tpl using values in .env.
# The rendered file is gitignored — it contains live secrets.
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ ! -f .env ]]; then
  echo "error: .env not found. Copy .env.example and fill in values." >&2
  exit 1
fi

set -a
# shellcheck disable=SC1091
source .env
set +a

: "${GARAGE_RPC_SECRET:?GARAGE_RPC_SECRET must be set in .env}"
: "${GARAGE_ADMIN_TOKEN:?GARAGE_ADMIN_TOKEN must be set in .env}"

envsubst '${GARAGE_RPC_SECRET} ${GARAGE_ADMIN_TOKEN}' \
  < garage/garage.toml.tpl \
  > garage/garage.toml

chmod 600 garage/garage.toml
echo "rendered garage/garage.toml"
