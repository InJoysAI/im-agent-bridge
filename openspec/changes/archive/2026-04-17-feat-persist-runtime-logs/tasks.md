# 实施任务清单

> Change: feat-persist-runtime-logs — Runtime 调用日志条件持久化与 PII 脱敏
> SSoT：runtime_logs 表已在 SSoT/schema/migrations/00001_init.sql 中定义，本提案不新增 DB 迁移，不修改 SSoT/api/main.tsp。
> 依赖：feat-runtime-nanobot-adapter（NanoBotAdapter 实现基础已就位）

## 1. SSoT 预检

- [x] 1.1 确认 `SSoT/schema/migrations/00001_init.sql` 中 `runtime_logs` 表结构满足需求（含 `latency_ms INTEGER`、`request_payload JSONB`、`response_payload JSONB`、`status VARCHAR(32)`、`error_code VARCHAR(64)`、`error_message TEXT`）
- [x] 1.2 确认 `SSoT/api/main.tsp` 无需修改（runtime_logs 写入为内部逻辑，不暴露新 API 端点）

## 2. db/runtime_logs.rs — 条件 INSERT 函数

- [x] 2.1 在 `gateway/src/db/runtime_logs.rs` 实现 `insert_runtime_log` 函数
  - 入参：`pool`、`event_id`、`bot_id`、`runtime_type`、`status`（`"success"` | `"error"`）、`error_code`（`Option<&str>`）、`error_message`（`Option<&str>`）、`latency_ms`（`i32`）、`request_payload`（`Option<serde_json::Value>`）、`response_payload`（`Option<serde_json::Value>`）
  - 仅当 `status=error` 时由调用方传入 payload；函数内部接受 `None` payload 作为 safety fallback
- [x] 2.2 写入失败时使用 `tracing::warn!` 记录，不向调用方返回 `Err`（主链路不受影响）；同时递增 `runtime_log_write_failures_total` 计数器（对接 Phase 4 Prometheus，见 cross_cutting_concepts.md）

## 3. PII 脱敏函数

- [x] 3.1 实现 `sanitize_request_payload(payload: serde_json::Value) -> serde_json::Value`
  - 采用白名单模式：仅保留 `session_id`、`event_id`、`runtime_type`、`model` 等排障必需安全字段
  - 丢弃其余全部字段（含 `user_id`、`input_text` 及未来可能新增的敏感字段）
- [x] 3.2 实现 `sanitize_response_payload(payload: serde_json::Value) -> serde_json::Value`
  - 采用白名单模式：仅保留 `error_type`、`error_message`、`status_code` 等安全字段
  - 丢弃其余全部字段（防止原始用户消息内容回显至日志）
  - `error_message` 额外处理：
    - 截断至最多 **512 字符**，超出部分替换为 `"...[truncated]"`
    - 使用正则模式移除敏感片段，替换为 `"[REDACTED]"`（见 security_policy.md）：
      - Bearer Token：`Bearer\s+[A-Za-z0-9._\-]+`
      - Shopify secret/token：`shp[a-zA-Z]+_[0-9a-fA-F]{32,64}`
- [x] 3.3 单元测试覆盖：
  - [x] `user_id` 字段不出现在输出中
  - [x] `input_text` 字段不出现在输出中
  - [x] `session_id`、`event_id`、`runtime_type` 等白名单字段保留
  - [x] 白名单外的任意未知字段不出现在输出中
  - [x] `error_message` 超过 512 字符时被截断，末尾包含 `"...[truncated]"`
  - [x] `error_message` 中的 Bearer Token 模式被替换为 `"[REDACTED]"`
  - [x] 空/null payload 输入不 panic

## 4. NanoBotAdapter — 计时集成

- [x] 4.1 在 `gateway/src/adapters/nanobot.rs` Runtime 调用前插入 `let start = std::time::Instant::now();`
- [x] 4.2 调用结束后（含所有重试/降级后）计算 `let latency_ms = start.elapsed().as_millis() as i32;`（总耗时口径）
- [x] 4.3 确保 NanoBotAdapter 将 `latency_ms` 连同调用结果一起返回给调用层；adapter 内部不直接调用 `insert_runtime_log`（满足 criterion.md §3.5：Runtime Adapter MUST NOT 承担持久化职责）

## 5. 消息编排层 — 条件写入集成

- [x] 5.1 在 Gateway 消息编排层（`bridge_client.rs` 或消息处理 handler）接收 NanoBotAdapter 返回的 `(result, latency_ms)`
- [x] 5.2 成功路径（status=success）：跳过 `insert_runtime_log`，不写 `runtime_logs`
- [x] 5.3 失败路径（status=error）：调用脱敏函数处理 payload，再调用 `insert_runtime_log`（`error_code` 填入对应错误码：`RUNTIME_TIMEOUT` / `RUNTIME_UNAVAILABLE` / `RUNTIME_BAD_RESPONSE`）
- [x] 5.4 确认 `insert_runtime_log` 失败不向上传播（由 step 2.2 保证）；失败时 `runtime_log_write_failures_total` 计数器递增（对接 Phase 4 Prometheus）

## 6. 测试

- [x] 6.1 单元测试（`cargo test`）
  - [x] `sanitize_request_payload`：验证 `user_id` / `input_text` 被移除，元数据字段保留
  - [x] `sanitize_response_payload`：验证敏感内容被移除
  - [x] 条件写入逻辑（mock sqlx pool）：success 路径不触发 INSERT；error 路径触发 INSERT 含脱敏 payload
  - [x] adapter 模块层调用验证：`nanobot.rs` 内无任何对 `db::runtime_logs` 的直接依赖
- [x] 6.2 集成测试（testcontainers + PostgreSQL）
  - [x] 成功调用场景：`runtime_logs` 中无新增行
  - [x] Runtime 超时场景：`runtime_logs` 新增一行，`status='error'`、`latency_ms > 0`、`request_payload` 无 `user_id` / `input_text`
- [x] 6.3 手动验证
  - [x] 注入 Runtime 不可达，查询 `runtime_logs` 确认 payload 已脱敏，`latency_ms` 有值
  - [x] 正常消息处理，查询 `runtime_logs` 确认无新增行

## 7. 验证

- [x] `cargo test` 全部通过（含新增单元测试）
- [x] `node design/context-dev/tools/specflow/specflow.mjs validate feat-persist-runtime-logs --strict` 通过
- [x] 手动确认：`runtime_logs` 写入失败不阻断主消息处理链路

## 8. 归档

- [x] `node design/context-dev/tools/specflow/specflow.mjs archive feat-persist-runtime-logs --yes`
