# 实施任务清单

> feat-gateway-db-layer — Gateway DB 连接层。前置依赖 feat-infra-gateway-scaffold 已完成。SSoT 迁移文件已就绪（00001_init.sql + 00002_channel_bindings_unique.sql），本变更不新增迁移文件。

## 0. SSoT 确认（SSoT-first）

- [x] 0.1 确认 `SSoT/schema/migrations/00001_init.sql` 与 `00002_channel_bindings_unique.sql` 已存在，内容包含 5 张核心表 + 全部索引
- [x] 0.2 确认 `SSoT/api/main.tsp` 无需变更（本提案无新 API 端点）
- [x] 0.3 在本地开发数据库执行 `make db-migrate-up`，验证迁移无报错

## 1. Cargo.toml 依赖确认

- [x] 1.1 确认 `gateway/Cargo.toml` 包含 `sqlx` 并启用 features: `postgres`, `uuid`, `runtime-tokio-rustls`, `macros`
- [x] 1.2 确认 `uuid` crate 启用 `v4` feature（应用层生成 UUID）

## 2. DB 连接池实现（Roadmap Task 1）

- [x] 2.1 创建 `gateway/src/db/pool.rs`
  - [x] 2.1.1 使用 `PgPoolOptions::new().max_connections(config.db_max_connections).connect(&config.database_url)` 初始化连接池
  - [x] 2.1.2 实现 `health_check(pool: &PgPool) -> Result<(), AppError>` — 执行 `SELECT 1` 探测连接
  - [x] 2.1.3 连接池初始化失败时输出明确错误（只打印字段名 `DATABASE_URL`，禁止打印连接串值或密码明文），然后 `process::exit(1)`（RISK-006：`security_policy.md` §敏感数据禁止日志）
- [x] 2.2 更新 `gateway/src/db/mod.rs`，pub 导出 `pool` 模块及 `health_check`

## 3. DB 熔断检查实现（Roadmap Task 2）

- [x] 3.1 在 inbound handler（`gateway/src/adapters/inbound.rs` 或等价文件）中实现熔断检查
  - [x] 3.1.1 在 Bearer Token 验证 + payload 解析后、`bot_id` 解析前调用 `health_check(pool)`
  - [x] 3.1.2 `health_check` 失败时：调用 Bridge reply API 向已解析的 `chat_id` 回写文本 `"系统暂时不可用，请稍后重试"`（BR-041、BR-063）
  - [x] 3.1.3 回写后对 mb-adapter 返回 HTTP 503；记录 ERROR 级别日志（含 `db_unavailable` 字段，`cross_cutting_concepts.md` §日志规范）；递增 `db_unavailable_total` Counter
  - [x] 3.1.4 Bridge reply 调用本身失败时：跳过回写（不因回写失败引入 panic），仍返回 503 并额外记录日志
  - [x] 3.1.5 将上述逻辑提取为可复用函数 `db::health_guard(pool, chat_id, bridge_url)`，供后续 `feat-gateway-inbound-gate` 重构时直接调用（M-3 耦合约定：`design.md` §Decisions）
- [x] 3.2 在 `gateway/src/main.rs` 中注册 `PgPool` Extension（供 handler 提取）

## 4. Goose 迁移 CI 集成（Roadmap Task 3）

- [x] 4.1 在 Gateway 启动时查询 `information_schema.tables`，验证 `bots`、`channel_bindings`、`sessions`、`message_events`、`runtime_logs` 5 张表均存在；不满足时输出 ERROR 日志并退出
- [x] 4.2 在 CI workflow（如 `.github/workflows/ci.yml`）中添加步骤：启动测试 PG 容器 → 运行 `goose -dir SSoT/schema/migrations up` → 运行 Gateway 测试

## 5. 开发环境 Seed 脚本（Roadmap Task 4）

- [x] 5.1 创建 `scripts/seed_db.sh`
  - [x] 5.1.1 插入 1 条默认 bot 记录到 `bots` 表（固定 UUID、bot_name: `default-bot`、runtime_type: `nanobot`）
  - [x] 5.1.2 插入对应 channel_bindings 记录（platform: `telegram`，bridge_gateway_name: `default`）
  - [x] 5.1.3 脚本接受 DATABASE_URL 环境变量或命令行参数
- [x] 5.2 在脚本头部添加幂等保护（`INSERT ... ON CONFLICT DO NOTHING`）

## 6. BR-032 函数签名约定落地

- [x] 6.1 在 `gateway/src/db/mod.rs` 顶部注释中明确 BR-032 约定：所有 db 函数签名必须携带 `bot_id: Uuid` 参数
- [x] 6.2 code review checklist 确认：无全表 SELECT/UPDATE/DELETE（无 bot_id 过滤条件）

## 7. 单元测试

- [x] 7.1 为 `health_check()` 编写单元测试（使用 `testcontainers` 或 mock）：PG 可达时返回 Ok；PG 不可达时返回 Err
- [x] 7.2 为 inbound handler 熔断检查编写集成测试：模拟 DB 不可用，验证 HTTP 503 响应、Bridge reply 回写调用（含回写文本）、ERROR 级别日志（含 `db_unavailable` 字段）及 `db_unavailable_total` Counter 递增

## 8. 手动验证

- [x] 8.1 启动 Gateway（DATABASE_URL 指向运行中的 PG）→ `health_check()` 返回 Ok，日志正常
- [x] 8.2 停止 PG → 通过 mb-adapter 发送入站请求（含 chat_id）→ 验证：Gateway 返回 503；Bridge reply API 收到回写请求（文本为"系统暂时不可用，请稍后重试"）；ERROR 级别日志中含 `db_unavailable` 字段；`db_unavailable_total` Counter 递增；重启 PG → 后续请求恢复正常
- [x] 8.3 执行 `bash scripts/seed_db.sh` → `bots` 和 `channel_bindings` 表有记录

## 9. 验证与归档

- [x] specflow validate feat-gateway-db-layer --strict（运行：`node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-db-layer --strict`）
- [x] specflow archive feat-gateway-db-layer --yes（运行：`node design/context-dev/tools/specflow/specflow.mjs archive feat-gateway-db-layer --yes`，由用户执行）
