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
  prod)    COMPOSE_FILE="docker-compose.prod.yml"    ; PROJECT="poziomki-rs"         ; ENV_FILE=".env"         ;;
  staging) COMPOSE_FILE="docker-compose.staging.yml" ; PROJECT="poziomki-rs-staging" ; ENV_FILE=".env.staging" ;;
  *) echo "unknown env: $ENV" >&2; exit 1 ;;
esac

[[ -f "$ENV_FILE" ]] || { echo "missing $ENV_FILE" >&2; exit 1; }

# Snapshot current digest for rollback before any mutation.
cp "$ENV_FILE" "${ENV_FILE}.prev"

# Update BACKEND_DIGEST line atomically.
awk -v d="$DIGEST" '/^BACKEND_DIGEST=/{print "BACKEND_DIGEST="d; found=1; next} {print} END{if(!found) print "BACKEND_DIGEST="d}' \
  "$ENV_FILE" > "${ENV_FILE}.new"
mv "${ENV_FILE}.new" "$ENV_FILE"

# Render garage.toml if prod (staging uses its own; extend when staging garage lands).
if [[ "$ENV" == "prod" ]]; then
  ./scripts/render-garage-toml.sh
fi

docker compose -p "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" pull api worker

# Run migrations in a one-off container on the compose project's network
# using the api service definition. This gives the migrator the full
# composed DATABASE_URL + pgdog DNS that the service depends on, which a
# plain `docker run` on the default bridge can't see. --no-deps avoids
# starting postgres/pgdog as side effects (they're already running).
docker compose -p "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" \
  run --rm --no-deps --entrypoint /app/poziomki_backend-cli api migrate

docker compose -p "$PROJECT" --env-file "$ENV_FILE" -f "$COMPOSE_FILE" up -d --no-build

# Append to deploy history for audit.
printf '%s\t%s\t%s\n' "$(date -Is)" "$ENV" "$DIGEST" >> .deploy-history
