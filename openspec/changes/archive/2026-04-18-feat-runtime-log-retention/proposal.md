# Change: runtime_logs 14 天过期数据自动清理

## Why

`criterion.md §4`（数据治理）与 `security_policy.md §82-88`（runtime_logs 保留期 14 天）共同要求 `runtime_logs` 表中写入超过 14 天的行必须被自动清理。

当前状态：`feat-persist-runtime-logs` 已完成 runtime_logs 条件写入（仅 `status=error` 时写入，且脱敏 PII），但尚无任何定时清理机制。如不清理，过期错误日志将无限累积，造成合规风险与存储膨胀（TD-005 技术债务）。

本提案承接 TD-005，补齐清理机制，使 `runtime_logs` 完整履行 14 天 TTL 约束。

## What Changes

### 新增功能
- 新增 Goose 迁移 `00006_runtime_logs_retention_idx.sql`：为 `runtime_logs.created_at` 补建单列 B-Tree 索引 `idx_runtime_logs_created_at`，供批量清理 DELETE 查询高效定位过期行
- 新增 Goose 迁移 `00007_runtime_logs_retention_cron.sql`：统一管理 pg_cron 扩展、`cleanup_runtime_logs()` 过程与 `runtime-logs-cleanup` 定时任务注册

### 修改功能
- 修改 `deploy/postgres/docker-compose.yml`：PostgreSQL 服务启动参数显式设置 `shared_preload_libraries=pg_cron` 与 `cron.database_name=im`
- 修改 `deploy/postgres/dockerfile`：安装 `postgresql-18-cron`，确保镜像内提供 pg_cron 动态库

### 技术实现
- Goose 迁移：`CREATE INDEX CONCURRENTLY idx_runtime_logs_created_at ON runtime_logs (created_at)`（WHERE 子句不包含 `bot_id`，复合索引首列未被约束，无法高效支撑仅按 `created_at` 过滤的批量 DELETE，需补建单列索引）
- pg_cron 迁移：在 `00007_runtime_logs_retention_cron.sql` 中执行 `CREATE EXTENSION IF NOT EXISTS pg_cron`、创建过程 `cleanup_runtime_logs()`，并注册 `0 3 * * *` 调度任务
- 备选方案：若目标环境不可用 pg_cron（托管数据库限制），改为宿主机 `crontab` + `deploy/postgres/cleanup-runtime-logs.sh`，清理语义等价（详见 design.md）

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/runtime-log-retention/spec.md` — runtime_logs 过期清理行为规范（清理执行、正常行保留、无锁风险）

### 涉及的代码
- **新增**：
  - `SSoT/schema/migrations/00006_runtime_logs_retention_idx.sql`（单列索引 Goose 迁移）
  - `SSoT/schema/migrations/00007_runtime_logs_retention_cron.sql`（pg_cron 扩展 + 过程 + job 注册）
  - `deploy/postgres/cleanup-runtime-logs.sh`（备选：pg_cron 不可用时的宿主机定时清理脚本）
  - `deploy/postgres/.env.example`（PostgreSQL compose 配置变量模板）

- **修改**：
  - `deploy/postgres/docker-compose.yml`（PostgreSQL 服务启用 pg_cron 并设置 `cron.database_name`）
  - `deploy/postgres/dockerfile`（安装 `postgresql-18-cron`）
  - `deploy/postgres/README.md`（统一为 compose + migrate 执行路径）
  - `openspec/changes/feat-runtime-log-retention/tasks.md`（按实际实施过程更新）

### 依赖关系
- **依赖**：`feat-persist-runtime-logs`（已完成；runtime_logs 写入机制必须先到位）
- **被依赖**：无

### 风险与注意事项
- pg_cron 需要 PostgreSQL 扩展支持；托管数据库（RDS、CloudSQL 等）需在实施前确认扩展可用性（备选宿主机 cron 方案已准备）
- 批量 DELETE 产生大量 dead tuples，需确保 autovacuum 配置充分（`performance_tuning.md` 已建议激进 autovacuum，含 `message_events`/`runtime_logs` 高写入表）
- 首次运行若存量过期数据较多，可手动分批执行清理 SQL，再启用定时 job

### 验证标准
- ✅ 清理任务运行后，`runtime_logs` 中无 `created_at < NOW() - 14 days` 的行
- ✅ 14 天内的正常行不受影响
- ✅ 批量删除不产生表级锁（DELETE + LIMIT 分批 或 pg_cron 低峰期执行）
