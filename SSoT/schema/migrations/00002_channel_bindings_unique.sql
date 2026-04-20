-- +goose Up
-- +goose StatementBegin

-- ============================================================
-- Add UNIQUE constraint on channel_bindings to enforce
-- "one source → exactly one bot_id" invariant.
-- Without this, the same (platform, bridge_gateway_name, bridge_channel_name)
-- could match multiple rows with different bot_ids, making bot_id resolution
-- non-deterministic.
-- COALESCE(bridge_channel_name, '') aligns with idx_channel_bindings_lookup
-- degraded matching semantics.
--
-- Aligns with: domain/domain_model.md ChannelBinding §唯一约束
--              architecture/api_strategy.md §1.6 bot_id 解析逻辑
-- ============================================================

CREATE UNIQUE INDEX uq_channel_bindings_source
    ON channel_bindings (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''));

-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin

DROP INDEX IF EXISTS uq_channel_bindings_source;

-- +goose StatementEnd
