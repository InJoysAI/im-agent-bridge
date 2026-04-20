## Context

`message_events` 表需要 30 天 TTL 清理（`criterion.md §4`，`security_policy.md §82-88`）。`feat-gateway-db-layer` 已完成 `message_events` 建表与写入机制，`feat-runtime-log-retention` 已完成 pg_cron 基础设施搭建（扩展安装、Docker 配置、runtime_logs 清理过程）。本提案在此基础上补齐 `message_events` 的 30 天清理机制。

## Goals / Non-Goals

- Goals:
  - 确保 `message_events` 中 `created_at > 30d` 的行被自动删除
  - 清理操作不阻塞 Gateway 主链路（分批、行级锁 only）
  - 单列索引支撑清理查询高效执行（避免 Seq Scan）
- Non-Goals:
  - 其他表（`sessions`）的清理（TD-004，独立后续提案 `feat-session-cleanup` 承接）
  - 实时逐行过期（批量定时清理足够）
  - `pg_partman` 自动分区管理（MVP 阶段过度设计）

## Decisions

### Decision 1: 复用 feat-runtime-log-retention 的 pg_cron 基础设施

- **Decision**: 不再重复创建 pg_cron 扩展与 Docker 配置；直接在新 Goose 迁移中创建 `cleanup_message_events()` 过程并注册 cron job
- **Rationale**: `00007_runtime_logs_retention_cron.sql` 已执行 `CREATE EXTENSION IF NOT EXISTS pg_cron`，`deploy/postgres/dockerfile` 已安装 `postgresql-18-cron`，`docker-compose.yml` 已配置 `shared_preload_libraries=pg_cron` 与 `cron.database_name=im`。重复创建扩展会报冲突或被 `IF NOT EXISTS` 忽略，没有必要
- **Impact**: 迁移 `00009` 仅包含过程创建与 job 注册，不包含 `CREATE EXTENSION`

### Decision 2: 索引策略 -- 新增 idx_message_events_created_at 单列索引

- **Decision**: 新增 `CREATE INDEX CONCURRENTLY idx_message_events_created_at ON message_events (created_at)`
- **Rationale**: 现有 `idx_message_events_session_created ON message_events (session_id, created_at)` 为复合索引；当 WHERE 子句仅包含 `created_at < X`（无 `session_id` 过滤）时，PostgreSQL 查询规划器无法高效使用该复合索引（前缀列缺失，可能降级为 Seq Scan）。单列索引可将 DELETE 查询精确走 Index Scan/Bitmap Index Scan
- **CONCURRENTLY 原因**: 添加索引时不锁表（`migrations_and_ssot.md §向后兼容变更`），不影响 Gateway 并发写入
- **Alternatives considered**: 依赖现有复合索引 -- 在 MVP 规模小表时可能工作，但不稳定；随表增长 planner 行为难以预测，不采用

### Decision 3: 调度时间错峰

- **Decision**: `message-events-cleanup` 调度时间设为 `30 3 * * *`（每日 03:30 UTC），与 `runtime-logs-cleanup` 的 `0 3 * * *`（每日 03:00 UTC）错峰 30 分钟
- **Rationale**: 避免两个清理 job 同时运行导致 I/O 竞争和 autovacuum 压力集中。`message_events` 写入量通常大于 `runtime_logs`（每条入站消息一条 event vs 仅 error 时写入 runtime_log），清理批次可能更多，错峰有助于资源平稳使用
- **时区假设**: pg_cron 的调度时间基于 PostgreSQL 实例的 `timezone` 参数（默认 UTC）。当前 Docker 部署配置未显式设置 `timezone`，因此调度时间 `30 3 * * *` 等价于 UTC 03:30。若生产环境 PostgreSQL 实例配置了非 UTC 时区（如 `Asia/Shanghai`），需相应调整 cron 表达式以确保清理任务在业务低峰期执行。可通过 `SHOW timezone;` 确认实例当前时区设置

### Decision 4: DELETE 分批策略 -- 与 cleanup_runtime_logs() 一致

- **Decision**: 使用 `CREATE PROCEDURE cleanup_message_events()` + 循环 DELETE + LIMIT 1000 + COMMIT 分批
- **Rationale**: 与 `feat-runtime-log-retention` Decision 3 一致。单次大批量 DELETE 产生大量 dead tuples 并延长锁持有时间；1000 行/批可快速完成，autovacuum 有时间在两批之间清理。`CREATE PROCEDURE` 允许过程体内执行 `COMMIT`，是实现"分批提交"的正确方式

### Decision 5: pg_cron job 注册幂等策略

- **Decision**: 在 `00009_message_events_retention_cron.sql` 的 `-- +goose Up` 段中，先执行 `SELECT cron.unschedule('message-events-cleanup')` 再执行 `SELECT cron.schedule('message-events-cleanup', ...)`，并使用 `DO $$ BEGIN ... EXCEPTION WHEN ... END $$` 包裹 unschedule 调用以忽略 job 不存在时的异常
- **Rationale**: Goose 迁移可能因网络或事务中断后重试（`goose up` 幂等性要求）。若 `cron.schedule` 被重复调用且 job 名称已存在，pg_cron 会抛出唯一约束冲突错误导致迁移失败。先 unschedule 再 schedule 确保迁移可安全重跑。`EXCEPTION WHEN` 处理首次执行时 job 不存在的场景
- **Impact**: 迁移脚本略增 3-5 行 PL/pgSQL 异常处理代码，换取幂等安全保障

## Risks / Trade-offs

- `message_events` 表写入量通常高于 `runtime_logs`，清理压力相应更大。autovacuum 配置需充分（`performance_tuning.md` 已建议激进参数）
- 首次运行若存量过期数据极大，执行时间可能较长；建议在启用定时 job 前手动批量清理一次
- `CREATE INDEX CONCURRENTLY` 需 Goose 迁移使用 `-- +goose NO TRANSACTION` 注解
- pg_cron 基础设施依赖 `feat-runtime-log-retention` 已完成；若该提案未先执行，需先完成其 Docker 配置与 00007 迁移

## Migration Plan

1. 应用 Goose 迁移 `00008_message_events_retention_idx.sql`（注意 `NO TRANSACTION` 注解）
2. 应用 Goose 迁移 `00009_message_events_retention_cron.sql` 创建过程与定时任务
3. 验证：`cron.job` 确认 job 已注册；手动插入 31 天前测试数据并调用 `CALL cleanup_message_events()` 验证删除
4. 首次运行：若存量过期数据大，手动执行 `CALL cleanup_message_events()` 清理存量后再依赖定时 job
