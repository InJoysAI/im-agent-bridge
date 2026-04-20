## Context

Gateway（Rust/tokio）需要结构化日志能力。当前 `tracing` crate 已作为 SHOULD 依赖被 tech_stack 引用；本 change 将其与 `tracing-subscriber` 配合正式落地。

日志必须满足两个硬约束（criterion.md §4 / cross_cutting_concepts.md §11.1）：
1. JSON 格式可机器解析
2. 任何级别均不输出 Bearer Token 等敏感凭证

## Goals / Non-Goals

Goals:
- JSON 结构化日志输出，通过 `RUST_LOG` 控制级别
- 脱敏 filter 屏蔽所有高敏感凭证：GATEWAY_BEARER_TOKEN / BRIDGE_BEARER_TOKEN / TELEGRAM_BOT_TOKEN / SHOPIFY_CLIENT_SECRET / DATABASE_URL / POSTGRES_PASSWORD（SENSITIVE_FIELDS 常量列表，集中定义）
- `event_id`（即 TAD trace_id）贯穿 Gateway 侧全部可观测事件的 span/event（共 8 类，MCP 调用归属 Runtime）

Non-Goals:
- 分布式追踪（Jaeger/OpenTelemetry）——MVP 不引入（roadmap out_of_scope）
- 日志聚合平台接入（ELK/Loki 等）——MVP 不引入
- 运行时动态调整日志级别

## Decisions

- **Decision**: 使用 `tracing-subscriber` 的 `.fmt().json()` 层作为格式化器，不自行实现 JSON 序列化。
  - **Alternatives considered**: `slog`（生态较旧）、`env_logger`（无 JSON 支持）。`tracing-subscriber` 与 tokio 生态原生集成，且 `tracing` 已在 tech_stack SHOULD 列表中。

- **Decision**: 脱敏 filter 实现为自定义 `tracing::Layer`，在序列化前遮蔽命中字段值（替换为 `[REDACTED]`），而非在调用侧手动过滤。
  - **Alternatives considered**: 在每个 `tracing::event!` 调用处手动避免传入敏感值——分散且易遗漏，选择集中 Layer 方式保证防御纵深。

- **Decision**: `event_id` 通过 `tracing::Span` 的字段传播（`tracing::info_span!("inbound", event_id = %id)`），利用 tokio 的 `instrument` 或手动 `enter`/`in_scope`，不通过函数参数逐层透传。
  - **Alternatives considered**: 通过函数参数显式传递——侵入性高，修改面广。

## Risks / Trade-offs

- 脱敏 Layer 漏报：`SENSITIVE_FIELDS` 列表需与实际环境变量名保持一致；若字段名变更需同步更新（集中定义降低漏报概率）。
- JSON 格式化性能开销：相比文本格式有轻微序列化成本；MVP 吞吐量（单机 Telegram bot）下可接受，后续如有热路径需求可开启异步写。

## Migration Plan

无历史数据迁移。日志格式切换为 JSON 后，若有现有 grep 脚本依赖文本格式，需相应调整为 `jq` 解析。

## Open Questions

- 是否需要将 `tracing-subscriber` 的输出同时写文件（`RollingFileAppender`）？当前方案仅写 stdout，由容器日志驱动（Docker logging driver）收集。如需文件输出，应在后续独立 change 中引入。
- 中敏感字段（chat_id / user_id）是否需要在日志中做哈希或脱敏？当前方案允许明文输出（侜为排障关键字段），但 security_policy.md 将其划为中敏感，后续如有合规要求可对 user_id 做哈希化处理。
