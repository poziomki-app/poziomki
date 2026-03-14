#!/bin/sh
set -eu

CFG_DIR="/tmp/pgdog"
mkdir -p "${CFG_DIR}"

# ---------------------------------------------------------------------------
# Parse DATABASE_URL or use individual DB_* env vars
# ---------------------------------------------------------------------------
if [ -n "${DB_HOST:-}" ] && [ -n "${DB_NAME:-}" ] && [ -n "${DB_USER:-}" ]; then
  DB_PORT="${DB_PORT:-5432}"
  DB_PASSWORD="${DB_PASSWORD:-}"
else
  # Require DATABASE_URL
  : "${DATABASE_URL:?Missing DATABASE_URL}"
  proto="$(printf '%s' "${DATABASE_URL}" | sed -n 's#^\(.*://\).*$#\1#p')"
  trimmed="$(printf '%s' "${DATABASE_URL}" | sed "s#${proto}##")"
  userpass="$(printf '%s' "${trimmed}" | sed -n 's#^\([^@]*\)@.*$#\1#p')"
  hostdb="$(printf '%s' "${trimmed}" | sed "s#^${userpass}@##")"

  DB_USER="$(printf '%s' "${userpass}" | cut -d: -f1)"
  if printf '%s' "${userpass}" | grep -q ':'; then
    DB_PASSWORD="$(printf '%s' "${userpass}" | cut -d: -f2-)"
  else
    DB_PASSWORD=""
  fi

  hostport="$(printf '%s' "${hostdb}" | cut -d/ -f1)"
  DB_NAME="$(printf '%s' "${hostdb}" | cut -d/ -f2-)"
  DB_HOST="$(printf '%s' "${hostport}" | cut -d: -f1)"
  if printf '%s' "${hostport}" | grep -q ':'; then
    DB_PORT="$(printf '%s' "${hostport}" | cut -d: -f2)"
  else
    DB_PORT="5432"
  fi
fi

# ---------------------------------------------------------------------------
# Pool settings (with PgBouncer-compatible env var names)
# ---------------------------------------------------------------------------
POOL_SIZE="${DEFAULT_POOL_SIZE:-20}"
POOL_MODE="${POOL_MODE:-transaction}"
LISTEN_PORT="${LISTEN_PORT:-6432}"
HEALTHCHECK_PORT="${HEALTHCHECK_PORT:-6433}"

QUERY_TIMEOUT="${QUERY_TIMEOUT:-30000}"
BAN_TIMEOUT="${BAN_TIMEOUT:-30000}"
SERVER_LIFETIME="${SERVER_LIFETIME:-3600000}"
OPENMETRICS_PORT="${OPENMETRICS_PORT:-9187}"

cat > "${CFG_DIR}/pgdog.toml" <<EOF
[general]
host = "0.0.0.0"
port = ${LISTEN_PORT}
workers = 2
default_pool_size = ${POOL_SIZE}
min_pool_size = 1
pooler_mode = "${POOL_MODE}"
auth_type = "scram"
prepared_statements = "extended"

# Health & monitoring
healthcheck_port = ${HEALTHCHECK_PORT}
healthcheck_interval = 30000
openmetrics_port = ${OPENMETRICS_PORT}

# Timeouts
idle_timeout = 60000
connect_timeout = 5000
checkout_timeout = 3000
query_timeout = ${QUERY_TIMEOUT}
server_lifetime = ${SERVER_LIFETIME}
client_idle_in_transaction_timeout = 15000

# Recovery
ban_timeout = ${BAN_TIMEOUT}
connection_recovery = "recover"

# Logging
log_connections = true
log_disconnections = true

[[databases]]
name = "${DB_NAME}"
host = "${DB_HOST}"
port = ${DB_PORT}
EOF

cat > "${CFG_DIR}/users.toml" <<EOF
[[users]]
name = "${DB_USER}"
database = "${DB_NAME}"
password = "${DB_PASSWORD}"
EOF

exec /usr/local/bin/pgdog \
  --config "${CFG_DIR}/pgdog.toml" \
  --users "${CFG_DIR}/users.toml"
