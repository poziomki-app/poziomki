#!/bin/bash
# Run this ONCE after first Synapse startup to create the registration token.
# Usage: ./synapse/init.sh <registration_token> [synapse_container_name]
#
# The registration_token should match your MATRIX_REGISTRATION_TOKEN env var.

set -euo pipefail

TOKEN="${1:?Usage: $0 <registration_token> [container_name]}"
CONTAINER="${2:-poziomki-rs-synapse-1}"

# Get the registration_shared_secret from the container
SHARED_SECRET=$(docker exec "$CONTAINER" cat /data/registration_shared_secret 2>/dev/null || true)

if [ -z "$SHARED_SECRET" ]; then
    echo "Error: Could not read registration_shared_secret from container."
    echo "Make sure Synapse has started at least once."
    exit 1
fi

# Get an admin nonce
NONCE=$(docker exec "$CONTAINER" curl -s http://localhost:8008/_synapse/admin/v1/register | python3 -c "import sys,json; print(json.load(sys.stdin)['nonce'])")

# Create an admin user to use the admin API
ADMIN_USER="admin"
ADMIN_PASS="$(openssl rand -hex 32)"

# Generate HMAC for admin registration
MAC=$(echo -n "${NONCE}\x00${ADMIN_USER}\x00${ADMIN_PASS}\x00admin" | openssl dgst -sha1 -hmac "$SHARED_SECRET" | awk '{print $2}')

# Register admin user
docker exec "$CONTAINER" curl -s -X POST http://localhost:8008/_synapse/admin/v1/register \
    -H "Content-Type: application/json" \
    -d "{\"nonce\":\"$NONCE\",\"username\":\"$ADMIN_USER\",\"password\":\"$ADMIN_PASS\",\"admin\":true,\"mac\":\"$MAC\"}" > /tmp/synapse_admin.json

ADMIN_TOKEN=$(python3 -c "import json; print(json.load(open('/tmp/synapse_admin.json'))['access_token'])")
rm -f /tmp/synapse_admin.json

echo "Admin user created. Creating registration token..."

# Create the registration token (no expiry, unlimited uses)
RESULT=$(docker exec "$CONTAINER" curl -s -X POST http://localhost:8008/_synapse/admin/v1/registration_tokens/new \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $ADMIN_TOKEN" \
    -d "{\"token\":\"$TOKEN\",\"uses_allowed\":null,\"expiry_time\":null}")

echo "Registration token created:"
echo "$RESULT" | python3 -m json.tool 2>/dev/null || echo "$RESULT"
echo ""
echo "Done! Synapse is ready. The backend can now register users with this token."
