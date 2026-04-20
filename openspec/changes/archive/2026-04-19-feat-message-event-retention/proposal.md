# Change: message_events 30 天过期数据自动清理

## Why

`criterion.md §4`（数据治理）与 `security_policy.md §82-88`（message_events 保留期 30 天）共同要求 `message_events` 表中写入超过 30 天的行必须被自动清理。

当前状态：`feat-gateway-db-layer` 已完成 `message_events` 表建表与写入机制（入站消息事件持久化），但尚无任何定时清理机制。如不清理，过期消息事件将无限累积，造成合规风险与存储膨胀（TD-005 技术债务中 `message_events` 部分）。全量路线图审查报告将此识别为 P-01 级 CRITICAL 硬合规空白。

本提案与 `feat-runtime-log-retention`（runtime_logs 14 天清理，已完成）采用一致的实现模式，补齐 `message_events` 的 30 天 TTL 清理机制。

## What Changes

### 新增功能
- 新增 Goose 迁移 `00008_message_events_retention_idx.sql`：为 `message_events.created_at` 补建单列 B-Tree 索引 `idx_message_events_created_at`，供批量清理 DELETE 查询高效定位过期行
- 新增 Goose 迁移 `00009_message_events_retention_cron.sql`：创建 `cleanup_message_events()` 存储过程与 `message-events-cleanup` pg_cron 定时任务注册

### 修改功能
- 无修改功能（pg_cron 扩展与 Docker 配置已由 `feat-runtime-log-retention` 完成安装与启用）

### 技术实现
- Goose 迁移：`CREATE INDEX CONCURRENTLY idx_message_events_created_at ON message_events (created_at)`（现有 `idx_message_events_session_created(session_id, created_at)` 为复合索引，前缀列为 `session_id`，无法高效支撑仅按 `created_at` 过滤的批量 DELETE，需补建单列索引）
- pg_cron 迁移：在 `00009_message_events_retention_cron.sql` 中创建过程 `cleanup_message_events()` 并注册 `30 3 * * *` 调度任务（每日 03:30 UTC，与 `runtime-logs-cleanup` 03:00 错峰）
- 清理过程采用 `CREATE PROCEDURE` + 循环 DELETE + LIMIT 1000 + COMMIT 分批策略，与 `cleanup_runtime_logs()` 一致
- 备选方案：若目标环境 pg_cron 不可用，改为宿主机 `crontab` + psql 脚本，清理语义等价

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/message-event-retention/spec.md` -- message_events 过期清理行为规范（清理执行、正常行保留、无锁风险、索引支撑）

### 涉及的代码
- **新增**：
  - `SSoT/schema/migrations/00008_message_events_retention_idx.sql`（单列索引 Goose 迁移）
  - `SSoT/schema/migrations/00009_message_events_retention_cron.sql`（cleanup_message_events 过程 + job 注册）

- **修改**：无（pg_cron 运行环境已由 `feat-runtime-log-retention` 就绪）

### 依赖关系
- **依赖**：`feat-gateway-db-layer`（已完成；message_events 表结构必须先到位）
- **依赖**：`feat-runtime-log-retention`（已完成；pg_cron 扩展安装、Docker 配置、`shared_preload_libraries=pg_cron` 与 `cron.database_name=im` 均由该提案落地，本提案直接复用）
- **被依赖**：`fix-audit-remediation`（需本提案完成后方可开展审查缺口修复）

### 风险与注意事项
- pg_cron 扩展需要 PostgreSQL 支持；托管数据库（RDS、CloudSQL 等）需在实施前确认扩展可用性（备选宿主机 cron 方案已准备，与 `feat-runtime-log-retention` design.md 一致）
- 批量 DELETE 产生大量 dead tuples，需确保 autovacuum 配置充分（`performance_tuning.md` 已建议激进 autovacuum，含 `message_events` 高写入表）
- 首次运行若存量过期数据较多，可手动分批执行清理 SQL，再启用定时 job
- `message_events` 表写入量通常大于 `runtime_logs`（每条入站消息一条 event vs 仅 error 时写入 runtime_log），清理批次可能更多

### 验证标准
- ✅ 清理任务运行后，`message_events` 中无 `created_at < NOW() - 30 days` 的行
- ✅ 30 天内的正常行不受影响
- ✅ 批量删除不产生表级锁（DELETE + LIMIT 分批，pg_cron 低峰期执行）
- ✅ `message_events.created_at` 存在单列 B-Tree 索引 `idx_message_events_created_at`

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | openspec/proposal-roadmap.md |
| roadmap_source_supplement | N/A |
| phase | Phase 6 |
| business_goal | 满足 security_policy.md 及 criterion.md §4 数据治理要求：message_events 30 天 TTL 自动清理 |
| dependencies | 前置: feat-gateway-db-layer（done）, feat-runtime-log-retention（done）；被依赖: fix-audit-remediation |
| acceptance_criteria | 清理执行后无过期行；正常行保留；无锁风险；索引存在 |

### 关联 Context 资产
| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §4 数据治理：message_events 保留 30 天 |
| architecture | `.context/architecture/security_policy.md` | §82-88 数据保留期约束 |
| db | `.context/db/schema_design.md` | message_events 表结构与索引策略 |
| db | `.context/db/migrations_and_ssot.md` | Goose 迁移规范；CREATE INDEX CONCURRENTLY |
| db | `.context/db/performance_tuning.md` | 高写入表 autovacuum 配置 |
| architecture | `.context/architecture/risks_and_debt.md` | TD-005 数据保留清理依赖手动 |
| domain | `.context/domain/edge_cases.md` | 数据清理边缘场景参考（首次清理存量、空表处理） |
