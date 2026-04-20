-- +goose Up
-- +goose StatementBegin

-- ============================================================
-- Add runtime_model column to bots table
--
-- NanoBot's OpenAI-compatible /v1/chat/completions endpoint
-- requires a `model` field in the request body.  This value
-- identifies the agent/model name to use (e.g. "nanobot").
-- Storing it per-bot allows future runtime types or multi-agent
-- deployments to configure different model identifiers without
-- code changes.
--
-- Default 'nanobot' covers all existing rows (MVP stage,
-- NanoBot is the only supported runtime type).
--
-- Aligns with: openspec/proposal-roadmap.md feat-runtime-nanobot-adapter
--              SSoT/api/main.tsp RuntimeProcessRequest (session_id/bot_id fields)
-- Note: .context/domain/domain_model.md and .context/db/schema_design.md will be
--       updated to reflect this new field after implementation is complete.
-- ============================================================

ALTER TABLE bots ADD COLUMN runtime_model TEXT NOT NULL DEFAULT 'nanobot';

-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin

ALTER TABLE bots DROP COLUMN IF EXISTS runtime_model;

-- +goose StatementEnd
