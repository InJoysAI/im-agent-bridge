# Change: Gateway DB 连接层

## Why

IM Agent Bridge 的 Gateway 需要与 PostgreSQL 建立可靠的连接层，作为后续所有业务查询（Channel 解析、Session 管理、消息持久化）的基础。

当前状态：Gateway Rust 骨架（`feat-infra-gateway-scaffold`）已完成，环境变量 `DATABASE_URL` 已在 `config.rs` 声明，但连接池尚未初始化，DB 层完全缺失。

**核心业务需求**：
- DB 不可用时系统必须快速失败（短路熔断 → HTTP 503），遵循"宁可报错不可错乱"原则（BR-040/BR-041）
- Goose 迁移（`00001_init.sql` + `00002_channel_bindings_unique.sql`）已就绪，需在 CI/启动时验证全部 5 张表 + 索引正确创建
- 确立 `bot_id` 贯穿所有 DB 函数签名的架构规范（BR-032），防止多 Bot 数据越界

## What Changes

### 新增功能
- `gateway/src/db/pool.rs`：sqlx `PgPool` 初始化（`max_connections=100`）+ `health_check()` 函数
- DB 熔断检查（handler 层）：在 inbound payload 解析后、`bot_id` 解析前执行 `health_check(pool)`；不可用时调用 Bridge reply API 向 `chat_id` 回写 `"系统暂时不可用，请稍后重试"`，对 mb-adapter 返回 HTTP 503，记录 ERROR 级别系统告警日志（含 `db_unavailable` 字段），递增 `db_unavailable_total` Counter（BR-041、BR-063；日志级别权威：`cross_cutting_concepts.md` §日志规范第 26 行）
- Goose 迁移验证机制：Gateway 启动时或 CI 中验证迁移状态（5 张表 + 全部索引）
- `scripts/seed_db.sh`：开发/测试环境录入默认 bot 实例与 channel_bindings 映射数据
- BR-032 规范文档：确立所有 DB 函数签名必须携带 `bot_id: Uuid` 参数的约定

