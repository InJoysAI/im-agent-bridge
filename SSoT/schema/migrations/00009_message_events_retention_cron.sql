-- +goose Up
-- +goose StatementBegin

CREATE OR REPLACE PROCEDURE cleanup_message_events()
LANGUAGE plpgsql
AS $$
DECLARE
  deleted_count INTEGER;
BEGIN
  LOOP
    DELETE FROM message_events
    WHERE id IN (
      SELECT id
      FROM message_events
      WHERE created_at < NOW() - INTERVAL '30 days'
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
  BEGIN
    PERFORM cron.unschedule('message-events-cleanup');
  EXCEPTION
    WHEN OTHERS THEN
      NULL;
  END;

  PERFORM cron.schedule(
    'message-events-cleanup',
    '30 3 * * *',
    'CALL cleanup_message_events()'
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
    WHERE jobname = 'message-events-cleanup'
  ) THEN
    PERFORM cron.unschedule('message-events-cleanup');
  END IF;
END;
$$;

DROP PROCEDURE IF EXISTS cleanup_message_events();

-- +goose StatementEnd
