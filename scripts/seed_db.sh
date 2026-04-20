#!/usr/bin/env bash
set -euo pipefail

DATABASE_URL_INPUT="${1:-${DATABASE_URL:-}}"
if [ -z "${DATABASE_URL_INPUT}" ]; then
  echo "usage: DATABASE_URL=postgres://... bash scripts/seed_db.sh"
  echo "   or: bash scripts/seed_db.sh postgres://..."
  exit 1
fi

DEFAULT_BOT_ID="11111111-1111-4111-8111-111111111111"
DEFAULT_BINDING_ID="22222222-2222-4222-8222-222222222222"

psql "${DATABASE_URL_INPUT}" <<'SQL'
INSERT INTO bots (id, bot_name, name, runtime_type, runtime_endpoint, is_enabled)
VALUES (
  '11111111-1111-4111-8111-111111111111',
  'default-bot',
  'Default Bot',
  'nanobot',
  'http://nanobot:9000/runtime',
  TRUE
)
ON CONFLICT (id) DO NOTHING;

INSERT INTO channel_bindings (
  id,
  bot_id,
  platform,
  bridge_gateway_name,
  bridge_channel_name,
  is_enabled
)
VALUES (
  '22222222-2222-4222-8222-222222222222',
  '11111111-1111-4111-8111-111111111111',
  'telegram',
  'default',
  NULL,
  TRUE
)
ON CONFLICT DO NOTHING;
SQL

echo "seed done: bot_id=${DEFAULT_BOT_ID}, binding_id=${DEFAULT_BINDING_ID}"
