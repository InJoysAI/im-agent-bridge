# DB — 数据库设计

> **Metadata**
> - **Source**: `.context/db/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-12 23:17`
> - **Generator**: `Context-Agent v1.0`

## 源文档

| 文件 | 类型 | 说明 |
|------|------|------|
| `source/IM-Agent-Bridge-TAD.md` | TAD §7/§8/§10/§11 | 数据库模型设计（PostgreSQL 数据模型、安全、可观测性） |

## 数据库概述

- **数据库**: PostgreSQL
- **访问方**: 主要由 Gateway 访问；Runtime 不强依赖数据库
- **隔离策略**: 所有 Bot 实例共享同一 PostgreSQL 实例，通过 `bot_id` 实现逻辑隔离

## 生成文件索引

| 文件 | 说明 |
|------|------|
| `README.md` | 本文件：DB 模块入口、文件索引、表设计概述、关键索引、数据治理 |
| `schema_design.md` | 表设计 / ER 图 / 索引策略 / JSONB / 约束规则 |
| `performance_tuning.md` | Autovacuum / 连接池 / 高频查询延迟目标 |
| `migrations_and_ssot.md` | Goose 工作流 / SSoT 原则 / 零停机迁移 |
| `security_hardening.md` | 认证 / 权限 / 敏感数据治理 / 数据保留期 |
| `observability.md` | 关键指标 / pg_stat_statements / Vacuum 监控 / Trace |

## 表设计

| 表名 | 用途 | 关键字段 |
|------|------|---------|
| `bots` | Bot 基础配置 | id (UUID), bot_name, runtime_type, runtime_endpoint, is_enabled |
| `channel_bindings` | Bot ↔ 渠道入口绑定 | bot_id, platform, bridge_gateway_name, bridge_channel_name |
| `sessions` | Session 映射 | session_id, bot_id, chat_id, chat_type, runtime_session_key |
| `message_events` | 消息事件/处理状态/回写状态 | event_id, bot_id, session_id, status, reply_status |
| `runtime_logs` | Runtime 调用日志/错误索引 | event_id, bot_id, runtime_type, status, latency_ms |

## 关键索引

| 索引名 | 表 | 类型 | 用途 |
|---------|------|------|------|
| `idx_sessions_bot_platform_chat` | `sessions` | B-tree | sessions 高频查询（bot_id, platform, chat_id） |
| `uq_message_events_inbound_dedup` | `message_events` | UNIQUE | 入站幂等去重（platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id） |
| `uq_message_events_reply_id` | `message_events` | UNIQUE | 回写幂等（reply_id） |
| `idx_message_events_session_created` | `message_events` | B-tree | 按 session + 时间排序查询 |
| `idx_message_events_bot` | `message_events` | B-tree | 按 bot_id 查询消息事件 |
| `idx_channel_bindings_lookup` | `channel_bindings` | B-tree | **主查询路径**：按 platform + bridge_gateway_name + COALESCE(bridge_channel_name,'') 解析 bot_id（IMPL-001） |
| `uq_channel_bindings_source` | `channel_bindings` | UNIQUE | 渠道来源唯一约束（platform, bridge_gateway_name, COALESCE(bridge_channel_name,'')），防止 bot_id 解析歧义 |
| `idx_channel_bindings_bot_platform` | `channel_bindings` | B-tree | 反向查询：按 bot_id + platform |
| `idx_runtime_logs_event` | `runtime_logs` | B-tree | 按 event_id 查询 Runtime 日志 |
| `idx_runtime_logs_bot_created` | `runtime_logs` | B-tree | 按 bot_id + 时间查询 Runtime 日志 |

## 数据治理

- `message_events.input_text/output_text`: 截断至 512 字符，保留 **30 天**
- `runtime_logs`: 仅 error 时写入 payload（脱敏 PII），保留 **14 天**
- `sessions`: 无自动过期，按 `updated_at` 清理长期不活跃会话

> ⚠️ `source/` 目录中的文件为权威来源，谨慎修改。
