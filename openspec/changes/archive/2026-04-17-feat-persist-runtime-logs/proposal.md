# Change: Runtime 调用日志条件持久化与 PII 脱敏

## Why

Gateway 当前无任何 `runtime_logs` 写入实现。TAD §10.5 明确要求：

- `runtime_logs` 仅在 `status=error` 时写入行记录（正常调用不落库，减少 PII 风险与存储压力）
- 写入的 `request_payload` / `response_payload` 必须脱敏 PII（移除 `user_id`、原文消息内容）
- 每次错误记录均附带 `latency_ms`（Runtime 调用耗时，用于故障分析）

`runtime_logs` 表已在 `SSoT/schema/migrations/00001_init.sql` 中建立。本提案实现 Gateway 对该表的应用层写入逻辑，完成运行时可观测性闭环（criterion.md §4 数据治理 / runtime_view.md 场景 1/2）。

## What Changes

### 新增功能
- `gateway/src/db/runtime_logs.rs`：条件 INSERT 函数，仅当 `status=error` 时写入 `runtime_logs` 行记录（含 `latency_ms`、脱敏 payload、`error_code`、`error_message`）
- PII 脱敏函数（`sanitize_request_payload` / `sanitize_response_payload`）：从 `serde_json::Value` 中字段级 remove `user_id`、`input_text`（原文消息内容）等 PII 字段

### 修改功能
- `gateway/src/adapters/nanobot.rs`（NanoBotAdapter）：集成 `std::time::Instant` 计时，将 `latency_ms` 连同调用结果一起返回给调用层（adapter 不直接写库，不承担持久化职责，满足 criterion.md §3.5 MUST NOT）
- Gateway 消息编排层（`bridge_client.rs` 或消息处理 handler）：接收 NanoBotAdapter 返回的结果与 `latency_ms`，在 `status=error` 路径触发条件写入

### 技术实现
- 使用 `Instant::now()` 在调用 Runtime 前记录时刻，调用结束（**含所有重试/降级**）后以 `elapsed().as_millis() as i32` 计算总耗时 `latency_ms`；返回 `(Result<..., RuntimeError>, latency_ms)` 给编排层
- PII 脱敏：对 `serde_json::Value` 采用白名单模式，仅保留排障必需安全字段，丢弃其余全部字段（未知字段默认不落库）；`error_message` 额外截断至 512 字符并移除 Bearer Token / Shopify secret 敏感片段
- 写入失败仅记录 `tracing::warn!` 并递增 `runtime_log_write_failures_total` 计数器（对接 Phase 4 Prometheus），不向主链路返回错误

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/runtime-persistence/spec.md` — Runtime 调用日志条件持久化行为规范（条件写入、PII 脱敏、latency_ms、写入失败隔离）

### 涉及的代码
- **新增**：
  - `gateway/src/db/runtime_logs.rs`
- **修改**：
  - `gateway/src/adapters/nanobot.rs`

### 依赖关系
- **依赖**：`feat-runtime-nanobot-adapter`（NanoBotAdapter 实现基础，提供调用链切入点）
- **被依赖**：无

### 风险与注意事项
- 白名单字段覆盖风险：若排障场景新增所需字段，需显式加入白名单；通过 Review 机制和接口变更时同步评估降低遗漏风险
- `runtime_logs` 写入失败不得阻断主链路，避免日志写入降级为业务障碍（BR-063：错误可见性原则下的可接受降级）

### 验证标准
- ✅ 正常调用（status=success）不向 `runtime_logs` 写入任何行记录
- ✅ 错误调用（status=error）写入 `runtime_logs`，`request_payload` 中无 `user_id` 和原文消息内容
- ✅ 错误调用写入的 `runtime_logs` 行中 `latency_ms` 有非负整数值（毫秒）
- ✅ `runtime_logs` 写入失败时，主消息处理链路（回写用户）不受影响，`tracing::warn!` 出现于日志

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §4 数据治理：runtime_logs 仅 error 写入且脱敏 PII；§3.7 DB 持久化 MUST |
| db | `.context/db/schema_design.md` | runtime_logs 表结构（latency_ms、request_payload JSONB、response_payload JSONB） |
| architecture | `.context/architecture/runtime_view.md` | 场景 1/2：Gateway → Runtime 调用链路与错误处理流程 |
| architecture | `.context/architecture/security_policy.md` | runtime_logs 14 天生命周期限制；PII 数据处置策略（Out 范围：过期清理非本提案职责） |
| architecture | `.context/architecture/cross_cutting_concepts.md` | latency_ms 与可观测指标体系对齐 |
| architecture | `.context/architecture/tech_stack.md` | Gateway Rust 依赖（serde_json、sqlx、tokio） |
| domain | `.context/domain/business_rules.md` | BR-070 消息数据最小化；BR-071 PII 脱敏 SHOULD；BR-063 错误可见性 MUST |
