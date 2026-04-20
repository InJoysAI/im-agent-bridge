# 性能调优指南 — IM Agent Bridge

> **Metadata**
> - **Source**: `.context/db/source/IM-Agent-Bridge-TAD.md` (§8.3, §12.4)
> - **Generated At**: `2026-04-13 18:17`
> - **Generator**: `Context-Agent v1.0`

---

> ⚠️ TAD 为 MVP 单节点架构设计，未提供具体 PostgreSQL 调优参数。本文在源文档约束范围内给出 MVP 规模适用的建议，标注 `N/A – 源文档未提供` 的小节为最佳实践补充。

---

## 🖥️ 基础设施前置条件

TAD §13 建议 MVP 使用 Docker Compose 单机部署。

| 推荐项 | 建议 | 说明 |
|--------|------|------|
| WAL 存储 | 独立 SSD（条件允许时） | WAL 顺序写入，分离后减少数据文件 I/O 竞争 |
| 文件系统 | XFS 或 ext4，`noatime` 挂载 | 禁用访问时间更新，减少 I/O 开销 |
| 内存 | ≥ 4GB 供 PostgreSQL 使用 | MVP 规模，`shared_buffers = 1GB` 起步 |
| Huge Pages | N/A – 源文档未提供 | MVP 单机规模暂不强制要求 |
| CPU/NUMA | N/A – 源文档未提供 | 单机 Docker Compose 无 NUMA 优化需求 |

---

## 💾 内存配置

> N/A – 源文档未提供具体参数。以下为 MVP 单机 8GB RAM 参考配置：

```ini
# postgresql.conf（MVP 参考，按实际 RAM 调整）
shared_buffers = 2GB               # RAM 的 25%
effective_cache_size = 6GB         # RAM 的 75%
work_mem = 16MB                    # 并发较低，可适当提高
maintenance_work_mem = 512MB       # VACUUM/CREATE INDEX
```

---

## 📝 WAL 与 Checkpoint

> N/A – 源文档未提供具体参数。以下为 MVP 推荐配置：

```ini
checkpoint_timeout = 15min
max_wal_size = 2GB
checkpoint_completion_target = 0.9
wal_compression = on
```

---

## 🧹 Autovacuum 调优

`message_events` 和 `runtime_logs` 为高写入表，默认 autovacuum 配置过于保守：

```ini
# 降低触发阈值，增强清理能力
autovacuum_vacuum_scale_factor = 0.02   # 默认 0.2，调低至 2%
autovacuum_vacuum_cost_limit = 1000     # 默认 200
autovacuum_max_workers = 4             # 默认 3
```

### 监控 Dead Tuple

```sql
-- 检查需要 Vacuum 的表（重点关注 message_events）
SELECT schemaname, relname, n_dead_tup, last_autovacuum
FROM pg_stat_user_tables
WHERE n_dead_tup > 10000
ORDER BY n_dead_tup DESC;
```

---

## 🔌 连接池配置

TAD §13 建议 Gateway 单一服务访问 PostgreSQL，MVP 规模连接数较低。

| 参数 | MVP 建议 | 说明 |
|------|---------|------|
| `max_connections` | 100 | MVP 单实例 Gateway，无需过多连接 |
| PgBouncer | 可选 | MVP 阶段 Gateway 自带连接池（如 `sqlx`/`deadpool-postgres`）可暂不引入 |

> **TAD §12.4 约束**：PostgreSQL 不可用时 Gateway 必须短路熔断（`503 Service Unavailable`），不得在无 DB 时继续处理任何业务请求。这要求连接池具备健康检查能力。

```ini
# postgresql.conf（MVP）
max_connections = 100
```

---

## 📊 关键性能约束

TAD §12.6 明确了端到端超时预算：

| 阶段 | 超时上限 | DB 影响 | 来源 |
|------|---------|---------|------|
| Bot 配置 + Session 查询 | ~200ms（含 Bridge→Gateway 推送） | DB 查询需在毫秒级完成 | TAD §12.6 |
| 消息状态写入 | 异步写入，不阻塞主链路 | 写入延迟对端到端影响较小 | TAD §12.6 |
| **端到端 P95 目标** | **≤ 5s** | DB 相关操作建议 < 50ms（推导値，非 TAD 直接约束） | TAD §12.6 推导 |

### 高频查询优化目标

| 查询 | 索引支撑 | 预期延迟 |
|------|---------|---------|
| `channel_bindings` 按 `platform+gateway+channel` 查询 `bot_id` | `idx_channel_bindings_lookup` | < 5ms |
| `sessions` 按 `bot_id+platform+chat_id` 查询 | `idx_sessions_bot_platform_chat` | < 5ms |
| `message_events` 入站幂等检查 | `uq_message_events_inbound_dedup` | < 5ms |

---

## AI 引用指南

当 AI 生成数据库配置时：
1. MVP 单机规模，`shared_buffers = 25% RAM` 起步
2. 针对 `message_events`/`runtime_logs` 高写入表激进配置 Autovacuum
3. PostgreSQL 不可用时 Gateway **必须**短路熔断，连接池必须配置健康检查
4. DB 查询建议延迟 < 50ms（源自端到端 P95 ≤ 5s 目标的推导，非 TAD 直接约束）
