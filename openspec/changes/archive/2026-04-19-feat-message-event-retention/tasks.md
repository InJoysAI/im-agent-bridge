# 实施任务清单

> 前置依赖 `feat-gateway-db-layer` 已完成。本变更为纯 DB 层运维机制；无 API 契约变更，无 Gateway Rust 代码修改。SSoT 涉及：Goose 迁移（单列索引 + pg_cron 清理过程与调度）。pg_cron 扩展与 Docker 配置已由 `feat-runtime-log-retention` 就绪，无需重复配置。

## 1. SSoT 先行检查

- [x] 1.1 确认 API 层无变更：本提案不涉及 `SSoT/api/main.tsp` 修改，跳过 TypeSpec 更新
- [x] 1.2 确认 `message_events` 当前无单列 `created_at` 索引（现有 `idx_message_events_session_created(session_id, created_at)` 为复合索引，不足以支撑仅按 `created_at` 过滤的批量 DELETE）
- [x] 1.3 确认 pg_cron 扩展已就绪（`feat-runtime-log-retention` 已通过 `00007_runtime_logs_retention_cron.sql` 创建扩展并配置 Docker 环境）

## 2. Schema 变更（Goose 迁移）

- [x] 2.1 创建迁移文件 `SSoT/schema/migrations/00008_message_events_retention_idx.sql`
  - 文件头必须包含 `-- +goose NO TRANSACTION`（`CREATE INDEX CONCURRENTLY` 不支持在事务内执行）
  - Up: `CREATE INDEX CONCURRENTLY idx_message_events_created_at ON message_events (created_at);`
  - Down: `DROP INDEX IF EXISTS idx_message_events_created_at;`
- [x] 2.2 本地验证迁移执行
  ```bash
  export GOOSE_DRIVER=postgres
  # 统一从本地 gateway/.env 读取 DATABASE_URL（避免把明文用户名/密码写入仓库）。
  # 注意：DATABASE_URL 可能包含 `&`，不要直接 `source gateway/.env`，否则 shell 会把 `&` 解释为后台符号。
  export DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')"
  export GOOSE_DBSTRING="$DATABASE_URL"
  make db-migrate-up
  ```
- [x] 2.3 `goose status` 确认 `00008_message_events_retention_idx.sql` 状态为 `applied`

## 3. pg_cron 清理任务配置

- [x] 3.1 创建 Goose 迁移 `SSoT/schema/migrations/00009_message_events_retention_cron.sql`：
  - `CREATE OR REPLACE PROCEDURE cleanup_message_events() ...`（DELETE + LIMIT 1000 + COMMIT 分批，清理 `created_at < NOW() - INTERVAL '30 days'` 的行）
  - job 注册采用幂等策略：先在 `DO $$ BEGIN ... EXCEPTION WHEN ... END $$` 块中执行 `cron.unschedule('message-events-cleanup')`（容忍 job 不存在），再执行 `cron.schedule('message-events-cleanup', '30 3 * * *', 'CALL cleanup_message_events()')`（每日 03:30 UTC，与 `runtime-logs-cleanup` 03:00 错峰）
- [x] 3.2 使用统一流程执行迁移：`make db-migrate-up`
- [x] 3.3 运行态验证：
  - `SELECT jobid, jobname, schedule, command FROM cron.job WHERE jobname='message-events-cleanup'` 返回已注册任务

## 4. 测试

### 4.1 单元测试（迁移文件验证）

- [x] 4.1.1 验证 `00008_message_events_retention_idx.sql` 文件头包含 `-- +goose NO TRANSACTION` 注解（`CREATE INDEX CONCURRENTLY` 不支持事务内执行，缺失此注解将导致迁移失败）
- [x] 4.1.2 验证 `00008_message_events_retention_idx.sql` 包含 `-- +goose Up` 与 `-- +goose Down` 两个方向的迁移段
- [x] 4.1.3 验证 `00009_message_events_retention_cron.sql` 包含 `-- +goose Up` 与 `-- +goose Down` 两个方向的迁移段
- [x] 4.1.4 验证 `cleanup_message_events()` 过程定义中 DELETE 条件为 `created_at < NOW() - INTERVAL '30 days'`（30 天，非 14 天；避免复制 `cleanup_runtime_logs()` 时遗漏修改）
- [x] 4.1.5 验证 `cron.schedule` 调用中 job 名称为 `'message-events-cleanup'`，调度表达式为 `'30 3 * * *'`（与 `runtime-logs-cleanup` 的 `'0 3 * * *'` 错峰 30 分钟）
- [x] 4.1.6 验证 `-- +goose Down` 中包含 `cron.unschedule('message-events-cleanup')` 与 `DROP PROCEDURE IF EXISTS cleanup_message_events`（确保回滚完整）
- [x] 4.1.7 验证 `00009_message_events_retention_cron.sql` 的 `-- +goose Up` 段中 `cron.schedule` 调用前包含幂等处理逻辑：先 `cron.unschedule('message-events-cleanup')`（包裹在 `EXCEPTION WHEN` 块中以容忍 job 不存在），再执行 `cron.schedule`

