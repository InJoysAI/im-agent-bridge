-- +goose NO TRANSACTION

-- +goose Up
CREATE INDEX CONCURRENTLY idx_message_events_created_at ON message_events (created_at);

-- +goose Down
DROP INDEX IF EXISTS idx_message_events_created_at;
