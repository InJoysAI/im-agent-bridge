-- +goose Up
-- +goose StatementBegin

-- ============================================================
-- IM Agent Bridge — Initial Schema
-- Based on: TAD v1.1 §8 PostgreSQL 数据模型
-- ============================================================

-- Bot 基础配置
CREATE TABLE bots (
    id UUID PRIMARY KEY,
    bot_name VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    runtime_type VARCHAR(32) NOT NULL,
    runtime_endpoint TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Bot ↔ 渠道入口绑定
CREATE TABLE channel_bindings (
    id UUID PRIMARY KEY,
    bot_id UUID NOT NULL REFERENCES bots(id),
    platform VARCHAR(32) NOT NULL,
    bridge_gateway_name VARCHAR(128) NOT NULL,
    bridge_channel_name VARCHAR(128),
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Session 映射
CREATE TABLE sessions (
    id UUID PRIMARY KEY,
    session_id VARCHAR(256) UNIQUE NOT NULL,
    bot_id UUID NOT NULL REFERENCES bots(id),
    platform VARCHAR(32) NOT NULL,
    chat_id VARCHAR(128) NOT NULL,
    chat_type VARCHAR(32) NOT NULL,
    last_user_id VARCHAR(128),
    runtime_session_key VARCHAR(256),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 消息事件/处理状态/回写状态
CREATE TABLE message_events (
    id UUID PRIMARY KEY,
    event_id VARCHAR(128) UNIQUE NOT NULL,
    bot_id UUID NOT NULL REFERENCES bots(id),
    session_id VARCHAR(256) NOT NULL,
    platform VARCHAR(32) NOT NULL,
    bridge_gateway_name VARCHAR(128) NOT NULL,
    bridge_channel_name VARCHAR(128),
    bridge_message_id VARCHAR(128) NOT NULL,
    reply_id VARCHAR(128) UNIQUE NOT NULL,
    chat_id VARCHAR(128) NOT NULL,
    chat_type VARCHAR(32) NOT NULL,
    user_id VARCHAR(128),
    input_text TEXT,
    output_text TEXT,
    status VARCHAR(32) NOT NULL,
    reply_status VARCHAR(32),
    reply_error_code VARCHAR(64),
    reply_error_message TEXT,
    error_code VARCHAR(64),
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Runtime 调用日志/错误索引
CREATE TABLE runtime_logs (
    id UUID PRIMARY KEY,
    event_id VARCHAR(128) NOT NULL REFERENCES message_events(event_id),
    bot_id UUID NOT NULL REFERENCES bots(id),
    runtime_type VARCHAR(32) NOT NULL,
    request_payload JSONB,
    response_payload JSONB,
    status VARCHAR(32) NOT NULL,
    error_code VARCHAR(64),
    error_message TEXT,
    latency_ms INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================================
-- Indexes (TAD §8.3)
-- ============================================================

-- sessions 高频查询：按 bot + platform + chat_id 查找 session
CREATE INDEX idx_sessions_bot_platform_chat ON sessions (bot_id, platform, chat_id);

-- message_events 入站幂等：同一来源同一 message_id 只处理一次
CREATE UNIQUE INDEX uq_message_events_inbound_dedup
    ON message_events (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''), bridge_message_id);

-- message_events 回写幂等：同一 reply_id 只回写一次
CREATE UNIQUE INDEX uq_message_events_reply_id ON message_events (reply_id);

-- message_events 高频查询：按 session + 时间排序
CREATE INDEX idx_message_events_session_created ON message_events (session_id, created_at);

-- message_events 按 bot 查询
CREATE INDEX idx_message_events_bot ON message_events (bot_id);

-- channel_bindings 按 bot + platform 查询（反向查询）
CREATE INDEX idx_channel_bindings_bot_platform ON channel_bindings (bot_id, platform);

-- channel_bindings 主查询路径：按 platform + bridge_gateway_name + bridge_channel_name 解析 bot_id
-- TAD §6.1.1 明确该查询模式；COALESCE 处理 NULL bridge_channel_name（降级匹配）
CREATE INDEX idx_channel_bindings_lookup
    ON channel_bindings (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''));

-- runtime_logs 按 event_id 查询
CREATE INDEX idx_runtime_logs_event ON runtime_logs (event_id);

-- runtime_logs 按 bot + 时间查询
CREATE INDEX idx_runtime_logs_bot_created ON runtime_logs (bot_id, created_at);

-- +goose StatementEnd

-- +goose Down
-- +goose StatementBegin

DROP INDEX IF EXISTS idx_runtime_logs_bot_created;
DROP INDEX IF EXISTS idx_runtime_logs_event;
DROP INDEX IF EXISTS idx_channel_bindings_lookup;
DROP INDEX IF EXISTS idx_channel_bindings_bot_platform;
DROP INDEX IF EXISTS idx_message_events_bot;
DROP INDEX IF EXISTS idx_message_events_session_created;
DROP INDEX IF EXISTS uq_message_events_reply_id;
DROP INDEX IF EXISTS uq_message_events_inbound_dedup;
DROP INDEX IF EXISTS idx_sessions_bot_platform_chat;

DROP TABLE IF EXISTS runtime_logs;
DROP TABLE IF EXISTS message_events;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS channel_bindings;
DROP TABLE IF EXISTS bots;

-- +goose StatementEnd
