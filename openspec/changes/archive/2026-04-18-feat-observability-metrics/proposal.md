# Change: Prometheus 指标

## Why

Gateway 作为 Core Layer 的唯一入口，当前已完成结构化日志（`feat-observability-logging`），但缺少量化指标暴露能力。`.context/architecture/cross_cutting_concepts.md` §11.2（:32-46）定义了 10 个业务 Counter 指标（`messages_received_total`、`messages_replied_total`、`runtime_call_success_total`、`runtime_call_timeout_total`、`mcp_call_success_total`、`mcp_call_error_total`、`reply_write_success_total`、`reply_write_error_total`、`rate_limited_total`、`db_unavailable_total`），以及 `feat-persist-runtime-logs` 引入的第 11 个 `runtime_log_write_failures_total`。`.context/db/observability.md`（:14-24）进一步定义了告警阈值与 DB 关联。

当前系统仅有日志可观测，缺乏面向运维的实时量化仪表盘数据源。引入 Prometheus 指标并暴露 `GET /metrics` 端点，运维可直接 scrape 并在 Grafana（Out of scope）等面板中可视化告警。

## What Changes

### 新增功能
- **prometheus-client 集成**：在 `gateway/src/observability/` 中新增 `metrics.rs` 模块，定义全部 TAD §11.2 Counter（11 个，全部为 Counter 类型）
- **`GET /metrics` 端点**：在 axum Router 注册 `/metrics` 路由，输出 Prometheus exposition format（`text/plain; version=0.0.4`）
- **业务链路埋点**：在 inbound handler / Runtime adapter / bridge_client / rate_limiter / DB pool 的关键路径增量递增 Counter

