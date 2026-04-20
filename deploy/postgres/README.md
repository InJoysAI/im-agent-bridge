# PostgreSQL Runtime Logs Retention

## pg_cron setup (optional, only when image supports pg_cron)

`postgres:latest` may not include `pg_cron`. If unsupported, enabling
`shared_preload_libraries=pg_cron` will prevent PostgreSQL from starting.

1. Use a PostgreSQL image/package that ships `pg_cron`.
2. Add startup flag:
   - `shared_preload_libraries=pg_cron`
3. Restart the PostgreSQL container.
4. Run:

```bash
psql "$DATABASE_URL" -f deploy/postgres/pg-cron-setup.sql
```

This creates:
- `cleanup_runtime_logs()` procedure
- cron job `runtime-logs-cleanup` at `03:00 UTC` daily

## Local compose with bundled pg_cron

`deploy/postgres/docker-compose.yml` is configured to build the local image
from `deploy/postgres/dockerfile`, install `pg_cron`, and preload it at server
startup.

Compose variables are read from `deploy/postgres/.env` (create it from
`deploy/postgres/.env.example`).

```bash
cp deploy/postgres/.env.example deploy/postgres/.env
docker compose -f deploy/postgres/docker-compose.yml build postgres
docker compose -f deploy/postgres/docker-compose.yml up -d postgres
make db-migrate-up
```

Or bring up postgres and then migrate:

```bash
docker compose -f deploy/postgres/docker-compose.yml up -d --build postgres
make db-migrate-up
```

Check result:

```bash
make db-migrate-status
docker exec postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "SELECT extname FROM pg_extension WHERE extname='pg_cron';"
docker exec postgres psql -U "$POSTGRES_USER" -d "$POSTGRES_DB" -c "SELECT jobid, jobname, schedule, command FROM cron.job WHERE jobname='runtime-logs-cleanup';"
```

## Retention Migration Source of Truth

Runtime log retention automation now lives in Goose migration:

```text
SSoT/schema/migrations/00007_runtime_logs_retention_cron.sql
```

It applies:
- `CREATE EXTENSION IF NOT EXISTS pg_cron`
- `CREATE OR REPLACE PROCEDURE cleanup_runtime_logs()`
- `cron.schedule('runtime-logs-cleanup', '0 3 * * *', 'CALL cleanup_runtime_logs()')`

The deploy helper SQL file is now informational only:

```text
deploy/postgres/pg-cron-setup.sql
```

If needed, verify runtime settings:

```bash
docker exec postgres psql -U "$POSTGRES_USER" -d postgres -c "SHOW cron.database_name;"
docker exec postgres psql -U "$POSTGRES_USER" -d postgres -c "SHOW shared_preload_libraries;"
```

## Host cron fallback (default when pg_cron is unavailable)

Install a host-level cron task:

```bash
chmod +x deploy/postgres/cleanup-runtime-logs.sh
crontab -l > /tmp/crontab.im-agent-bridge || true
echo "0 3 * * * DATABASE_URL='postgresql://user:pass@host:5432/db' /abs/path/to/deploy/postgres/cleanup-runtime-logs.sh" >> /tmp/crontab.im-agent-bridge
crontab /tmp/crontab.im-agent-bridge
```

The fallback script deletes rows older than 14 days in batches of 1000.
