## Context

feat-persist-runtime-logs 在已有 `runtime_logs` 表（`SSoT/schema/migrations/00001_init.sql`）基础上，实现 NanoBotAdapter 调用链中的条件写入逻辑。涉及三个核心设计决策：条件写入策略、PII 脱敏策略、写入失败隔离策略。

关键架构约束（criterion.md §3.5）：Runtime Adapter MUST NOT 承担持久化职责。因此 `nanobot.rs` 仅负责计时（返回 `latency_ms`），条件写入由 Gateway 消息编排层（`bridge_client.rs` 或消息处理 handler）触发。

## Goals / Non-Goals

- Goals:
  - `status=error` 时写入 `runtime_logs`，附带 `latency_ms` 和脱敏 payload
  - 确保 PII（`user_id`、原文消息内容）不落入日志库
  - 写入失败不阻断主链路
- Non-Goals:
  - 正常调用日志写入（明确排除，减少 PII 风险和存储压力）
  - 修改 `runtime_logs` 表结构（表已在 SSoT 中定义，无迁移需求）
  - 日志查询/聚合 API（本提案范围外）

## Decisions

- Decision: **仅 status=error 时写入整行记录**（不写 success 行）
  - Alternatives considered: 写入所有调用行（success 时 payload 置 NULL）— 拒绝，增加存储压力且 success 场景的 latency_ms 对排障价值有限；error 场景已能覆盖性能异常分析需求

- Decision: **字段级白名单脱敏**（仅保留 `session_id`、`event_id`、`runtime_type`、`model` 等排障必需安全字段，丢弃其余全部字段）
  - Alternatives considered: 黑名单 remove（`remove("user_id")` 等）— 灵活但存在新增字段漏放风险，随外部 API 变化需持续维护；白名单一劳永逸，未来新增字段默认不落库，PII 安全边界更强

- Decision: **写入失败静默（`tracing::warn!` + `runtime_log_write_failures_total` 计数器递增），不向主链路传播 Err**
  - Alternatives considered: 写入失败返回 Err 中断处理 — 拒绝，日志系统故障不应导致用户请求失败（BR-063：错误可见性原则下的可接受降级）；计数器供 Phase 4 采集

- Decision: **`latency_ms` 计为包含重试/降级的总耗时**
  - Alternatives considered: 仅记首次请求耗时 — 拒绝，重试耗时对排障超时类错误有明确价值；总耗时能反映用户感知延迟

- Decision: **`error_message` 实施 512 字符截断 + Bearer/Shopify secret 敏感片段移除**
  - Alternatives considered: 不手动处理仅依赖白名单防护 — 拒绝，`error_message` 是 Runtime 返回的自由文本，可能回显用户输入或凭证字符串；截断限制存储压力，正则移除满足 security_policy.md 凭证禁止入日志约束

## Risks / Trade-offs

- 白名单字段覆盖风险：若排障场景新增所需字段，需显式加入白名单；缓解：白名单变更须 Review，初始白名单字段已覆盖主要排障维度（`session_id` / `event_id` / `runtime_type` / `model`）
- latency_ms 精度：仅精确到毫秒；对超时（≈15000ms）场景精度已足够，当前场景无需亚毫秒精度

## Migration Plan

N/A — 无 DB 迁移（`runtime_logs` 表已存在），无 API 变更。

## Open Questions

N/A
