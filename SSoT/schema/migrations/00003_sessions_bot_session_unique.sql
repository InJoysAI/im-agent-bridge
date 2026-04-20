-- +goose Up
-- +goose StatementBegin

-- ============================================================
-- Fix sessions unique constraint for multi-Bot isolation (BR-032)
--
-- Problem: sessions.session_id was UNIQUE globally (00001_init.sql).
-- With multiple Bots sharing one PostgreSQL instance (BR-032), two
-- different Bots serving the same chat_id produce the same
-- session_id (e.g. "telegram:private:12345") and collide on the
-- single-column UNIQUE constraint.
--
-- Fix: drop the single-column UNIQUE and replace with a composite
-- UNIQUE on (bot_id, session_id).  The upsert conflict target
-- becomes ON CONFLICT (bot_id, session_id).
--
-- Aligns with: domain/business_rules.md BR-032
--              openspec/changes/feat-gateway-channel-session/proposal.md
-- ============================================================

ALTER TABLE sessions DROP CONSTRAINT IF EXISTS sessions_session_id_key;

CREATE UNIQUE INDEX uq_sessions_bot_session
    ON sessions (bot_id, session_id);

-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin

DROP INDEX IF EXISTS uq_sessions_bot_session;

ALTER TABLE sessions ADD CONSTRAINT sessions_session_id_key UNIQUE (session_id);

-- +goose StatementEnd
