#!/usr/bin/env bash

set -euo pipefail

if [[ -z "${DATABASE_URL:-}" ]]; then
  echo "DATABASE_URL is required"
  exit 1
fi

# Batched deletion keeps lock scope small and avoids long write stalls.
while true; do
  deleted_count="$(
    psql "${DATABASE_URL}" -t -A -v ON_ERROR_STOP=1 -c "
      WITH to_delete AS (
        SELECT id
        FROM runtime_logs
        WHERE created_at < NOW() - INTERVAL '14 days'
        ORDER BY created_at
        LIMIT 1000
      )
      DELETE FROM runtime_logs r
      USING to_delete d
      WHERE r.id = d.id
      RETURNING 1;
    " | wc -l | tr -d ' '
  )"

  if [[ "${deleted_count}" == "0" ]]; then
    break
  fi
done
