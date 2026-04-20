# PostgreSQL 可观测性规范 — IM Agent Bridge

> **Metadata**
> - **Source**: `.context/db/source/IM-Agent-Bridge-TAD.md` (§11.1, §11.2, §11.3, §12.4)
> - **Generated At**: `2026-04-13 18:17`
> - **Generator**: `Context-Agent v1.0`

---

## 📊 关键指标与阈値

### TAD 明确指标

TAD §11.2 定义了以下业务指标（由 Gateway 上报）；`db_unavailable_total` 来自 TAD §12.4：

| 维度 | 指标 | 告警阈値 | DB 关联 |
|------|------|---------|---------|
| **可用性** | `db_unavailable_total` | 任何增量 | PostgreSQL 不可用熔断计数（TAD §12.4） |
| **业务** | `messages_received_total` | - | 入站消息计数 |
| **业务** | `messages_replied_total` | - | 回写成功计数 |
| **业务** | `runtime_call_success_total` / `runtime_call_timeout_total` | - | Runtime 健康 |
| **业务** | `mcp_call_success_total` / `mcp_call_error_total` | - | MCP 健康 |
| **业务** | `reply_write_success_total` / `reply_write_error_total` | - | 回写健康 |
| **业务** | `rate_limited_total` | - | 限流计数（§12.7，5 msg/sec/chat_id） |

### PostgreSQL 运维指标（最佳实践，非 TAD 直接定义）

| 维度 | 指标 | 建议阈値 | 说明 |
|------|------|---------|------|
| **可用性** | PostgreSQL Up/Down | 状态变化 | 数据库存活 |
| **性能** | Active Connections | > 80% `max_connections` | 连接池即将打满 |
| **性能** | Query Latency（session + config 查询） | > 50ms | 源自端到端 P95 ≤ 5s 目标的推导阈値 |
| **性能** | Lock Wait Time | > 5s | 锁争用 / 死锁 |
| **风险** | Transaction ID Age | > **15 亿** | **极高危：需立即 Vacuum** |
| **存储** | Disk Usage | > 85% | 磁盘空间不足 |

---

## ⚠️ Transaction ID 回卷监控

> **最高危风险**：PostgreSQL 32 位事务 ID 上限约 21 亿，超过 15 亿需立即处理。

```sql
-- 检查 XID 年龄（> 15 亿立即 VACUUM FREEZE）
SELECT
    datname,
    age(datfrozenxid) AS xid_age,
    CASE
        WHEN age(datfrozenxid) > 1500000000 THEN '🔴 CRITICAL'
        WHEN age(datfrozenxid) > 1000000000 THEN '🟡 WARNING'
        ELSE '🟢 OK'
    END AS status
FROM pg_database
ORDER BY xid_age DESC;

-- 紧急处理
vacuumdb --freeze --all --verbose
```

---

## 📈 pg_stat_statements 配置

```ini
# postgresql.conf
shared_preload_libraries = 'pg_stat_statements'
pg_stat_statements.track = all
pg_stat_statements.max = 10000
```

```sql
CREATE EXTENSION IF NOT EXISTS pg_stat_statements;
```

### 高频慢查询分析

重点监控以下高频查询（对应 TAD §5.1 主链路）：

```sql
-- 总耗时 Top 10
SELECT
    LEFT(query, 100) AS query_preview,
    calls,
    ROUND(total_exec_time::numeric, 2) AS total_ms,
    ROUND(mean_exec_time::numeric, 2) AS mean_ms,
    ROUND((100 * total_exec_time / SUM(total_exec_time) OVER())::numeric, 2) AS pct
FROM pg_stat_statements
ORDER BY total_exec_time DESC
LIMIT 10;
```

关键查询延迟目标（确保端到端 P95 ≤ 5s，TAD §12.6）：

| 查询 | 期望延迟 |
|------|---------|
| `channel_bindings` 解析 `bot_id` | < 5ms |
| `sessions` 查找/写入 | < 5ms |
| `message_events` 幂等检查（唯一索引） | < 5ms |
| `message_events` / `runtime_logs` 写入 | < 20ms |

---

## 🧹 Vacuum 健康检查

`message_events` 和 `runtime_logs` 为高写入表，需重点监控 Autovacuum：

```sql
-- Dead Tuple 监控（重点关注 message_events）
SELECT
    schemaname,
    relname,
    n_live_tup,
    n_dead_tup,
    ROUND(100.0 * n_dead_tup / NULLIF(n_live_tup + n_dead_tup, 0), 2) AS dead_pct,
    last_vacuum,
    last_autovacuum
FROM pg_stat_user_tables
WHERE n_dead_tup > 1000
ORDER BY n_dead_tup DESC;

-- 当前 Autovacuum 进程
SELECT pid, datname, relid::regclass AS table_name, phase
FROM pg_stat_progress_vacuum;
```

