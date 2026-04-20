## Context

本提案为 Gateway 入站 handler 实现 channel_bindings 查询与 session 管理能力。`channel_bindings` 表结构已在 `00001_init.sql` 完整定义；`sessions` 表的 `session_id UNIQUE` 全局约束（`00001_init.sql`）在多 Bot 场景下存在缺陷，需新增迁移 `00003` 将约束升级为 `(bot_id, session_id)` 联合唯一。

## Goals / Non-Goals
- Goals:
  - 确定 channel_bindings COALESCE 降级查询的 SQL 实现方式
  - 确定 session_id 生成函数的代码位置
  - 确定 sessions upsert 的幂等 SQL 策略
- Non-Goals:
  - 修改 API 契约（SSoT/api/main.tsp 404 已定义）
  - 引入用户级 session 粒度（TD-006，Post-MVP）

## Decisions

- **Decision: channel_bindings 降级查询采用两步 SQL 而非单一复杂 SQL**
  - 实现（完整逻辑）：
    - 若 `bridge_channel_name = Some("general")`：先精确查询 `WHERE platform=$1 AND bridge_gateway_name=$2 AND COALESCE(bridge_channel_name,'')='general'`；若无结果，第二步降级查询 `WHERE platform=$1 AND bridge_gateway_name=$2 AND bridge_channel_name IS NULL`
    - 若 `bridge_channel_name = None`：直接执行降级查询（`bridge_channel_name IS NULL`），跳过无意义的精确查询 roundtrip
  - 复用 `idx_channel_bindings_lookup` 的正确论据：该索引定义为 `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''))`。精确查询谓词写为 `COALESCE(bridge_channel_name,'') = COALESCE($3,'')` 与索引表达式一致，优化器可走 Index Scan；降级查询谓词 `bridge_channel_name IS NULL` 等价于索引中 `COALESCE(bridge_channel_name,'') = ''`，同样命中索引
  - Alternatives considered: 单 SQL `WHERE ... AND (bridge_channel_name=$3 OR bridge_channel_name IS NULL) ORDER BY ...` — 功能等价但可读性差，拒绝

- **Decision: `generate_session_id()` 作为纯函数置于 `gateway/src/models/` 或内联于 handler**
  - 实现：纯函数 `fn generate_session_id(platform: &str, chat_type: &str, chat_id: &str) -> String`，无副作用，便于单元测试
  - 理由：session_id 生成规则固定（BR-010/011），不依赖 DB 或外部状态，应为纯函数

- **Decision: sessions upsert 使用 `ON CONFLICT (bot_id, session_id) DO UPDATE`，并新建迁移 `00003`**
  - SQL：`INSERT INTO sessions (...) ON CONFLICT (bot_id, session_id) DO UPDATE SET updated_at = NOW(), last_user_id = EXCLUDED.last_user_id`
  - 迁移 `00003`：`ALTER TABLE sessions DROP CONSTRAINT sessions_session_id_key`；`CREATE UNIQUE INDEX uq_sessions_bot_session ON sessions (bot_id, session_id)`
  - 理由：`00001_init.sql` 中 `sessions.session_id UNIQUE` 是全局唯一约束；多 Bot 共享同一 PostgreSQL 实例时（BR-032），两个不同 Bot 管理相同 chat_id 将产生相同 session_id 字符串（如 `telegram:private:12345`），引发唯一约束冲突。将冲突键升级为 `(bot_id, session_id)` 后，相同 session_id 在不同 bot_id 下可独立共存，幂等 upsert 仍然有效

## Risks / Trade-offs

- 两步查询（精确 + 降级）有两次 DB roundtrip，对大多数消息（有精确绑定）命中第一步即返回，性能可接受；内网延迟忽略不计
- `generate_session_id()` 当前 MVP 仅处理 `private` / `group`；未来新增 chat_type 时需同步更新此函数

## Migration Plan

新增 `SSoT/schema/migrations/00003_sessions_bot_session_unique.sql`：
1. `ALTER TABLE sessions DROP CONSTRAINT sessions_session_id_key`（删除全局单列唯一约束）
2. `CREATE UNIQUE INDEX uq_sessions_bot_session ON sessions (bot_id, session_id)`（新建联合唯一索引）

Down 迁移可逆：删除新索引并恢复旧约束。

## Open Questions

无。
