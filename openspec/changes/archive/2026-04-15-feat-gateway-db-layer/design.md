## Context

Gateway DB 连接层需要在 Rust/axum 体系内实现 sqlx PgPool 初始化、DB 健康检查与熔断中间件。核心设计决策集中在：熔断探测机制的实现方式、连接池共享方式以及 BR-032 规范的落地约定。

## Goals / Non-Goals

- Goals:
  - 确定熔断探测机制（主动探测 vs 错误计数器）
  - 确定 PgPool 在 axum 中的共享方式
  - 确立 bot_id 函数签名约定（BR-032）

- Non-Goals:
  - 连接池动态扩缩（MVP 固定 max_connections=100）
  - DB 读写分离
  - 具体业务查询（由后续提案实现）

## Decisions

- **Decision: 熔断探测位置 — handler 层最前面（payload 解析后），而非全局 middleware 最顶层**
  - DB 健康检查（`pool.acquire()`）必须放在 Bearer Token 验证 + payload 解析之后、业务处理之前执行。
  - 原因：BR-041 + BR-063 要求向用户返回可见错误提示（"系统暂时不可用，请稍后重试"）；只有在 payload 解析后才能获取 `chat_id`/`platform`，进而调用 Bridge reply API 完成回写。若放在全局 middleware 最顶层，payload 尚未解析，无法获取回写所需字段，导致用户侧静默失败，违反 BR-063。
  - 具体实现：在 inbound handler 中，在 `bot_id` 解析之前先执行 `health_check(pool)`；失败时：① 调用 Bridge reply API 向 `chat_id` 发送 `"系统暂时不可用，请稍后重试"`，② 返回 HTTP 503（给 mb-adapter 感知），③ 记录 ERROR 级别系统告警日志（`cross_cutting_concepts.md` §日志规范），④ 递增 `db_unavailable_total` Counter。
  - **与 inbound-gate 耦合点**：`feat-gateway-inbound-gate` 将对 inbound handler 进行重构（加入 Bearer Token 验证、限流等）。熔断检查的抽象位置约定：提取为 `db::health_guard(pool, chat_id, bridge_url)` 函数，由 inbound handler 在 payload 验证通过后显式调用，不嵌入通用 middleware，确保 inbound-gate 重构时可直接复用，不产生重复逻辑。
  - Alternatives considered：全局 middleware 最顶层探测（实现最简，但违反 BR-041/BR-063，因无 chat_id 上下文，无法回写用户提示）

- **Decision: PgPool 共享方式 — axum::Extension**
  - 在 `main.rs` 中将 `Arc<PgPool>` 或直接 `PgPool`（已内置 Arc）注入为 `Extension`，middleware 和 handler 均通过 `Extension(pool): Extension<PgPool>` 提取。
  - Alternatives considered：全局 `OnceLock<PgPool>`（测试隔离性差）

- **Decision: BR-032 函数签名约定**
  - 所有 `db/` 模块函数统一签名：`async fn xxx(pool: &PgPool, bot_id: Uuid, ...) -> Result<T, AppError>`
  - 约定以注释形式写入 `db/mod.rs`，并在 PR 代码审查时人工核查（不引入 lint 规则，MVP）

- **Decision: 迁移验证方式 — 启动时查询 information_schema**
  - Gateway 启动时执行 `SELECT COUNT(*) FROM information_schema.tables WHERE table_schema='public' AND table_name=ANY(...)` 验证 5 张表存在；不直接运行 `goose status`（避免引入额外 CLI 依赖）
  - CI 中可单独运行 `goose -dir SSoT/schema/migrations status` 验证

## Risks / Trade-offs

- `pool.acquire()` 探测在每次请求中引入约 1–5ms 开销，但在 MVP 流量规模下可接受
- 熔断采用"请求级探测"而非"连接级统计"，PG 瞬断后的第一次请求仍会 503，不影响正确性

## Migration Plan

本提案不引入新 Goose 迁移文件。`SSoT/schema/migrations/` 中已有：
- `00001_init.sql`：5 张表 + 基础索引
- `00002_channel_bindings_unique.sql`：渠道来源唯一约束

后续 Schema 变更须在新迁移文件中追加，禁止修改上述已执行文件。

## Open Questions

- sqlx `PgPoolOptions::connect_timeout`：建议设为 5s，防止 PG 慢启动时 Gateway 卡死——待实现时确认

## 已决策问题（评审后明确）

- **DB 熔断时用户通知职责归属**（评审 P0 澄清项）：
  - 决策：由 Gateway handler 层负责，在 payload 解析后、bot_id 解析前执行 DB 健康检查；失败时：① 调用 Bridge reply API 回写 `"系统暂时不可用，请稍后重试"`，② 返回 HTTP 503，③ 记录 ERROR 级别日志（`cross_cutting_concepts.md` §日志规范），④ 递增 `db_unavailable_total` Counter。
  - 依据：`edge_cases.md §5`（PostgreSQL 不可达 → 向用户返回"系统暂时不可用，请稍后重试"）；`edge_cases.md §8`（系统响应规范：系统不可用 → 用户可见消息）；`cross_cutting_concepts.md` §日志规范（DB 不可用 = ERROR）；BR-041；BR-063。
  - 排除方案：由 mb-adapter 处理 503 后发送回写（职责不清晰，且 mb-adapter 提案尚未定义此兜底行为）。
