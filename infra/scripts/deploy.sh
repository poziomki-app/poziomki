#!/usr/bin/env bash
# Deploy wrapper — renders templates, pulls pinned backend image, runs migrations,
# then `up -d --no-build`. Called by CI (deploy-prod.yml / deploy-staging.yml) but
# also works from the VPS shell for manual ops.
#
# Usage:
#   ./scripts/deploy.sh prod  ghcr.io/poziomki-app/poziomki-backend@sha256:<digest>
#   ./scripts/deploy.sh staging ghcr.io/poziomki-app/poziomki-backend@sha256:<digest>
set -euo pipefail

cd "$(dirname "$0")/.."

ENV="${1:?environment required: prod|staging}"
DIGEST="${2:?image digest required}"

case "$ENV" in
  prod)
    COMPOSE_FILE="docker-compose.prod.yml"
    PROJECT="poziomki-rs"
    ENV_FILE=".env"
    PULL_SERVICES=(api worker)
    ;;
  staging)
    COMPOSE_FILE="docker-compose.staging.yml"
    PROJECT="poziomki-rs-staging"
    ENV_FILE=".env.staging"
    PULL_SERVICES=(api_staging worker_staging)
    ;;
  *) echo "unknown env: $ENV" >&2; exit 1 ;;
esac

[[ -f "$ENV_FILE" ]] || { echo "missing $ENV_FILE" >&2; exit 1; }

# Staging imgproxy has no published image tag — the service is built on the
# host from ../imgproxy. `up -d --no-build` below would fail on first run if
# the image isn't cached, so build it lazily here. Prod imgproxy pulls a
# published tag and is unaffected.
if [[ "$ENV" == "staging" ]]; then
  if ! docker image inspect poziomki-rs-staging-imgproxy_staging >/dev/null 2>&1; then
    echo "building imgproxy_staging (first run on this host)…"
    docker compose -p "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" build imgproxy_staging
  fi
fi

# Snapshot current digest for rollback before any mutation.
cp "$ENV_FILE" "${ENV_FILE}.prev"

# Update BACKEND_DIGEST line atomically.
awk -v d="$DIGEST" '/^BACKEND_DIGEST=/{print "BACKEND_DIGEST="d; found=1; next} {print} END{if(!found) print "BACKEND_DIGEST="d}' \
  "$ENV_FILE" > "${ENV_FILE}.new"
mv "${ENV_FILE}.new" "$ENV_FILE"

# Garage config is rendered once on prod deploys; staging reuses the
# same shared Garage cluster, so nothing to render here.
if [[ "$ENV" == "prod" ]]; then
  ./scripts/render-garage-toml.sh
fi

docker compose -p "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" pull "${PULL_SERVICES[@]}"

# Migrations run on api startup (backend/src/app.rs calls run_migrations()
# before serving). There is no `migrate` subcommand in the CLI. `up -d`
# below brings up the api container, which applies any pending migration
# before binding the listener. Pre-deploy migration would need a real
# one-shot subcommand — add it if we ever want an explicit gate.
docker compose -p "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" up -d --no-build

# Append to deploy history for audit.
printf '%s\t%s\t%s\n' "$(date -Is)" "$ENV" "$DIGEST" >> .deploy-history
