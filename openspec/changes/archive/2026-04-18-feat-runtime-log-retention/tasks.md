# 实施任务清单

> 前置依赖 `feat-persist-runtime-logs` 已完成。本变更为纯 DB 层运维机制；无 API 契约变更，无 Gateway Rust 代码修改。SSoT 涉及：Goose 迁移（单列索引 + pg_cron 过程与调度）+ deploy 配置（pg_cron 运行参数）。

## 1. SSoT 先行检查

- [x] 1.1 确认 API 层无变更：本提案不涉及 `SSoT/api/main.tsp` 修改，跳过 TypeSpec 更新
- [x] 1.2 确认 `runtime_logs` 当前无单列 `created_at` 索引（现有 `idx_runtime_logs_bot_created(bot_id, created_at)` 为复合索引，不足以支撑仅按 `created_at` 过滤的批量 DELETE）

## 2. Schema 变更（Goose 迁移）

- [x] 2.1 创建迁移文件 `SSoT/schema/migrations/00006_runtime_logs_retention_idx.sql`
  - 文件头必须包含 `-- +goose NO TRANSACTION`（`CREATE INDEX CONCURRENTLY` 不支持在事务内执行）
  - Up: `CREATE INDEX CONCURRENTLY idx_runtime_logs_created_at ON runtime_logs (created_at);`
  - Down: `DROP INDEX IF EXISTS idx_runtime_logs_created_at;`
- [x] 2.2 本地验证
  ```bash
  export GOOSE_DRIVER=postgres
  export DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')"
  export GOOSE_DBSTRING="$DATABASE_URL"
  make db-migrate-up
  ```
- [x] 2.3 `goose status` 确认 `00006_runtime_logs_retention_idx.sql` 状态为 `applied`

## 3. pg_cron 清理任务配置

- [x] 3.1 改造 `deploy/postgres/dockerfile`：安装 `postgresql-18-cron`，并在启动参数启用 `shared_preload_libraries=pg_cron`
- [x] 3.2 修改 `deploy/postgres/docker-compose.yml`：保留 `postgres` 单服务，启动参数包含 `cron.database_name=im`，并使用 `deploy/postgres/.env` 注入配置
- [x] 3.3 新增 Goose 迁移 `SSoT/schema/migrations/00007_runtime_logs_retention_cron.sql`：
  - `CREATE EXTENSION IF NOT EXISTS pg_cron;`
  - `CREATE OR REPLACE PROCEDURE cleanup_runtime_logs() ...`（DELETE + LIMIT 1000 + COMMIT 分批）
  - `cron.schedule('runtime-logs-cleanup', '0 3 * * *', 'CALL cleanup_runtime_logs()')`
- [x] 3.4 使用统一流程执行迁移：`make db-migrate-up`（不再依赖 `deploy/postgres/pg-cron-setup.sql` 手工注册）
- [x] 3.5 运行态验证：
  - `SHOW shared_preload_libraries` 包含 `pg_cron`
  - `SELECT extname FROM pg_extension WHERE extname='pg_cron'` 返回 1 行
  - `SELECT jobid, jobname, schedule, command FROM cron.job WHERE jobname='runtime-logs-cleanup'` 返回已注册任务

## 4. 验证测试

- [x] 4.1 插入测试行：`INSERT INTO runtime_logs (..., created_at) VALUES (..., NOW() - INTERVAL '15 days');`
- [x] 4.2 手动执行清理 SQL，确认测试行被删除（`SELECT COUNT(*) FROM runtime_logs WHERE created_at < NOW() - INTERVAL '14 days'` = 0）
- [x] 4.3 插入正常行：`created_at = NOW() - INTERVAL '7 days'`，确认清理后该行仍存在
- [x] 4.4 使用 `EXPLAIN` 确认 DELETE 查询使用 `idx_runtime_logs_created_at`（Index Scan 或 Bitmap Index Scan，非 Seq Scan）
- [x] 4.5 手动回归验证：插入 15 天前测试行后执行 `CALL cleanup_runtime_logs();`，确认被删除
  - 实际执行结果：`before_cleanup=1`，`after_cleanup=0`

## 5. 文档

- [x] 5.1 更新 `.context/db/schema_design.md` §索引策略：在索引清单中增加 `idx_runtime_logs_created_at ON runtime_logs (created_at)` 条目

## 6. 验证与归档

- [x] 6.1 specflow validate feat-runtime-log-retention --strict（运行提案严格验证）：
  ```
  node design/context-dev/tools/specflow/specflow.mjs validate feat-runtime-log-retention --strict
  ```
- [x] 6.2 代码审查：确认 pg_cron SQL 语法正确，DELETE + LIMIT 分批逻辑无误，索引 CONCURRENTLY 关键字已加
- [ ] 6.3 specflow archive feat-runtime-log-retention --yes（合并后归档）：
  ```
  node design/context-dev/tools/specflow/specflow.mjs archive feat-runtime-log-retention --yes
  ```