### 4.2 集成测试（PostgreSQL 实例验证）

> 在 `deploy/postgres/docker-compose.yml` 启动的 PostgreSQL 实例上执行，需先 `make db-migrate-up` 应用全部迁移。

- [x] 4.2.1 索引存在性验证：
  ```sql
  SELECT indexname FROM pg_indexes
  WHERE tablename = 'message_events' AND indexname = 'idx_message_events_created_at';
  ```
  预期返回 1 行
- [x] 4.2.2 过期行清理验证：插入 `created_at = NOW() - INTERVAL '31 days'` 的测试行 → 执行 `CALL cleanup_message_events();` → 验证该行已被删除
  ```sql
  -- 准备：插入过期测试行
  INSERT INTO message_events (id, event_id, bot_id, session_id, platform, bridge_gateway_name, bridge_message_id, reply_id, chat_id, chat_type, status, created_at)
  VALUES (gen_random_uuid(), 'test-expired-evt-' || gen_random_uuid()::text, (SELECT id FROM bots LIMIT 1), 'test:session', 'telegram', 'test-gw', 'test-msg-' || gen_random_uuid()::text, 'test-reply-' || gen_random_uuid()::text, '999', 'private', 'done', NOW() - INTERVAL '31 days');
  -- 执行清理
  CALL cleanup_message_events();
  -- 验证
  SELECT COUNT(*) FROM message_events WHERE created_at < NOW() - INTERVAL '30 days';
  -- 预期：0
  ```
- [x] 4.2.3 正常行保留验证：插入 `created_at = NOW() - INTERVAL '15 days'` 的测试行 → 执行 `CALL cleanup_message_events();` → 验证该行仍存在
  ```sql
  INSERT INTO message_events (id, event_id, bot_id, session_id, platform, bridge_gateway_name, bridge_message_id, reply_id, chat_id, chat_type, status, created_at)
  VALUES (gen_random_uuid(), 'test-fresh-evt-' || gen_random_uuid()::text, (SELECT id FROM bots LIMIT 1), 'test:session', 'telegram', 'test-gw', 'test-msg-' || gen_random_uuid()::text, 'test-reply-' || gen_random_uuid()::text, '999', 'private', 'done', NOW() - INTERVAL '15 days');
  CALL cleanup_message_events();
  SELECT COUNT(*) FROM message_events WHERE event_id LIKE 'test-fresh-evt-%';
  -- 预期：≥1
  ```
- [x] 4.2.4 空表/无过期行时无错误：确保 `message_events` 中无过期行后执行 `CALL cleanup_message_events();`，过程正常结束无报错
- [x] 4.2.5 大批量分批验证：插入 > 1000 条 `created_at = NOW() - INTERVAL '31 days'` 的测试行 → 执行 `CALL cleanup_message_events();` → 验证全部被删除（验证循环 DELETE + LIMIT 1000 多批次逻辑）
  ```sql
  -- 插入 1500 条过期行
  INSERT INTO message_events (id, event_id, bot_id, session_id, platform, bridge_gateway_name, bridge_message_id, reply_id, chat_id, chat_type, status, created_at)
  SELECT gen_random_uuid(), 'bulk-test-' || i || '-' || gen_random_uuid()::text, (SELECT id FROM bots LIMIT 1), 'test:session', 'telegram', 'test-gw', 'bulk-msg-' || i || '-' || gen_random_uuid()::text, 'bulk-reply-' || i || '-' || gen_random_uuid()::text, '999', 'private', 'done', NOW() - INTERVAL '31 days'
  FROM generate_series(1, 1500) AS s(i);
  CALL cleanup_message_events();
  SELECT COUNT(*) FROM message_events WHERE created_at < NOW() - INTERVAL '30 days';
  -- 预期：0
  ```
- [x] 4.2.6 索引使用验证：确认 DELETE 查询走索引扫描而非全表扫描
  ```sql
  EXPLAIN SELECT id FROM message_events WHERE created_at < NOW() - INTERVAL '30 days' LIMIT 1000;
  -- 预期：包含 idx_message_events_created_at 的 Index Scan 或 Bitmap Index Scan，不出现 Seq Scan
  ```
- [x] 4.2.7 并发写入无锁验证：在一个会话中执行 `CALL cleanup_message_events()`，同时在另一个会话中执行 `INSERT INTO message_events(...)` + `SET lock_timeout = '1s'`，确认 INSERT 成功返回而非锁超时
  ```sql
  -- 会话 A：CALL cleanup_message_events();
  -- 会话 B（并发）：
  SET lock_timeout = '1s';
  INSERT INTO message_events (id, event_id, bot_id, session_id, platform, bridge_gateway_name, bridge_message_id, reply_id, chat_id, chat_type, status, created_at)
  VALUES (gen_random_uuid(), 'concurrent-evt-' || gen_random_uuid()::text, (SELECT id FROM bots LIMIT 1), 'test:session', 'telegram', 'test-gw', 'concurrent-msg-' || gen_random_uuid()::text, 'concurrent-reply-' || gen_random_uuid()::text, '999', 'private', 'done', NOW());
  -- 预期：INSERT 成功，未触发锁等待超时
  -- 附加验证：
  SELECT * FROM pg_locks WHERE relation = 'message_events'::regclass AND mode = 'AccessExclusiveLock';
  -- 预期：空结果集
  ```