### 技术实现
- 使用 `prometheus-client` crate（Prometheus 官方 Rust 客户端，轻量且无 protobuf 依赖）
- 全局 `Registry` + `Arc<Metrics>` 注入 axum State，各 handler 通过共享引用递增指标
- 指标命名严格与 TAD §11.2 表一致，使用 `_total` 后缀 Counter
- `GET /metrics` 端点不做 Bearer Token 校验，与 `/health` 同策略：同端口 `:8080`，不经 Bearer Token middleware；`criterion.md` :109 的 MUST「接收 Bridge 入站消息并校验 Bearer Token」适用于业务入站端点 `POST /gateway/inbound`，不适用于运维/健康检查端点；网络隔离由部署拓扑保证（Gateway `:8080` 仅 Internal Server 私有内网监听，`deployment_view.md` :291 "Gateway 禁止公网暴露"）；指标标签不包含敏感信息

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/observability-metrics/spec.md` — Prometheus 指标暴露与业务链路埋点规范

### 涉及的代码
- **新增**：
  - `gateway/src/observability/metrics.rs` — Counter 定义 + Registry + encode 函数
  - `gateway/src/handlers/metrics.rs` — `GET /metrics` handler

- **修改**：
  - `gateway/Cargo.toml` — 新增 `prometheus-client` 依赖
  - `gateway/src/main.rs` — 注册 `/metrics` 路由，初始化 `Metrics` 并注入 State
  - `gateway/src/handlers/mod.rs` — 导出 metrics handler
  - `gateway/src/observability/mod.rs` — 导出 metrics 模块
  - `gateway/src/handlers/inbound.rs` — 在消息接收、限流、DB 不可用等路径递增 Counter
  - `gateway/src/adapters/nanobot.rs` — Runtime 调用成功/超时 Counter 递增
  - `gateway/src/bridge_client.rs` — 回写成功/失败 Counter 递增
  - `gateway/src/db/runtime_logs.rs` — runtime_log 写入失败 Counter 递增

### 依赖关系
- **依赖**：`feat-observability-logging`（已完成；日志基础设施与 event_id 贯穿已就绪）
- **被依赖**：`feat-e2e-integration-test`（下游验收）

### 风险与注意事项
- `/metrics` 端点不经过 Bearer Token 认证——合规依据见上述技术实现段落；网络隔离由 `deployment_view.md` :291 部署约束保证；验收时需确认 `/metrics` 不可从非内网地址访问
- `prometheus-client` 需要新增 Cargo 依赖；该 crate 为 Prometheus 官方维护，无 protobuf/编译器额外依赖
- 指标递增操作使用 atomic 计数器，零 mutex 竞争开销
- MCP 调用 Counter（`mcp_call_success_total`、`mcp_call_error_total`）在 Gateway 侧**仅预定义注册**，不保证递增（MCP 调用发生在 Runtime 内部，`.context/architecture/runtime_view.md` 确认 Gateway 无法直接观测）；后续由 Runtime 侧独立 change 承接有效计数（如 Runtime 暴露自身 `/metrics` 或回传 MCP 结果到 Gateway）

### 关联 Context 资产
| Scope | 资产路径 | 关联说明 |
|-------|---------|----------|
| criterion | `.context/criterion.md` | §3.4 Gateway MUST（:109）；§4 安全约束（:192-199） |
| architecture | `.context/architecture/cross_cutting_concepts.md` | §11.2 指标监控（:32-46）：TAD 权威 Counter 定义（11 个）；§11.3 分布式追踪 |
| architecture | `.context/architecture/deployment_view.md` | §部署约束（:291）：Gateway 禁止公网暴露——`/metrics` 网络隔离依据 |
| architecture | `.context/architecture/security_policy.md` | API 安全（:99-109）：内网白名单 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-003（:48）、RISK-004（:60）：指标用于风险缓解告警 |
| architecture | `.context/architecture/tech_stack.md` | Rust MUST；tokio MUST |
| architecture | `.context/architecture/api_strategy.md` | 入站/回写/Runtime 调用路径（确定埋点位置） |
| domain | `.context/domain/risks_and_debt.md` | PD-004（:177）：可观测性基础设施债务 |
| domain | `.context/domain/business_rules.md` | BR-063 错误可见性（:359-365） |
| domain | `.context/domain/edge_cases.md` | DB 不可用/限流/超时等边缘场景 |
| db | `.context/db/observability.md` | TAD 指标与告警联动（:14-24）；采集集成（:163） |

### 验证标准
- ✅ `GET /metrics` 返回 HTTP 200 + Prometheus exposition format（`text/plain; version=0.0.4`）
- ✅ 发送一条入站消息后 `messages_received_total` Counter 递增
- ✅ 完整处理一条消息后 `messages_replied_total`、`runtime_call_success_total`、`reply_write_success_total` 各递增
- ✅ 触发限流后 `rate_limited_total` 递增
- ✅ 模拟 DB 不可用后 `db_unavailable_total` 递增
- ✅ 模拟 Runtime 超时后 `runtime_call_timeout_total` 递增
- ✅ 模拟回写失败后 `reply_write_error_total` 递增
- ✅ MCP Counter 已注册但值保持为 0（Gateway 侧仅预定义，不埋点）
- ✅ `node design/context-dev/tools/specflow/specflow.mjs validate feat-observability-metrics --strict` 通过
- ✅ `cargo test` 通过（包含 metrics 相关单元测试）

### 提案大纲对齐（Roadmap Alignment）
| 字段 | 内容 |
|------|------|
| roadmap_source_primary | `openspec/proposal-roadmap.md` |
| roadmap_source_supplement | N/A |
| phase | Phase 4（可观测性 + E2E 验证） |
| business_goal | 暴露 TAD §11.2 定义的 11 个 Counter + `GET /metrics` 端点 |
| dependencies | 前置: `feat-observability-logging`（已完成）；被依赖: `feat-e2e-integration-test` |
| acceptance_criteria | `/metrics` 可访问 → 200 + Prometheus 格式；全部 11 个指标已注册；发消息后 `messages_received_total` 递增；各边缘场景 Counter 递增 |
