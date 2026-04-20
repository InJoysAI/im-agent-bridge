# 迁移与 SSoT 约束规范 — IM Agent Bridge

> **Metadata**
> - **Source**: `.context/db/source/IM-Agent-Bridge-TAD.md` (§14 目录结构, §8.2 建表示例)
> - **Generated At**: `2026-04-13 18:17`
> - **Generator**: `Context-Agent v1.0`

---

## 🎯 SSoT 原则

> **Single Source of Truth**：`SSoT/schema/migrations/` 是数据库 Schema 的**唯一真相源**（TAD §14）。

### 禁止行为

| ❌ 禁止 | ✅ 正确做法 |
|--------|-----------|
| 直接在数据库手动执行 DDL | 创建 Goose SQL 迁移文件到 `SSoT/schema/migrations/` |
| 在生产库手动执行 DDL | 通过 CI/CD Pipeline 执行 `goose up` |
| 多人同时修改 Schema | PR Review 合并迁移文件后执行 |
| 修改已执行的迁移文件 | 创建新的迁移文件进行变更 |

---

## 🔧 Goose 工作流

### 标准流程

```mermaid
flowchart LR
    A[创建迁移文件] --> B[goose create]
    B --> C[编写 SQL]
    C --> D[Review 迁移 SQL]
    D --> E[PR 合并]
    E --> F[CI: goose up]
    F --> G[验证 Schema]
```

### 常用命令

```bash
# 创建新迁移文件（SQL 格式）
goose -dir SSoT/schema/migrations create <migration_name> sql

# 应用迁移（开发环境）
goose -dir SSoT/schema/migrations \
  postgres "postgres://user:pass@localhost:5432/dev?sslmode=disable" up

# 查看迁移状态
goose -dir SSoT/schema/migrations \
  postgres "postgres://..." status

# 回滚最近一次迁移（仅开发环境）
goose -dir SSoT/schema/migrations \
  postgres "postgres://..." down

# 使用环境变量（推荐）
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING="postgres://user:pass@localhost:5432/dev?sslmode=disable"
export GOOSE_MIGRATION_DIR=SSoT/schema/migrations
goose up
```

### 迁移文件格式

```sql
-- +goose Up
CREATE TABLE bots (
    id UUID PRIMARY KEY,
    bot_name VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(128) NOT NULL,
    runtime_type VARCHAR(32) NOT NULL,
    runtime_endpoint TEXT NOT NULL,
    is_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- +goose Down
DROP TABLE bots;
```

### 初始迁移文件（TAD §14 参考）

TAD §14 目录结构中已有 `SSoT/schema/migrations/00001_init.sql` 占位，初始迁移包含 TAD §8.2 全部建表 DDL + §8.3 全部索引 DDL：

```
SSoT/schema/migrations/
└── 00001_init.sql    # 包含 bots/channel_bindings/sessions/message_events/runtime_logs 建表 + 索引
```

---

## 🔄 零停机迁移策略

### Expand-Contract 模式

| 阶段 | 操作 | 目的 |
|------|------|------|
| **Expand** | 添加新列/表（可空或有默认值） | 新旧代码兼容 |
| **Migrate** | 后台数据迁移 | 填充新字段 |
| **Switch** | 应用代码切换到新字段 | 读写新字段 |
| **Contract** | 删除旧列/表 | 清理 |

### 示例：为 `sessions` 添加新字段

```sql
-- Phase 1: Expand（00002_sessions_add_context.sql）
-- +goose Up
ALTER TABLE sessions ADD COLUMN context_metadata JSONB;

-- +goose Down
ALTER TABLE sessions DROP COLUMN context_metadata;
```

---

## ✅ 向后兼容变更（Safe）

| 变更类型 | 安全性 | 说明 |
|---------|--------|------|
| 添加可空列 | ✅ Safe | 不影响现有数据 |
| 添加有默认值的列 | ✅ Safe | 需注意大表性能 |
| 添加新表 | ✅ Safe | 无影响 |
| `CREATE INDEX CONCURRENTLY` | ✅ Safe | 不阻塞读写 |
| 添加外键约束 | ⚠️ Caution | 需 `NOT VALID` + `VALIDATE` 分步执行 |

---

## ❌ 生产环境禁止操作

| 操作 | 风险 | 替代方案 |
|------|------|---------| 
| `DROP TABLE` / `DROP COLUMN` | 数据丢失 | 先 Rename + 保留 30 天后再 Drop |
| `ALTER TABLE RENAME` | 应用中断 | Expand-Contract 模式 |
| `CREATE INDEX`（非 CONCURRENTLY） | 锁表 | `CREATE INDEX CONCURRENTLY` |
| 修改列类型 | 全表锁 | 添加新列 + 数据迁移 |
| `TRUNCATE` | 数据丢失 | 禁止，使用 `DELETE WHERE` + Autovacuum |

---

## 📋 迁移 PR Checklist

```markdown
## 迁移 PR Checklist

- [ ] 创建了 Goose 迁移文件（`SSoT/schema/migrations/`）
- [ ] 迁移文件包含 `-- +goose Up` 和 `-- +goose Down`
- [ ] Review 迁移 SQL 内容
- [ ] 验证向后兼容性（不影响在运行的 Gateway）
- [ ] 大表索引变更使用 `CREATE INDEX CONCURRENTLY`
- [ ] 外键约束使用 `NOT VALID` + `VALIDATE` 分步
- [ ] 回滚脚本已准备（Down 迁移可执行）
- [ ] 迁移文件已更新 `.context/db/schema_design.md`（如有结构变更）
```

---

## AI 引用指南

当 AI 生成数据库变更时：
1. **禁止直接输出裸 DDL** — 必须创建 Goose 迁移文件到 `SSoT/schema/migrations/`
2. 遵循 Expand-Contract 模式，避免零停机风险
3. 索引变更使用 `CREATE INDEX CONCURRENTLY`
4. 不安全操作（DROP/RENAME）需分阶段或人工确认
5. 初始 Schema 在 `SSoT/schema/migrations/00001_init.sql` 中维护

---

## 📋 实现决策记录

记录超出 TAD §8 原文范围、在实现阶段明确新增的 Schema 决策。

### IMPL-001 — `idx_channel_bindings_lookup` 索引

| 项 | 内容 |
|----|------|
| **状态** | ✅ 已决策并合并进 `00001_init.sql` |
| **决策日期** | 2026-04-13 |
| **TAD 依据** | §6.1.1 明确：Gateway 按 `platform + bridge_gateway_name + bridge_channel_name` 查 `channel_bindings` 解析 `bot_id` |
| **§8.3 状态** | 未列出（§8.3 仅给出 `idx_channel_bindings_bot_platform (bot_id, platform)`，与主查询路径不匹配） |
| **决策内容** | 新增 `idx_channel_bindings_lookup ON channel_bindings (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''))` |
| **COALESCE 原因** | `bridge_channel_name` 可为 NULL；降级匹配路径（仅 platform+gateway）需与有 channel_name 的路径共用同一索引扫描 |
| **影响文件** | `SSoT/schema/migrations/00001_init.sql`、`.context/db/schema_design.md`、`.context/db/performance_tuning.md` |
| **后续建议** | 下一轮 TAD 修订时将此索引补入 §8.3 |
