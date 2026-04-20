# PostgreSQL 安全加固规范 — IM Agent Bridge

> **Metadata**
> - **Source**: `.context/db/source/IM-Agent-Bridge-TAD.md` (§10.2, §10.3, §10.4, §10.5)
> - **Generated At**: `2026-04-13 18:17`
> - **Generator**: `Context-Agent v1.0`

---

> **⚠️ 权威说明**
> 本文件为 **DB 视角**安全约束，聚焦 PostgreSQL 层面。最终权威归档于 `architecture/security_policy.md`。

---

## 🔐 认证加固

TAD §10.2 要求外部通信使用 HTTPS + Bearer Token（Bridge ↔ Gateway 层），PostgreSQL 认证层面 TAD 未明确指定，遵循最佳实践：

```ini
# postgresql.conf
password_encryption = scram-sha-256
```

| 方式 | 安全级别 | 说明 |
|------|---------|------|
| `trust` | ❌ 禁止 | 无密码，任何环境均不使用 |
| `md5` | ❌ 废弃 | 易受重放攻击 |
| `scram-sha-256` | ✅ 推荐 | 现代标准，强制使用 |

---

## 📋 pg_hba.conf 规范

TAD §10.3 权限边界：**只有 Gateway 允许访问 PostgreSQL**，Runtime 不直接访问 DB。

```
# TYPE  DATABASE        USER            ADDRESS                 METHOD

# 本地连接（仅 DBA/运维）
local   all             postgres                                scram-sha-256

# Gateway 服务（Docker 内网，严格白名单）
hostssl im_agent_db     gateway_user    172.20.0.0/16           scram-sha-256

# 监控系统（只读，pg_monitor）
hostssl all             monitor_user    10.0.10.5/32            scram-sha-256

# 禁止所有其他连接
host    all             all             0.0.0.0/0               reject
```

> TAD §13.2 明确 Gateway 与 PostgreSQL 在同一受控 Docker 网络，外部不暴露 PostgreSQL 端口。

---

## 🔒 TLS 配置

TAD §10.2：
- **外部通信（Bridge ↔ Gateway）**：强制 HTTPS
- **内部通信（Gateway ↔ PostgreSQL）**：Docker 内网，MVP 阶段可不强制 TLS，后续建议开启

```ini
# postgresql.conf（生产加固建议）
ssl = on
ssl_cert_file = '/path/to/server.crt'
ssl_key_file  = '/path/to/server.key'
ssl_min_protocol_version = 'TLSv1.2'
```

| 环境 | 要求 | 说明 |
|------|------|------|
| MVP Docker 内网 | 非强制 | Gateway ↔ PG 同受控网络 |
| 生产加固 | 推荐开启 | 防内网嗅探，`hostssl` 替换 `host` |

---

## 👤 最小权限原则

TAD §10.3 明确访问权限边界：

| 角色 | 权限 | 用途 |
|------|------|------|
| `gateway_user` | CONNECT + 表级 CRUD | Gateway 服务主账号 |
| `migration_user` | 临时 DDL 权限 | CI/CD Goose 迁移专用 |
| `monitor_user` | `pg_monitor` | 监控采集 |
| `postgres` | SUPERUSER | 紧急运维，不用于应用 |

```sql
-- Gateway 用户：仅表级 CRUD，禁止 DDL 和 TRUNCATE
GRANT CONNECT ON DATABASE im_agent_db TO gateway_user;
GRANT USAGE ON SCHEMA public TO gateway_user;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO gateway_user;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO gateway_user;
REVOKE TRUNCATE ON ALL TABLES IN SCHEMA public FROM gateway_user;

-- 迁移用户：临时授予 DDL，迁移完成后收回
-- （由 CI/CD 控制，不长期存在）

-- 监控用户：只读系统视图
GRANT pg_monitor TO monitor_user;
```

---

## 🔏 敏感数据处理（TAD §10.5）

TAD §10.5 明确的数据最小化策略：

| 字段 | 规则 | 原因 |
|------|------|------|
| `message_events.input_text` / `output_text` | **截断至 512 字符**落库，不作为业务数据源 | 仅用于短期排障 |
| `runtime_logs.request_payload` / `response_payload` | **仅 status=error 时写入**，且必须**脱敏 PII**（移除 user_id、原文内容） | 防止 PII 泄露 |
| Telegram Token | 环境变量或 Secret Manager，不入库 | 凭证安全 |
| Bridge Bearer Token | 环境变量，不入库 | 凭证安全 |
| PostgreSQL 密码 | 环境变量，不入库 | 凭证安全 |
| Shopify MCP client_id/secret | 由 MCP 实例 `.env` 注入，PostgreSQL 不存 | TAD ADR-007 |

### 访问控制（TAD §10.5）

- `input_text` / `output_text` / `request_payload` / `response_payload` 仅允许系统开发者/运维级别访问，**不对外暴露**
- 日志查询应记录审计日志（谁在何时查询了哪些数据）

### 数据保留期

| 表 | 保留期 | 清理方式 |
|----|--------|---------|
| `message_events` | **30 天** | 定时任务或 pg_partman |
| `runtime_logs` | **14 天** | 定时任务或 pg_partman |
| `sessions` | 无自动过期 | 按 `updated_at` 手工清理长期不活跃 |

---

## 📊 安全审计配置

```ini
# postgresql.conf
log_connections = on
log_disconnections = on
log_statement = 'ddl'          -- 记录所有 DDL
log_line_prefix = '%m [%p] %u@%d '
```

---

## AI 引用指南

当 AI 生成数据库安全配置时：
1. 只有 Gateway 可访问 PostgreSQL，不对 Runtime/外部暴露
2. `pg_hba.conf` 严格白名单，禁止 `trust`
3. `runtime_logs` payload 写入前必须脱敏 PII（user_id、原文消息）
4. 凭证（Token/密码/MCP 密钥）一律不入库，走环境变量或 Secret Manager
5. `message_events` / `runtime_logs` 强制保留期（30天/14天）
