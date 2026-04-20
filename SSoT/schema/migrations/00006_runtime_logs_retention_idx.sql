-- +goose NO TRANSACTION

-- +goose Up
CREATE INDEX CONCURRENTLY idx_runtime_logs_created_at ON runtime_logs (created_at);

-- +goose Down
DROP INDEX IF EXISTS idx_runtime_logs_created_at;