- [x] 4.2.8 pg_cron job 注册验证：
  ```sql
  SELECT jobid, jobname, schedule, command FROM cron.job WHERE jobname = 'message-events-cleanup';
  -- 预期：返回 1 行，schedule = '30 3 * * *'，command 包含 'cleanup_message_events'
  ```
- [x] 4.2.9 清理失败告警验证（BR-042）：模拟清理过程执行失败（如临时 REVOKE DELETE 权限后调用），验证 `cron.job_run_details` 中记录失败状态且后续调度不受影响
  ```sql
  -- 验证失败记录
  SELECT status, return_message FROM cron.job_run_details
  WHERE jobid = (SELECT jobid FROM cron.job WHERE jobname = 'message-events-cleanup')
  ORDER BY start_time DESC LIMIT 1;
  -- 预期：status = 'failed'，return_message 包含错误描述
  -- 验证后续调度未被禁用
  SELECT jobid, active FROM cron.job WHERE jobname = 'message-events-cleanup';
  -- 预期：active = true
  ```
- [x] 4.2.10 job 注册幂等性验证：连续执行两次 `00009` 迁移的 Up 段 SQL（模拟 `goose up` 重试），验证第二次执行不报唯一约束冲突，且 `cron.job` 中 `message-events-cleanup` 仍只有 1 条记录
  ```sql
  SELECT COUNT(*) FROM cron.job WHERE jobname = 'message-events-cleanup';
  -- 预期：1（非 2）
  ```

### 4.3 手动测试（运维场景验证）

- [x] 4.3.1 端到端 Docker 环境验证：从零启动 `deploy/postgres/docker-compose.yml` → `make db-migrate-up` 执行全部迁移（00001-00009） → 插入测试 bot 与过期 message_events 行 → 手动 `CALL cleanup_message_events();` → 确认清理成功
- [x] 4.3.2 cron job 实际执行验证：等待 pg_cron 调度时间到达（或临时修改 schedule 为 `'* * * * *'` 每分钟执行），确认 `cron.job_run_details` 中出现 `message-events-cleanup` 执行记录且状态为 `succeeded`
  ```sql
  SELECT jobid, runid, job_pid, status, return_message, start_time, end_time
  FROM cron.job_run_details
  WHERE jobid = (SELECT jobid FROM cron.job WHERE jobname = 'message-events-cleanup')
  ORDER BY start_time DESC LIMIT 5;
  ```
- [x] 4.3.3 Autovacuum 后续验证：在大批量清理后检查 dead tuples 是否被 autovacuum 及时清理
  ```sql
  SELECT relname, n_dead_tup, last_autovacuum, last_autoanalyze
  FROM pg_stat_user_tables
  WHERE relname = 'message_events';
  ```
- [x] 4.3.4 Goose 迁移回滚验证：执行 `goose down` 两次（先回滚 00009 再回滚 00008），确认 cron job 被移除、索引被删除，且 `goose status` 显示状态正确
- [x] 4.3.5 备选方案验证（可选）：若需验证宿主机 cron 备选方案，创建 `deploy/postgres/cleanup-message-events.sh`（参考 `cleanup-runtime-logs.sh`，修改表名和保留天数为 30 天），通过 `crontab` 调度执行并确认清理效果（本次因 pg_cron 可用而跳过）

## 5. 文档

- [x] 5.1 更新 `.context/db/schema_design.md` §索引策略：在索引清单中增加 `idx_message_events_created_at ON message_events (created_at)` 条目。更新方式：手动编辑 `schema_design.md` 对应索引章节追加新索引行（该文件为人工维护的 Context 文档，不通过自动生成工具更新；SSoT/schema/migrations 为唯一 DDL 真实来源，`schema_design.md` 仅作架构说明参考）

## 6. 验证与归档

- [x] 6.1 specflow validate feat-message-event-retention --strict（运行提案严格验证）：
  ```
  node design/context-dev/tools/specflow/specflow.mjs validate feat-message-event-retention --strict
  ```
- [x] 6.2 代码审查：确认 pg_cron SQL 语法正确，DELETE + LIMIT 分批逻辑无误，索引 CONCURRENTLY 关键字已加
- [x] 6.3 specflow archive feat-message-event-retention --yes（合并后归档）：
  ```
  node design/context-dev/tools/specflow/specflow.mjs archive feat-message-event-retention --yes
  ```
