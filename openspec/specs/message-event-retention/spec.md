# message-event-retention Specification

## Purpose
TBD

## Requirements
### Requirement: message_events 过期行自动清理
系统必须（MUST）通过定时任务将 `message_events` 表中 `created_at` 早于当前时间 30 天的行自动删除，以满足数据治理约束（`criterion.md §4`，`security_policy.md §82-88`）。

#### Scenario: 清理任务执行后无过期行
- **WHEN** 定时清理任务执行，且 `message_events` 中存在 `created_at < NOW() - INTERVAL '30 days'` 的行（模拟写入 `created_at = NOW() - INTERVAL '31 days'`）
- **THEN** 该行从 `message_events` 中被删除
- **AND** `SELECT COUNT(*) FROM message_events WHERE created_at < NOW() - INTERVAL '30 days'` 返回 0

#### Scenario: 30 天内正常行不受影响
- **WHEN** 定时清理任务执行，且 `message_events` 中存在 `created_at = NOW() - INTERVAL '15 days'` 的行
- **THEN** 该行仍存在于 `message_events` 表中

#### Scenario: 批量删除不产生表级锁
- **WHEN** `CALL cleanup_message_events()` 清理任务正在执行
- **THEN** 另一会话在 `SET lock_timeout = '1s'` 条件下执行 `INSERT INTO message_events(...)` 成功返回，未触发锁等待超时
- **AND** `SELECT * FROM pg_locks WHERE relation = 'message_events'::regclass AND mode = 'AccessExclusiveLock'` 返回空结果集

---

### Requirement: message_events.created_at 单列 B-Tree 索引
系统必须（MUST）在 `message_events` 表的 `created_at` 列建立单列 B-Tree 索引（`idx_message_events_created_at`），以支撑批量清理 DELETE 查询高效定位过期行，避免全表顺序扫描。

#### Scenario: 过期行清理查询使用索引扫描
- **WHEN** 执行 `EXPLAIN SELECT id FROM message_events WHERE created_at < NOW() - INTERVAL '30 days'`（与 `cleanup_message_events()` 内子查询一致）
- **THEN** 查询计划显示使用 `idx_message_events_created_at` 进行 Index Scan 或 Bitmap Index Scan，而非 Seq Scan

---

### Requirement: 清理失败告警（BR-042）
清理任务执行失败时，系统必须（MUST）将失败信息记录到 pg_cron 执行日志（`cron.job_run_details`），且失败不得（MUST NOT）阻塞 Gateway 主进程或其他定时任务的正常运行。

#### Scenario: 清理过程执行异常时记录失败状态
- **GIVEN** `cleanup_message_events()` 过程因异常（如连接中断、权限不足）执行失败
- **WHEN** pg_cron 调度器捕获该异常
- **THEN** `cron.job_run_details` 中该次执行记录的 `status` 为 `'failed'`，且 `return_message` 包含错误描述
- **AND** 后续 pg_cron 调度周期中，`message-events-cleanup` job 仍按原 schedule 正常调度（失败不导致 job 被禁用或移除）

#### Scenario: 清理失败不阻塞 Gateway 主进程
- **WHEN** `cleanup_message_events()` 执行失败
- **THEN** Gateway 应用进程不受影响（pg_cron 在独立 background worker 中运行，与应用连接池隔离）
- **AND** 其他 pg_cron job（如 `runtime-logs-cleanup`）正常执行不受干扰
