## ADDED Requirements

### Requirement: runtime_logs 过期行自动清理
系统必须（MUST）通过定时任务将 `runtime_logs` 表中 `created_at` 早于当前时间 14 天的行自动删除，以满足数据治理约束（`criterion.md §4`，`security_policy.md §82-88`）。

#### Scenario: 清理任务执行后无过期行
- **WHEN** 定时清理任务执行，且 `runtime_logs` 中存在 `created_at < NOW() - INTERVAL '14 days'` 的行（模拟写入 `created_at = NOW() - INTERVAL '15 days'`）
- **THEN** 该行从 `runtime_logs` 中被删除
- **AND** `SELECT COUNT(*) FROM runtime_logs WHERE created_at < NOW() - INTERVAL '14 days'` 返回 0

#### Scenario: 14 天内正常行不受影响
- **WHEN** 定时清理任务执行，且 `runtime_logs` 中存在 `created_at = NOW() - INTERVAL '7 days'` 的行
- **THEN** 该行仍存在于 `runtime_logs` 表中

#### Scenario: 批量删除不产生表级锁
- **WHEN** `CALL cleanup_runtime_logs()` 清理任务正在执行
- **THEN** 另一会话在 `SET lock_timeout = '1s'` 条件下执行 `INSERT INTO runtime_logs(...)` 成功返回，未触发锁等待超时
- **AND** `SELECT * FROM pg_locks WHERE relation = 'runtime_logs'::regclass AND mode = 'AccessExclusiveLock'` 返回空结果集

---

### Requirement: runtime_logs.created_at 单列 B-Tree 索引
系统必须（MUST）在 `runtime_logs` 表的 `created_at` 列建立单列 B-Tree 索引（`idx_runtime_logs_created_at`），以支撑批量清理 DELETE 查询高效定位过期行，避免全表顺序扫描。

#### Scenario: 过期行清理查询使用索引扫描
- **WHEN** 执行 `EXPLAIN SELECT id FROM runtime_logs WHERE created_at < NOW() - INTERVAL '14 days'`（与 `cleanup_runtime_logs()` 内子查询一致）
- **THEN** 查询计划显示使用 `idx_runtime_logs_created_at` 进行 Index Scan 或 Bitmap Index Scan，而非 Seq Scan
