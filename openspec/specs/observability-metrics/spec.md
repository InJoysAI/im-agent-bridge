# observability-metrics Specification

## Purpose
TBD

## Requirements
### Requirement: Prometheus Counter 定义
系统必须（MUST）定义 TAD §11.2 规定的全部业务指标，使用 `prometheus-client` crate 的 `Counter` 类型（全部为 Counter，无 Gauge），指标名与 TAD §11.2 表严格一致。

#### Scenario: 全部指标已注册到 Registry
- **WHEN** Gateway 启动并初始化 Prometheus Registry
- **THEN** Registry 中包含以下 Counter：`messages_received_total`、`messages_replied_total`、`runtime_call_success_total`、`runtime_call_timeout_total`、`mcp_call_success_total`、`mcp_call_error_total`、`reply_write_success_total`、`reply_write_error_total`、`rate_limited_total`、`db_unavailable_total`、`runtime_log_write_failures_total`
- **AND** 所有 Counter 初始值为 0

#### Scenario: 指标名称符合 Prometheus 命名约定
- **WHEN** 查看 `GET /metrics` 输出
- **THEN** 所有 Counter 使用 `_total` 后缀
- **AND** 指标名仅包含 `[a-zA-Z0-9_:]` 字符

---

### Requirement: GET /metrics 端点暴露 Prometheus 指标
系统必须（MUST）在 axum Router 注册 `GET /metrics` 路由，返回 Prometheus exposition format，状态码 200，Content-Type 为 `text/plain; version=0.0.4; charset=utf-8`。

#### Scenario: /metrics 端点返回 200 + Prometheus 格式
- **WHEN** 客户端发送 `GET /metrics` 请求
- **THEN** 返回 HTTP 200
- **AND** Content-Type 为 `text/plain; version=0.0.4; charset=utf-8`（或 `application/openmetrics-text`）
- **AND** 响应体包含 `# HELP` 和 `# TYPE` 注释行
- **AND** 每个 Counter 以 `指标名 值` 格式输出

#### Scenario: /metrics 端点不要求 Bearer Token 认证
- **WHEN** 客户端发送不携带 `Authorization` 头的 `GET /metrics` 请求
- **THEN** 返回 HTTP 200（不返回 401）
- **AND** 正常输出 Prometheus 指标

---

### Requirement: /metrics 端点网络隔离
系统必须（MUST）确保 `/metrics` 端点仅在 Internal Server 私有内网内可访问（与 `/health` 同策略；`/health` 端点由 `feat-infra-gateway-scaffold`（Phase 0 提案 1）引入，同端口 `:8080` 无鉴权），不可从公网到达。`criterion.md` :109 的 MUST「接收 Bridge 入站消息并校验 Bearer Token」适用于业务入站端点 `POST /gateway/inbound`，不适用于运维/健康检查端点。

#### Scenario: /metrics 与 /health 同端口同网络策略
- **WHEN** `/metrics` 路由注册到 Gateway 主 Router（`:8080`）
- **THEN** `/metrics` 与 `/health` 共享同一 listener 地址
- **AND** 网络隔离由部署拓扑保证（`deployment_view.md` :291 "Gateway 禁止公网暴露"）
- **AND** `/metrics` 响应中不包含敏感信息（Bearer Token / chat_id / user_id 等不出现在指标标签中）

#### Scenario: 部署验收——非内网地址不可访问 /metrics
- **WHEN** 从非 Internal Server 私有网络地址尝试访问 `GET :8080/metrics`
- **THEN** 连接被网络层拒绝（端口不可达）
- **AND** 无 HTTP 响应返回

---

### Requirement: 入站消息计数
系统必须（MUST）在每条入站消息到达 Gateway 时递增 `messages_received_total` Counter。

#### Scenario: 成功接收入站消息时递增 Counter
- **WHEN** Gateway 收到一条合法的入站文本消息并进入主处理链路
- **THEN** `messages_received_total` 递增 1

#### Scenario: 重复消息不重复递增
- **WHEN** 同一消息因幂等去重被返回 `ignored_duplicate`
- **THEN** `messages_received_total` 不递增（该消息未真正进入处理链路）

---

### Requirement: Runtime 调用成功/超时计数
系统必须（MUST）在 Runtime 调用完成后，根据结果递增 `runtime_call_success_total` 或 `runtime_call_timeout_total`。

#### Scenario: Runtime 调用成功
- **WHEN** NanoBotAdapter 调用 Runtime 并在 15s 内收到有效响应
- **THEN** `runtime_call_success_total` 递增 1

#### Scenario: Runtime 调用超时
- **WHEN** NanoBotAdapter 调用 Runtime 超过 15s 无响应
- **THEN** `runtime_call_timeout_total` 递增 1
- **AND** `runtime_call_success_total` 不递增

---

### Requirement: 回写成功/失败计数
系统必须（MUST）在 Bridge 回写完成后递增 `reply_write_success_total` 或 `reply_write_error_total`。

#### Scenario: 回写成功
- **WHEN** Gateway 成功将回复写入 Bridge（HTTP 200/409）
- **THEN** `reply_write_success_total` 递增 1
- **AND** `messages_replied_total` 递增 1

#### Scenario: 回写最终失败
- **WHEN** Gateway 回写 Bridge 经过 3 次重试后仍失败
- **THEN** `reply_write_error_total` 递增 1
- **AND** `messages_replied_total` 不递增

---

### Requirement: 限流计数
系统必须（MUST）在入站请求被限流时递增 `rate_limited_total`。

#### Scenario: 超过 5 msg/sec/chat_id 触发限流计数
- **WHEN** 同一 `chat_id` 在 1 秒内第 6 条消息被 Token Bucket 拒绝
- **THEN** `rate_limited_total` 递增 1
- **AND** 返回 HTTP 429

---

### Requirement: DB 不可用计数
系统必须（MUST）在检测到 PostgreSQL 不可用时递增 `db_unavailable_total`。

#### Scenario: PostgreSQL 连接失败时递增
- **WHEN** Gateway 尝试连接 PostgreSQL 并失败（熔断触发 503）
- **THEN** `db_unavailable_total` 递增 1

---

### Requirement: MCP Counter 预定义（仅注册）
系统必须（MUST）预定义 `mcp_call_success_total` 和 `mcp_call_error_total`，但 Gateway 侧**仅注册不埋点**（MCP 调用发生在 Runtime 内部，Gateway 无法观测）。后续由 Runtime 侧独立 change 承接有效计数。

#### Scenario: MCP Counter 存在但值为零
- **WHEN** 查看 `GET /metrics` 输出
- **THEN** `mcp_call_success_total` 和 `mcp_call_error_total` 存在
- **AND** 值均为 0（Gateway 侧不直接递增）

---

### Requirement: runtime_log 写入失败计数
系统必须（MUST）在 `runtime_logs` 写入失败时递增 `runtime_log_write_failures_total`（由 `feat-persist-runtime-logs` 引入，本提案统一注册）。

#### Scenario: runtime_logs 写入异常时递增
- **WHEN** `runtime_logs` INSERT 抛出异常（如 DB 短暂不可用）
- **THEN** `runtime_log_write_failures_total` 递增 1
- **AND** 主链路不阻断（回写流程继续）
