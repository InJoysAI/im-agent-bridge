# 实施任务清单

> 预计 1 天。前置 `feat-observability-logging` 已完成，观测基础设施（tracing-subscriber JSON + 脱敏 Layer）已就绪。本提案在此基础上增加 Prometheus 指标暴露。

## 1. 依赖引入与模块骨架
- [x] 1.1 在 `gateway/Cargo.toml` 添加 `prometheus-client = "0.22"` 依赖
- [x] 1.2 在 `gateway/src/observability/` 新建 `metrics.rs`
  - 定义 `Metrics` struct，包含 TAD §11.2 全部 11 个 Counter（`messages_received_total`、`messages_replied_total`、`runtime_call_success_total`、`runtime_call_timeout_total`、`mcp_call_success_total`、`mcp_call_error_total`、`reply_write_success_total`、`reply_write_error_total`、`rate_limited_total`、`db_unavailable_total`、`runtime_log_write_failures_total`）
  - 全部为 Counter 类型（TAD §11.2 无 Gauge）
  - 创建 `Metrics::new(registry: &mut Registry)` 构造函数，将所有 Counter 注册到 `prometheus_client::registry::Registry`
  - 创建 `encode_metrics(registry: &Registry) -> String` 辅助函数
- [x] 1.3 在 `gateway/src/observability/mod.rs` 中 `pub mod metrics;` 导出新模块

## 2. GET /metrics 端点注册
- [x] 2.1 在 `gateway/src/handlers/` 新建 `metrics.rs`
  - 实现 `metrics_handler`（axum handler），从 State 读取共享 `Registry`，调用 `encode_metrics` 返回 `text/plain; version=0.0.4; charset=utf-8`
- [x] 2.2 在 `gateway/src/handlers/mod.rs` 导出 `pub mod metrics;`
- [x] 2.3 在 `gateway/src/main.rs` 修改：
  - 创建 `prometheus_client::registry::Registry` + `Metrics`，包装为 `Arc`
  - 在 Router 注册 `.route("/metrics", get(metrics_handler))`（与 `/health` 同策略：不经过 Bearer Token middleware，同端口 `:8080`）
  - 将 `Arc<Registry>` 和 `Arc<Metrics>` 注入 axum State
  - `/metrics` 响应中不包含敏感标签（Bearer Token / chat_id / user_id 不出现在指标标签中）

## 3. 业务链路埋点
- [x] 3.1 入站路径（`handlers/inbound.rs`）：
  - 消息成功进入处理链路时 `metrics.messages_received_total.inc()`
  - 限流拒绝时 `metrics.rate_limited_total.inc()`
  - DB 不可用触发 503 时 `metrics.db_unavailable_total.inc()`
- [x] 3.2 Runtime 调用路径（`adapters/nanobot.rs`）：
  - 调用成功时 `metrics.runtime_call_success_total.inc()`
  - 超时时 `metrics.runtime_call_timeout_total.inc()`
- [x] 3.3 回写路径（`bridge_client.rs`）：
  - 回写成功（HTTP 200/409）时 `metrics.reply_write_success_total.inc()` + `metrics.messages_replied_total.inc()`
  - 最终失败时 `metrics.reply_write_error_total.inc()`
- [x] 3.4 runtime_logs 写入路径（`db/runtime_logs.rs`）：
  - 写入失败时 `metrics.runtime_log_write_failures_total.inc()`
- [x] 3.5 MCP Counter **仅注册不埋点**（MCP 调用在 Runtime 内部，Gateway 无法观测；后续由 Runtime 侧独立 change 承接有效计数）

## 4. 测试
- [x] 4.1 单元测试
  - [x] 4.1.1 `metrics.rs`：验证 `Metrics::new()` 注册全部 11 个 Counter，`encode_metrics()` 输出包含所有指标名
  - [x] 4.1.2 `metrics.rs`：验证指标递增后 encode 输出值变化
  - [x] 4.1.3 `handlers/metrics.rs`：验证 handler 返回 200 + `text/plain` Content-Type

- [x] 4.2 集成测试
  - [x] 4.2.1 启动 Gateway → `GET /metrics` → 200 + Prometheus 格式
  - [x] 4.2.2 发送入站消息 → `GET /metrics` → `messages_received_total` ≥ 1

- [x] 4.3 手动测试
  - [x] 4.3.1 `curl http://localhost:8080/metrics` 确认输出 Prometheus 格式
  - [x] 4.3.2 发送多条消息后确认 Counter 递增正确
  - [x] 4.3.3 模拟 Runtime 超时后确认 `runtime_call_timeout_total` 递增
  - [x] 4.3.4 确认 `/metrics` 响应中不包含敏感信息（无 Bearer Token / chat_id / user_id 标签）

## 5. SSoT 检查
- [x] 5.1 SSoT 未更改 — 本提案不涉及 DB Schema 或 API 合约变更（`/metrics` 为运维端点，不属于 Bridge↔Gateway/Gateway↔Runtime 业务契约；不修改 `SSoT/api/main.tsp`）
- [x] 5.2 不涉及接口设计（`/metrics` 为 Prometheus 标准格式运维端点，不在 api_strategy.md 范围内；运维端点不纳入 TypeSpec SSoT）
- [x] 5.3 不涉及新错误码（指标端点无业务错误场景）

## 6. 验证与归档
- [x] 6.1 验证所有验收标准
  - [x] 6.1.1 `GET /metrics` → 200 + Prometheus 格式
  - [x] 6.1.2 `/metrics` 输出包含全部 11 个指标名
  - [x] 6.1.3 发消息后 `messages_received_total` 递增
  - [x] 6.1.4 触发限流后 `rate_limited_total` 递增
  - [x] 6.1.5 模拟 DB 不可用后 `db_unavailable_total` 递增
  - [x] 6.1.6 模拟 Runtime 超时后 `runtime_call_timeout_total` 递增
  - [x] 6.1.7 模拟回写失败后 `reply_write_error_total` 递增
  - [x] 6.1.8 `mcp_call_success_total` / `mcp_call_error_total` 存在且值为 0
  - [x] 6.1.9 `cargo test` 全部通过
- [x] 6.2 运行 specflow validate feat-observability-metrics --strict（完整命令：`node design/context-dev/tools/specflow/specflow.mjs validate feat-observability-metrics --strict`），所有检查通过
- [x] 6.3 合并后运行 specflow archive feat-observability-metrics --yes（完整命令：`node design/context-dev/tools/specflow/specflow.mjs archive feat-observability-metrics --yes`）归档提案到 `openspec/changes/archive/`
