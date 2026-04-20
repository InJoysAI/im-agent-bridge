# Change: 入站网关（Bearer Token + 限流）

## Why

Gateway 骨架（`feat-infra-gateway-scaffold`）与 DB 连接层（`feat-gateway-db-layer`）已就绪，但 `POST /gateway/inbound` 路由尚不存在。Matterbridge 适配器（`mb-adapter`）当前无法将 Telegram 消息推送至 Gateway，整条链路处于断路状态。

本变更建立入站网关的**安全边界**与**流量管控**：
- **Bearer Token 校验**：无效请求必须在进入任何业务逻辑前被拦截，返回 401（BR-031、criterion.md §4）
- **Token Bucket 限流**：保护下游 DB、Runtime 不被单一 chat_id 的消息风暴压垮（BR-055）
- **InboundRequest 反序列化 + 字段校验**：确保进入链路的消息结构完整且格式合法（BR-004）
- **非文本消息拦截**：MVP 仅处理文本消息，非文本类型须在入口拦截（BR-001）

本变更**不含** channel_bindings 解析、session_id 生成、Runtime 调用，这些由 `feat-gateway-channel-session` 及后续提案实现。

## What Changes

### 新增功能
- `POST /gateway/inbound` axum 路由注册，含 `Authorization: Bearer <token>` middleware（constant-time 比较防时序攻击）
- Token Bucket 限流器：按 `chat_id` 维度，阈值 5 msg/sec，LRU 清理过期键；超限返回 429，不调用 Runtime，不写 `message_events`
- `gateway/src/models/inbound.rs` 内联手写 `InboundRequest` / `RawMessage` 等 model structs，顶部注释锚定 SSoT（`make api-compile` 验证契约一致性）
- 基本字段校验：缺必填字段 → 400；`message_type ≠ text` → 400 + 忽略提示（BR-001）
- 统一错误响应格式：`{ "error": "<message>" }`，覆盖 400/401/429/500/503

### 技术实现
- Bearer Token middleware：从 `Authorization: Bearer <token>` header 提取 token，使用 `constant_time_eq` crate 与 `Extension<BearerTokenConfig>` 做恒时比较；token 禁止出现在任何日志（tracing span field filter）
- Token Bucket 实现：`std::collections::HashMap<String, TokenBucket>` + `Arc<Mutex<...>>`；LRU 策略：每次请求时驱逐超过 60s 未访问的键；bucket 参数 capacity=5, refill_rate=5/s
- `gateway/src/models/inbound.rs` 直接定义所有 model structs（`InboundRequest`、`RawMessage`、枚举等），字段与 `SSoT/api/main.tsp` 严格对齐；`make api-compile` 保证 TypeSpec → OpenAPI YAML 契约验证链路有效
- 修正 `Makefile` 中 `api-gen-rs` 输出路径（供参考，产物加入 `.gitignore` 不纳入编译树），创建 `SSoT/api/openapi-generator-rs.yaml` 和 `.ignore` 配置文件

