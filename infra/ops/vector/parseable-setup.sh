#!/usr/bin/env bash
# Run on the server after deploying Vector + structured logging backend.
# Requires: PARSEABLE_PASSWORD env var set, Parseable running on localhost:8100.
set -euo pipefail

PASS="${PARSEABLE_PASSWORD:?Set PARSEABLE_PASSWORD}"
BASE="http://localhost:8100"

NETRC="$(mktemp)"
trap 'rm -f "$NETRC"' EXIT
chmod 600 "$NETRC"
printf 'machine localhost\nlogin admin\npassword %s\n' "$PASS" > "$NETRC"

echo "==> Deleting old docker stream..."
curl -sS --netrc-file "$NETRC" -X DELETE "${BASE}/api/v1/logstream/docker" || true

echo "==> Setting 30-day retention on streams..."
for stream in api worker infra; do
  curl -sSf --netrc-file "$NETRC" -X PUT \
    "${BASE}/api/v1/logstream/${stream}/retention" \
    -H 'Content-Type: application/json' \
    -d '[{"description":"30 day retention","action":"delete","duration":"30d"}]'
  echo ""
done

echo "==> Creating API error alert -> ntfy..."
curl -sSf --netrc-file "$NETRC" -X PUT \
  "${BASE}/api/v1/logstream/api/alert" \
  -H 'Content-Type: application/json' \
  -d '{
    "version": "v1",
    "alerts": [{
      "name": "api-errors",
      "message": "API error spike detected — check the api stream",
      "rule": {
        "type": "column",
        "config": {
          "column": "level",
          "operator": "=",
          "value": "ERROR",
          "repeats": 3
        }
      },
      "targets": [{
        "type": "webhook",
        "endpoint": "https://ntfy.poziomki.app/poziomki-ops",
        "repeat": { "interval": "5m", "times": 3 }
      }]
    }]
  }'
echo ""

echo "==> Creating worker outbox failure alert -> ntfy..."
curl -sSf --netrc-file "$NETRC" -X PUT \
  "${BASE}/api/v1/logstream/worker/alert" \
  -H 'Content-Type: application/json' \
  -d '{
    "version": "v1",
    "alerts": [{
      "name": "outbox-failures",
      "message": "Outbox job failure detected",
      "rule": {
        "type": "column",
        "config": {
          "column": "level",
          "operator": "=",
          "value": "ERROR",
          "repeats": 1
        }
      },
      "targets": [{
        "type": "webhook",
        "endpoint": "https://ntfy.poziomki.app/poziomki-ops",
        "repeat": { "interval": "10m", "times": 3 }
      }]
    }]
  }'
echo ""

echo "==> Done. Verify at ${BASE} (tunnel: ssh -L 8100:localhost:8100 poziomki)"
