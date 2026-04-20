-- +goose Up
-- +goose StatementBegin

ALTER TABLE bots ADD COLUMN IF NOT EXISTS telegram_username TEXT NULL;
ALTER TABLE bots ADD COLUMN IF NOT EXISTS require_mention BOOLEAN NOT NULL DEFAULT FALSE;

-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin

ALTER TABLE bots DROP COLUMN IF EXISTS require_mention;
ALTER TABLE bots DROP COLUMN IF EXISTS telegram_username;

-- +goose StatementEnd
