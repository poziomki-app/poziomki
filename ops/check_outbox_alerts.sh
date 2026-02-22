#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="${PROJECT_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"
COMPOSE_FILE="${COMPOSE_FILE:-$PROJECT_DIR/docker-compose.prod.yml}"
ENV_FILE="${ENV_FILE:-$PROJECT_DIR/.env}"
API_URL="${API_URL:-http://127.0.0.1:5150}"
WORKER_SERVICE="${WORKER_SERVICE:-worker}"
MAX_READY_AGE_SECONDS="${OUTBOX_MAX_READY_AGE_SECONDS:-60}"

if [[ -z "${OPS_STATUS_TOKEN:-}" && -f "$ENV_FILE" ]]; then
  token_line="$(grep -E '^OPS_STATUS_TOKEN=' "$ENV_FILE" | tail -n1 || true)"
  if [[ -n "$token_line" ]]; then
    OPS_STATUS_TOKEN="${token_line#OPS_STATUS_TOKEN=}"
  fi
fi

if [[ -z "${OPS_STATUS_TOKEN:-}" ]]; then
  echo "ALERT: OPS_STATUS_TOKEN is not set (and not found in $ENV_FILE)"
  exit 2
fi

worker_cid="$(docker compose -f "$COMPOSE_FILE" ps -q "$WORKER_SERVICE" 2>/dev/null || true)"
if [[ -z "$worker_cid" ]]; then
  echo "ALERT: worker service '$WORKER_SERVICE' is not running"
  exit 2
fi

worker_health="$(docker inspect --format '{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}' "$worker_cid" 2>/dev/null || echo "unknown")"
if [[ "$worker_health" != "healthy" ]]; then
  echo "ALERT: worker health is '$worker_health'"
  exit 2
fi

json="$(curl -fsS --max-time 5 \
  -H "x-ops-token: $OPS_STATUS_TOKEN" \
  "$API_URL/api/v1/ops/outbox/status")" || {
  echo "ALERT: failed to fetch outbox status from $API_URL"
  exit 2
}

python3 - "$MAX_READY_AGE_SECONDS" <<'PY' <<<"$json"
import json
import sys

max_ready_age = int(sys.argv[1])
try:
    payload = json.loads(sys.stdin.read())
except Exception as exc:
    print(f"ALERT: invalid JSON from outbox status endpoint: {exc}")
    raise SystemExit(2)

failed = int(payload.get("failedJobs", 0) or 0)
oldest_ready = payload.get("oldestReadyJobAgeSeconds", 0)
try:
    oldest_ready = int(oldest_ready or 0)
except Exception:
    oldest_ready = 0

alerts = []
if failed > 0:
    alerts.append(f"failedJobs={failed}")
if oldest_ready > max_ready_age:
    alerts.append(f"oldestReadyJobAgeSeconds={oldest_ready}>{max_ready_age}")

if alerts:
    print("ALERT: " + ", ".join(alerts))
    raise SystemExit(2)

ready = int(payload.get("readyJobs", 0) or 0)
pending = int(payload.get("pendingJobs", 0) or 0)
retrying = int(payload.get("retryingJobs", 0) or 0)
print(
    "OK:"
    f" worker=healthy readyJobs={ready}"
    f" pendingJobs={pending}"
    f" retryingJobs={retrying}"
    f" failedJobs={failed}"
    f" oldestReadyJobAgeSeconds={oldest_ready}"
)
PY