| 指标 | 阈值 | 处理 |
|------|------|------|
| Dead Tuple % | > 10% | 检查 Autovacuum 是否正常工作 |
| `last_autovacuum` | > 7 天 | 手动触发 `VACUUM` |
| `n_dead_tup` | > 100 万 | 调整 Autovacuum 参数 |

---

## 📝 日志配置

TAD §11.1 要求日志覆盖：消息接入、标准化、`session_id` 生成、Runtime 调用、MCP 调用、回写结果、错误。

PostgreSQL 侧日志配置：

```ini
# postgresql.conf
log_min_duration_statement = 100    -- 记录 > 100ms 的 SQL（对标 < 50ms 目标）
log_checkpoints = on
log_connections = on
log_disconnections = on
log_line_prefix = '%m [%p] %u@%d '
log_statement = 'ddl'               -- 记录所有 DDL（安全审计）
```

---

## 🔗 Trace 集成

TAD §11.3 建议为每次消息处理生成统一 `trace_id`，贯穿 Bridge → Gateway → Runtime → MCP → 回写（TAD 未指定具体锚点字段）。

DB 层面建议（最佳实践，非 TAD 直接要求）：
- `message_events.event_id` 可用作 DB 侧关联标识，便于跨组件关联查询
- `runtime_logs.event_id` FK 关联到 `message_events`，实现 Runtime 调用追踪
- 日志中建议携带 `event_id` 字段（如项目采用 `trace_id`，则建议将 `event_id` 映射到 `trace_id`）

---

## 🔗 采集与告警集成

| 组件 | 用途 |
|------|------|
| `postgres_exporter` | PostgreSQL 系统指标采集 |
| Prometheus | 指标存储与告警规则 |
| Grafana | 可视化仪表板 |

```yaml
# docker-compose.yml（MVP 参考）
services:
  postgres_exporter:
    image: prometheuscommunity/postgres-exporter
    environment:
      DATA_SOURCE_NAME: "postgresql://monitor_user:password@postgres:5432/im_agent_db?sslmode=disable"
    ports:
      - "9187:9187"
```

### 关键告警规则

```yaml
# prometheus_rules.yml
groups:
  - name: im_agent_db
    rules:
      - alert: PostgreSQLDown
        expr: pg_up == 0
        for: 1m
        labels:
          severity: critical

      - alert: PostgreSQLHighConnections
        expr: pg_stat_activity_count / pg_settings_max_connections > 0.8
        for: 5m
        labels:
          severity: warning

      - alert: PostgreSQLTransactionIdWraparound
        expr: pg_database_age > 1500000000
        for: 0m
        labels:
          severity: critical

      - alert: IMAgentDBUnavailable
        expr: increase(db_unavailable_total[1m]) > 0
        for: 0m
        labels:
          severity: critical

      # 数据保留期与清理监控（最佳实践推导，非 TAD 直接约束）
      # 保留期来源：db/schema_design.md 数据治理 / db/security_hardening.md（message_events=30天；runtime_logs=14天）
      # 阈值（5GB / 1h 清理失败）为最佳实践推导，应结合实际数据量调整
      - alert: MessageEventsRetentionCleanupFailed
        expr: increase(db_retention_cleanup_errors_total{table="message_events"}[1h]) > 0
        for: 0m
        labels:
          severity: warning
        annotations:
          summary: "message_events 保留期清理任务失败（30天规则），需人工介入"

      - alert: RuntimeLogsRetentionCleanupFailed
        expr: increase(db_retention_cleanup_errors_total{table="runtime_logs"}[1h]) > 0
        for: 0m
        labels:
          severity: warning
        annotations:
          summary: "runtime_logs 保留期清理任务失败（14天规则），需人工介入"

      - alert: MessageEventsTableSizeGrowth
        # 阈值 5GB 为最佳实践推导（MVP 单节点参考值），TAD 未直接规定
        expr: pg_table_size{relname="message_events"} > 5e9
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "message_events 表大小超过 5GB，请检查保留期清理是否正常执行"
```

---

## AI 引用指南

当 AI 配置数据库监控时：
1. 优先监控 Transaction ID Age（最高危）
2. 启用 `pg_stat_statements`，重点追踪 session/channel_bindings 查询延迟
3. `message_events` 为高写入表，Autovacuum 需激进配置
4. `event_id` 是 DB 侧 trace 锚点，日志中必须携带
5. PostgreSQL 不可用时 Gateway 熔断触发 `db_unavailable_total` 告警
