# Change: 结构化日志 (feat-observability-logging)

## Why

Gateway 当前缺乏结构化日志输出能力。MVP 主链路（入站/认证/Channel解析/Runtime/回写）排障依赖裸文本输出，无法机器解析，且 Bearer Token 等敏感凭证存在日志泄露风险。

引入 `tracing-subscriber` JSON 格式化输出并实现脱敏 filter，使日志可结构化采集、安全可审计，同时为 `feat-observability-metrics` 和 `feat-e2e-integration-test` 奠定可观测性基础。

## What Changes

### 新增功能
- `tracing-subscriber` 依赖，配置 JSON 格式化输出（INFO/WARN/ERROR 级别，通过 `RUST_LOG` 环境变量控制）
- 脱敏 filter 层：屏蔽所有高敏感凭证（`GATEWAY_BEARER_TOKEN`、`BRIDGE_BEARER_TOKEN`、`TELEGRAM_BOT_TOKEN`、`SHOPIFY_CLIENT_SECRET`、`DATABASE_URL`、`POSTGRES_PASSWORD`）进入任意级别日志
- Gateway 侧 8 类 TAD §11.1 必选事件埋点：消息接入 / 标准化完成 / session_id 生成-命中 / Runtime 调用 / 回写结果 / 错误 / 被限流请求 / DB 不可用
- 附加 Gateway 埋点（非 TAD 必选，提升可观测性）：Bearer Token 认证 / Channel 解析（bot_id）
- MCP 调用日志（TAD §11.1 第 5 类）归属 Runtime 侧独立 change，需确保携带相同 `event_id`
- `event_id` 字段贯穿主链路各 span（即 TAD §11.3 规范的 `trace_id` 概念，MVP 阶段以 `event_id` 命名实现，合并到同一字段）

### 修改功能
- `gateway/src/main.rs`：将现有日志初始化替换为 tracing-subscriber JSON subscriber

### 技术实现
- 依赖：`tracing-subscriber`（features: `env-filter`, `json`）+ `tracing`
- 脱敏 filter：自定义 `tracing::Layer` 实现，格式化前检测并遮蔽 `SENSITIVE_FIELDS` 列表中的字段值
- span 传播：通过 `tracing::Span` 在 tokio 异步任务链路中传递 `event_id`

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/observability-logging/spec.md` — 结构化日志行为规范

### 涉及的代码
- **新增**：
  - `gateway/src/observability/mod.rs` — subscriber 初始化 + 脱敏 layer
- **修改**：
  - `gateway/src/main.rs` — 替换日志初始化
  - `gateway/src/handlers/inbound.rs`（及相关 handler / adapter 模块）— 埋点 span/event

### 依赖关系
- **依赖**：`feat-gateway-inbound-gate`（已完成，主链路入站处理就绪）
- **被依赖**：`feat-observability-metrics`、`feat-e2e-integration-test`

### 风险与注意事项
- 脱敏漏报风险：敏感字段名或值格式变化时需同步更新 `SENSITIVE_FIELDS`（缓解：集中定义常量列表，禁止分散硬编码）
- JSON 输出轻微性能开销：MVP 吸关量下可接受；若未来出现热路径问题可切换为异步写
- 本提案实现 RISK-006（Bearer Token 泄露，`risks_and_debt.md`）的直接缓解措施：日志脱敏层屏蔽 Token 明文输出

### 验证标准
- ✅ 日志输出为 JSON 结构（可用 `jq .` 无报错解析）
- ✅ 日志中无任何高敏感凭证明文（Bearer Token / Shopify secret / DB 密码等均显示为 `[REDACTED]` 或被省略）
- ✅ Gateway 侧 8 类 TAD §11.1 必选事件均能通过 JSON 输出，并携带 `event_id`（即 trace_id）字段；MCP 调用由 Runtime 侧独立 change 覆盖

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md §4` | 安全约束：日志禁止记录敏感凭证（MUST） |
| architecture | `.context/architecture/cross_cutting_concepts.md §日志规范` | TAD §11.1：9 个必覆盖日志事件 + 脱敏要求 |
| architecture | `.context/architecture/security_policy.md §敏感数据处理` | Bearer Token 分类为高敏感，禁止日志 |
| architecture | `.context/architecture/tech_stack.md §核心依赖` | `tracing` SHOULD 依赖 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-006 Bearer Token 泄露：本提案脱敏 Filter 直接缓解该风险 |

### Roadmap 对齐快照

| 字段 | 内容 |
|------|------|
| roadmap_source | `openspec/proposal-roadmap.md` 提案 12 |
| phase | Phase 4 |
| priority | P1 |
| estimated | 1 天 |
| status | proposed |
| business_goal | tracing-subscriber JSON 配置 + 脱敏 filter（全凭证） + Gateway 侧 8 类 TAD 必选事件埋点 + 附加埋点（认证/Channel解析）；MCP 归属 Runtime |
| in_scope | tracing-subscriber JSON 配置 / 脱敏 filter / event_id 贯穿主链路 |
| out_of_scope | 分布式追踪（Jaeger，MVP 不引入） / MCP 调用日志（归属 Runtime 侧） |
| acceptance_criteria | JSON 格式输出 / 脱敏 / event_id 贯穿主链路 |
| depends_on | feat-gateway-inbound-gate |
| depended_by | feat-observability-metrics, feat-e2e-integration-test |
