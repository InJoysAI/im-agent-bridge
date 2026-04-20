-- +goose Up
-- +goose StatementBegin

CREATE EXTENSION IF NOT EXISTS pg_cron;

CREATE OR REPLACE PROCEDURE cleanup_runtime_logs()
LANGUAGE plpgsql
AS $$
DECLARE
  deleted_count INTEGER;
BEGIN
  LOOP
    DELETE FROM runtime_logs
    WHERE id IN (
      SELECT id
      FROM runtime_logs
      WHERE created_at < NOW() - INTERVAL '14 days'
      ORDER BY created_at
      LIMIT 1000
    );

    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    COMMIT;
    EXIT WHEN deleted_count = 0;
  END LOOP;
END;
$$;

DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM cron.job
    WHERE jobname = 'runtime-logs-cleanup'
  ) THEN
    PERFORM cron.unschedule('runtime-logs-cleanup');
  END IF;

  PERFORM cron.schedule(
    'runtime-logs-cleanup',
    '0 3 * * *',
    'CALL cleanup_runtime_logs()'
  );
END;
$$;

-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin

DO $$
BEGIN
  IF EXISTS (
    SELECT 1
    FROM cron.job
    WHERE jobname = 'runtime-logs-cleanup'
  ) THEN
    PERFORM cron.unschedule('runtime-logs-cleanup');
  END IF;
END;
$$;

DROP PROCEDURE IF EXISTS cleanup_runtime_logs();

-- +goose StatementEnd