### SSoT 状态
- **API 合约**：`SSoT/api/main.tsp` 已包含 `POST /gateway/inbound` 端点及 `InboundRequest` / `RawMessage` 模型；**本提案验证契约一致性，不新增端点定义**
- **迁移文件**：本变更不涉及 DB Schema 变更，**不新增 Goose 迁移文件**

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/inbound-gate/spec.md` — Bearer Token 认证、Token Bucket 限流、字段校验与非文本拦截行为规范

### 涉及的代码
- **新增**：
  - `SSoT/api/openapi-generator-rs.yaml`（codegen 配置，供 `make api-gen-rs` 参考使用）
  - `SSoT/api/openapi-generator-rs.ignore`（空文件，避免 Makefile 参数报错）
  - `gateway/src/handlers/inbound.rs`（inbound handler）
  - `gateway/src/middleware/auth.rs`（Bearer Token extractor，`BearerTokenConfig` + `BearerAuth`）
  - `gateway/src/middleware/rate_limit.rs`（Token Bucket 限流器）
  - `gateway/src/models/inbound.rs`（内联手写 model structs + `ErrorResponse` + `ValidatedJson` extractor）

- **修改**：
  - `Makefile`（修正 `api-gen-rs` 输出路径至 `gateway/src/generated/`）
  - `gateway/src/main.rs`（注册路由，注入 `BearerTokenConfig` Extension + `RateLimiter`）
  - `gateway/src/config.rs`（确认 `GATEWAY_BEARER_TOKEN` 字段存在）
  - `gateway/Cargo.toml`（添加 `constant_time_eq`、`async-trait`）
  - `.gitignore`（新增 `gateway/src/generated/` 忽略规则）

### 依赖关系
- **依赖**：`feat-gateway-db-layer`（done）— PgPool Extension、`health_check()`、DB 熔断逻辑
- **被依赖**：`feat-gateway-channel-session`（需要入站路由就位后方可实现 bot_id 解析）、`feat-observability-logging`

### 风险与注意事项
- RISK-006（Bearer Token 泄露）：`GATEWAY_BEARER_TOKEN` 仅通过环境变量注入；tracing span filter 必须屏蔽该字段；constant-time 比较防止时序侧信道攻击
- RISK-007（TAD/工具链能力差距）：实施时评估 `openapi-generator-cli -g rust` 输出为完整 Rust 子 crate（过重），已采用 RISK-007 回退方案：手写 model structs + SSoT 注释锚定 + `make api-compile` 契约验证。`typify` 可作为 model 数量增长后的升级路径
- Token Bucket 使用 `Arc<Mutex<HashMap>>` 存储于内存，Gateway 重启后限流状态清零（可接受，MVP 无跨进程限流要求）

### 验收标准
- ✅ 无 `Authorization` header 的请求 → HTTP 401
- ✅ 同一 `chat_id` 1s 内第 6 条消息 → HTTP 429，Runtime 不被调用，`message_events` 不写入
- ✅ 缺少 `platform` 字段 → HTTP 400
- ✅ `message_type = "image"` → HTTP 400 + 忽略提示，不写 `message_events`，不调用 Runtime
- ✅ `make api-compile` 通过，TypeSpec 契约与 `InboundRequest` struct 对齐验证完成
- ✅ 入站文本 > 4096 字符 → HTTP 400 + 用户提示（“消息过长，请缩短后重试”），记录长度日志，不写 `message_events`（BR-002）
- ✅ 空消息/仅空白文本 → HTTP 400，不进入主处理链路（BR-001）

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | `openspec/proposal-roadmap.md` |
| roadmap_source_supplement | N/A |
| phase | Phase 1 |
| change_id | `feat-gateway-inbound-gate` |
| title | 入站网关（Bearer Token + 限流） |
| business_goal | `POST /gateway/inbound` 路由 + Bearer Token 校验（无效 → 401）；Token Bucket 限流（5 msg/sec/chat_id → 429）；InboundRequest 反序列化 + 基本字段校验（缺必填 → 400）；非文本消息拦截（BR-001） |
| in_scope | axum 路由 + Bearer middleware；Token Bucket 限流器（LRU 清理）；手写 InboundRequest/RawMessage struct（SSoT 注释锚定）；非文本拦截；统一错误响应 400/401/429/500/503 |
| out_of_scope | channel_bindings 解析、session_id 生成（feat-gateway-channel-session） |
| dependencies | `feat-gateway-db-layer`（done） |
| acceptance_criteria | 无 auth → 401；1s 第 6 条 → 429；缺 platform → 400；image → 400 + 忽略 |
| key_tasks | 1.验证 SSoT/api/main.tsp（make api-compile）；2.Bearer Token middleware（constant_time_eq）；3.手写 InboundRequest/RawMessage struct + 字段校验 + 非文本拦截；4.Token Bucket 限流器；5.路由注册 + 单元测试 |
| risks | RISK-006（Bearer Token 泄露）：环境变量注入 + tracing filter + constant-time 比较 |
| related_context_assets | criterion.md；business_rules.md（BR-001/031/055）；api_strategy.md；security_policy.md；risks_and_debt.md |
| milestones | N/A |
| coverage_scope | N/A |
| gate_vs_non_gate | Gate：Bearer Token 无效 → 401；限流 → 429；非文本 → 400 |
| change_management | N/A |
| ops_support | N/A |
| kpi | N/A |
| risk_acceptance_policy | RISK-006 Token Bucket 内存存储：重启后限流状态清零，MVP 可接受 |

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.4 Gateway MUST/MUST NOT；§4 安全约束（Bearer Token）；§入站文本长度限制（118-119） |
| domain | `.context/domain/business_rules.md` | BR-001（非文本/空消息拦截）；BR-002（超长入站拒绝）；BR-031（Bearer Token 认证）；BR-055（Token Bucket 限流） |
| domain | `.context/domain/edge_cases.md` | 限流边界场景；非文本消息安全处理；入站超长（BR-002）；空消息忽略（:17-18） |
| architecture | `.context/architecture/api_strategy.md` | POST /gateway/inbound 契约（§1）；InboundRequest 字段定义；错误码语义 |
| architecture | `.context/architecture/security_policy.md` | Bearer Token 管理；敏感字段禁止日志；constant-time 比较要求 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-006（Bearer Token 泄露）；RISK-007（工具链能力差距） |
| architecture | `.context/architecture/cross_cutting_concepts.md` | 统一错误响应格式；tracing span 规范；限流日志等级；入站文本长度超限处理（:106-109） |
| architecture | `.context/architecture/tech_stack.md` | MVP MUST NOT 引入 Redis/缓存层（Token Bucket 内存方案归因） |
