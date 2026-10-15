#!/usr/bin/env bash
# Dev-only reset: wipes business data and recreates the plugin module and its
# single server with fresh credentials. Prints the credentials to put in the
# plugin's management_config.json.
#
# Usage: bash scripts/reset_dev.sh
set -euo pipefail

API="${API_CORE_URL:-http://127.0.0.1:3000}"
ADMIN_KEY="${SEED_ADMIN_API_KEY:-dev-admin-api-key}"
ADMIN_SECRET="${SEED_ADMIN_HMAC_SECRET:-dev-admin-hmac-secret}"
PG_CONTAINER="${PG_CONTAINER:-api-core-postgres}"
PG_USER="${PG_USER:-api_core}"
PG_DB="${PG_DB:-api_core}"

pg() { docker exec -i "$PG_CONTAINER" psql -U "$PG_USER" -d "$PG_DB" "$@"; }

sign() { printf '%s.%s' "$1" "$2" | openssl dgst -sha256 -hmac "$ADMIN_SECRET" -hex | awk '{print $2}'; }

api() {
  local method="$1" path="$2" body="${3:-}" ts sig
  ts=$(date +%s)
  if [ -n "$body" ]; then sig=$(sign "$ts" "$body"); else sig=$(sign "$ts" ""); fi
  curl -s -X "$method" "$API$path" \
    -H "x-api-key: $ADMIN_KEY" -H "x-timestamp: $ts" -H "x-signature: $sig" \
    -H 'content-type: application/json' ${body:+-d "$body"}
}

echo "== 1. Wipe business data and non-infrastructure modules =="
pg <<'SQL'
BEGIN;
DELETE FROM whitelist_queue;
DELETE FROM player_sessions;
DELETE FROM whitelists;
DELETE FROM hytale_servers;
DELETE FROM module_scopes WHERE module_id IN (SELECT id FROM modules WHERE name <> 'admin');
DELETE FROM modules WHERE name <> 'admin';
COMMIT;
SQL

echo "== 2. Register the plugin module =="
BODY='{"name":"plugin-hytale-dev","description":"Hytale server management plugin (local dev)","scopes":["player_sessions:write","player_sessions:read","hytale_servers:write","hytale_servers:read","whitelist:read"]}'
RESP=$(api POST /api/v1/modules "$BODY")
PLUGIN_ID=$(printf '%s' "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')
PLUGIN_KEY=$(printf '%s' "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["api_key"])')
PLUGIN_HMAC=$(printf '%s' "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["hmac_secret"])')

echo "== 3. Register the website module =="
BODY='{"name":"siteweb","description":"website & member portal","scopes":["hytale_servers:read","player_sessions:read","whitelist:read","whitelist:write"]}'
RESP=$(api POST /api/v1/modules "$BODY")
SITE_KEY=$(printf '%s' "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["api_key"])')
SITE_HMAC=$(printf '%s' "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["hmac_secret"])')

echo "== 4. Register the single server (owned by the plugin) =="
BODY="{\"module_id\":\"$PLUGIN_ID\",\"name\":\"survie\",\"url\":\"http://127.0.0.1:8080\"}"
RESP=$(api POST /api/v1/servers "$BODY")
SERVER_ID=$(printf '%s' "$RESP" | python3 -c 'import sys,json;print(json.load(sys.stdin)["id"])')

echo
echo "==============================================="
echo " PLUGIN_ID   = $PLUGIN_ID"
echo " PLUGIN_KEY  = $PLUGIN_KEY"
echo " PLUGIN_HMAC = $PLUGIN_HMAC"
echo " SERVER_ID   = $SERVER_ID"
echo " SITE_KEY    = $SITE_KEY"
echo " SITE_HMAC   = $SITE_HMAC"
echo "==============================================="
