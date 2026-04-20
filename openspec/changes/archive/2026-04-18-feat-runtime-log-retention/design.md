## Context

`runtime_logs` 表需要 14 天 TTL 清理（`criterion.md §4`，`security_policy.md §82-88`）。`feat-persist-runtime-logs` 已完成写入机制（仅 `status=error` 时写入，脱敏 PII），本提案补齐清理机制与索引策略。

## Goals / Non-Goals

- Goals:
  - 确保 `runtime_logs` 中 `created_at > 14d` 的行被自动删除
  - 清理操作不阻塞 Gateway 主链路（分批、行级锁 only）
  - 单列索引支撑清理查询高效执行（避免 Seq Scan）
- Non-Goals:
  - 其他表（`message_events`）的清理（超出本提案范围）
  - 实时逐行过期（批量定时清理足够）
  - `pg_partman` 自动分区管理（MVP 阶段过度设计）

## Decisions

### Decision 1: 清理机制落位于 Goose 迁移（主路径）+ 宿主机 cron（备选）

- **Decision**: 主路径通过 Goose 迁移统一创建/维护 pg_cron 能力；备选宿主机 crontab + psql 脚本
- **pg_cron 优势**: 在数据库内调度，无需外部 cron 依赖；失败历史可通过 `cron.job_run_details` 审计；与 PostgreSQL 事务语义一致
- **pg_cron 限制**: 需 PostgreSQL 扩展；需在 `shared_preload_libraries` 中预加载；托管数据库（RDS、CloudSQL）需确认扩展可用性
- **备选触发条件**: pg_cron 不可用时（如某些托管 DB 不允许扩展安装）改用宿主机 crontab：`0 3 * * * psql $DATABASE_URL -c "DELETE FROM runtime_logs WHERE created_at < NOW() - INTERVAL '14 days'"`
- **实施落位**:
  - `SSoT/schema/migrations/00007_runtime_logs_retention_cron.sql`：`CREATE EXTENSION` + `cleanup_runtime_logs()` + `cron.schedule(...)`
  - `deploy/postgres/dockerfile` + `deploy/postgres/docker-compose.yml`：提供 pg_cron 运行前置（安装扩展包 + preload + `cron.database_name=im`）
- **Alternatives considered**: `pg_partman`（RANGE 分区 + 自动 DROP old partition）清理成本低但初始配置复杂，需重建表并停机迁移；MVP 阶段不采用；可作为 Post-MVP 优化路径（`schema_design.md §分区策略` 已备注）

### Decision 2: 索引策略 — 新增 idx_runtime_logs_created_at 单列索引

- **Decision**: 新增 `CREATE INDEX CONCURRENTLY idx_runtime_logs_created_at ON runtime_logs (created_at)`
- **Rationale**: 现有 `idx_runtime_logs_bot_created ON runtime_logs (bot_id, created_at)` 为复合索引；当 WHERE 子句仅包含 `created_at < X`（无 `bot_id` 过滤）时，PostgreSQL 查询规划器无法高效使用该复合索引（前缀列缺失，可能降级为 Seq Scan）。单列索引可将 DELETE 查询精确走 Index Scan/Bitmap Index Scan
- **CONCURRENTLY 原因**: 添加索引时不锁表（`migrations_and_ssot.md §向后兼容变更`），不影响 Gateway 并发写入
- **Alternatives considered**: 依赖现有复合索引（`(bot_id, created_at)`）— 在 MVP 规模小表时可能工作，但不稳定；随表增长 planner 行为难以预测，不采用

### Decision 3: DELETE 分批策略 — CREATE PROCEDURE（非 DO 块）

- **Decision**: 使用 PostgreSQL 存储过程（`CREATE PROCEDURE`）封装循环 DELETE + COMMIT，由 pg_cron 调度 `CALL cleanup_runtime_logs()`
- **Rationale**:
  - 单次大批量 DELETE 产生大量 dead tuples 并延长锁持有时间；1000 行/批可快速完成，autovacuum 有时间在两批之间清理
  - PostgreSQL `DO` 匿名块**不支持事务控制语句**（`COMMIT`/`ROLLBACK`）；在 DO 块内循环而无 COMMIT，所有批次仍在同一事务中累积，行锁积压，dead tuples 对 autovacuum 不可见 — 与分批设计初衷完全背道而驰
  - PostgreSQL 11+ 的 `CREATE PROCEDURE` 允许过程体内执行 `COMMIT`，是实现"分批提交"的正确方式
- **PROCEDURE 方案**:
  ```sql
  CREATE OR REPLACE PROCEDURE cleanup_runtime_logs()
  LANGUAGE plpgsql
  AS $$
  DECLARE deleted INT;
  BEGIN
    LOOP
      DELETE FROM runtime_logs
      WHERE id IN (
        SELECT id FROM runtime_logs
        WHERE created_at < NOW() - INTERVAL '14 days'
        LIMIT 1000
      );
      GET DIAGNOSTICS deleted = ROW_COUNT;
      COMMIT;  -- 每批独立提交：释放行锁，dead tuples 对 autovacuum 立即可见
      EXIT WHEN deleted = 0;
    END LOOP;
  END;
  $$;

  -- pg_cron 调度（每日 03:00 UTC）
  SELECT cron.schedule('runtime-logs-cleanup', '0 3 * * *', 'CALL cleanup_runtime_logs()');
  ```
- **Alternatives considered**: 单次全量 DELETE（简单但锁风险高）；`TRUNCATE`（禁止，`migrations_and_ssot.md §生产环境禁止操作`）；高频单语句 pg_cron job（如每 15 分钟 `DELETE ... LIMIT 1000` 无循环）— 适用于 PostgreSQL < 11 或不需 procedure 的场景，语义等效但调度复杂度更高

## Risks / Trade-offs

- pg_cron 依赖 PostgreSQL 实例类型；生产使用托管 DB 前需确认扩展可用性（已准备备选方案）
- 分批 DELETE 在存量过期数据极大（首次运行场景）时执行时间可能较长；建议在启用定时 job 前手动批量清理一次
- `CREATE INDEX CONCURRENTLY` 在 Goose 迁移中需注意：Goose 默认将迁移包在事务内，而 `CONCURRENTLY` 不支持在事务内执行；迁移文件需使用 `-- +goose NO TRANSACTION` 注解或拆分为独立 statement
- autovacuum 配置需充分（`performance_tuning.md` 已建议激进参数）；首次大批量清理后建议手动执行 `VACUUM ANALYZE runtime_logs`

## Migration Plan

1. 应用 Goose 迁移（`00006_runtime_logs_retention_idx.sql`，注意 `NO TRANSACTION` 注解）
2. 启动 PostgreSQL（`deploy/postgres/docker-compose.yml`）并确保 `shared_preload_libraries=pg_cron` 与 `cron.database_name=im` 生效
3. 应用 Goose 迁移（`00007_runtime_logs_retention_cron.sql`）创建扩展、过程与定时任务
4. 首次运行：若存量过期数据大，手动执行 `CALL cleanup_runtime_logs()` 清理存量后再依赖定时 job
5. 验证：`cron.job` 确认 job 已注册；手动插入 15 天前测试数据并调用 `CALL cleanup_runtime_logs()` 验证删除

## Open Questions

- 生产 PostgreSQL 是使用 Docker 自建还是托管实例？若为托管，需在实施前确认 pg_cron 扩展是否可用（如 AWS RDS 原生支持，但 Aurora Serverless v1 不支持）