### 技术实现
- 使用 `sqlx::postgres::PgPoolOptions` 初始化连接池，从 `config.rs` 读取 `DATABASE_URL` 和 `DB_MAX_CONNECTIONS`（默认 100）
- `PgPool` 通过 `axum::Extension` 注入；熔断探测在 inbound handler 层（payload 解析后）执行 `health_check(pool)`；失败时：① 调用 Bridge reply API 回写用户提示 `"系统暂时不可用，请稍后重试"`，② 向调用方返回 503，③ 记录 ERROR 级别日志（`cross_cutting_concepts.md` §日志规范），④ 递增 `db_unavailable_total` Counter
- Goose 迁移验证：通过 `sqlx` 查询 `information_schema.tables` 确认 5 张表存在；或集成 `goose status` 命令到 CI workflow
- 所有 DB 函数签名约定：`async fn <fn>(pool: &PgPool, bot_id: Uuid, ...) -> Result<_, AppError>`

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/db-connection/spec.md` — DB 连接池、健康检查与熔断行为规范

### 涉及的代码
- **新增**：
  - `gateway/src/db/pool.rs`
  - `gateway/src/db/mod.rs`（更新 pub use）
  - `scripts/seed_db.sh`

- **修改**：
  - `gateway/src/main.rs`（注册 PgPool Extension）
  - `gateway/src/adapters/inbound.rs` 或等价 inbound handler（加入熔断检查 + Bridge reply 回写逻辑）
  - `gateway/src/config.rs`（可选：添加 `db_max_connections` 字段）
  - `gateway/Cargo.toml`（确认 `sqlx` features: `postgres, uuid, runtime-tokio-rustls, macros`）

### 依赖关系
- **依赖**：`feat-infra-gateway-scaffold`（已完成 done）— `config.rs` 环境变量加载、目录结构
- **被依赖**：`feat-gateway-inbound-gate`（需要 DB 熔断 middleware）

### 风险与注意事项
- RISK-004（PostgreSQL 不可用）：短路熔断是核心缓解措施；熔断逻辑本身不应引入 panic，必须使用 `Result` 传播
- RISK-006（凭证泄露）：启动失败错误输出只允许打印字段名，禁止打印 `DATABASE_URL` 连接串或密码明文（`security_policy.md` §敏感数据禁止日志）
- `pool.acquire()` 探测有额外延迟（约 1–5ms），但防止脏写比延迟更重要
- Goose 迁移文件禁止修改（`00001_init.sql` / `00002_channel_bindings_unique.sql` 已执行）；本提案不新增迁移文件
- `/bridge/reply` 接口当前标注为"保留但未实现"（`api_strategy.md` §2.1）；本提案在实现阶段依赖该接口可用。若 `/bridge/reply` 未就绪，"用户可见回写"验收项**不得**判定通过，变更**不可**标记为 done（与大纲依赖声明 `proposal-roadmap.md` §提案3 Out 边界一致）；其余验收项（health_check、503、迁移、BR-032）可独立验证

### 验收标准
- ✅ `health_check()` 在 PG 可达时返回 `Ok(())`
- ✅ PG 停止时，入站请求触发熔断：Gateway 调用 Bridge reply API 向 `chat_id` 回写 `"系统暂时不可用，请稍后重试"`，对 mb-adapter 返回 HTTP 503，记录 ERROR 级别系统告警日志（含 `db_unavailable` 字段），`db_unavailable_total` Counter 递增（BR-041、BR-063）
- ✅ Gateway 启动时 5 张核心表（bots/channel_bindings/sessions/message_events/runtime_logs）+ 全部索引均存在
- ✅ 所有后续 DB 函数签名包含 `bot_id: Uuid` 参数（代码审查确认，BR-032）

### SSoT 状态
- **迁移文件**：`SSoT/schema/migrations/00001_init.sql` + `00002_channel_bindings_unique.sql` 已就绪，**本提案不新增迁移文件**
- **API 合约**：`SSoT/api/main.tsp` 无变更（本提案无新 API 端点）

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | `openspec/proposal-roadmap.md` |
| roadmap_source_supplement | N/A |
| phase | Phase 1 |
| business_goal | sqlx PgPool 初始化 + 健康检查；DB 不可用短路熔断 → 503；Goose 迁移验证 |
| dependencies | `feat-infra-gateway-scaffold`（done） |
| acceptance_criteria | health_check() OK；PG 停止 → 503；5 张表+索引创建成功；BR-032 规范落地 |

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | BR-040/BR-041 熔断约束；BR-032 bot_id 隔离；BR-063 错误可见性 |
| domain | `.context/domain/business_rules.md` | BR-041 DB 不可用处理；BR-063 错误可见性原则 |
| domain | `.context/domain/edge_cases.md` | §5 数据库异常处理流程；§8 系统响应规范（"系统暂时不可用"） |
| db | `.context/db/schema_design.md` | 5 张核心表结构；索引设计 |
| db | `.context/db/migrations_and_ssot.md` | Goose SSoT 规范；禁止裸 DDL |
| db | `.context/db/performance_tuning.md` | max_connections=100 建议；连接池健康检查口径 |
| db | `.context/db/observability.md` | db_unavailable_total 告警阈值；指标落地要求 |
| architecture | `.context/architecture/tech_stack.md` | sqlx SHOULD；tokio MUST；Rust MUST |
| architecture | `.context/architecture/cross_cutting_concepts.md` | DB 不可用日志级别 ERROR；db_unavailable_total Counter |
| architecture | `.context/architecture/api_strategy.md` | 503 语义；/bridge/reply 契约字段与实现状态 |
| architecture | `.context/architecture/security_policy.md` | 敏感凭证禁止日志；DATABASE_URL 禁止明文输出 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-004（PG 不可用）；RISK-006（凭证泄露） |
