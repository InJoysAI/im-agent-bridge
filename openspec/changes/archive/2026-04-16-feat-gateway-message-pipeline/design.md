## Context

本提案在 `feat-gateway-channel-session` 完成后，向 inbound 处理链路中插入消息标准化、幂等去重和 message_events 状态机。核心设计问题是 **幂等去重的实现方式**，以及 **StandardMessage 的归属模块**。

## Goals / Non-Goals
- Goals:
  - 实现 StandardMessage struct + 构建函数，event_id = UUID v4
  - 实现 message_events INSERT（status=pending） + mark_processing
  - 实现基于 `uq_message_events_inbound_dedup` 的入站幂等去重
  - input_text 按 Unicode chars 截断至 512 字符落库
  - 所有 DB 操作携带 bot_id（BR-032）
- Non-Goals:
  - Runtime 调用（`feat-runtime-nanobot-adapter` 负责）
  - mark_done / mark_error 调用（定义函数但不在本提案中实际触发）
  - message_events 30 天清理定时任务（TD-005，已知技术债）
  - output_text 写入（Runtime 完成后处理）

## Decisions

- **Decision**: 幂等去重使用 `INSERT ... ON CONFLICT (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''), bridge_message_id) DO NOTHING RETURNING id`；返回 `None`（无 RETURNING 行）= 重复消息，直接返回 `ignored_duplicate`。
  - **关键注意**: `uq_message_events_inbound_dedup` 是表达式唯一索引（含 `COALESCE`），不是命名约束，因此不能用 `ON CONFLICT ON CONSTRAINT` 引用。必须在 ON CONFLICT 子句中写出完整表达式， PostgreSQL 将自动匹配该索引。
  - **Alternatives considered**:
    1. 先 SELECT 检查再 INSERT（两步法）：存在竞态窗口，在极高并发场景下可能双写；ON CONFLICT 原子性更强。
    2. `ON CONFLICT DO UPDATE SET status=status RETURNING id`（upsert）：语义不明确，会误更新记录。
    - **结论**：ON CONFLICT DO NOTHING RETURNING id 是最简洁且原子的方案；返回类型用 `Option<Uuid>` 替代 `bool` 更贴切 SQL 原生行为。

- **Decision**: `StandardMessage` 放置在 `gateway/src/models/standard_message.rs`，与 `InboundRequest` 同层。
  - **Alternatives considered**: 放在 `gateway/src/domain/` 独立目录（过度设计，MVP 阶段结构未超出 models 范畴）。

- **Decision**: `reply_id` 在 `insert_pending` 时同步生成（UUID v4），写入 `message_events`，不延迟。
  - **Rationale**: `message_events.reply_id` 字段为 `UNIQUE NOT NULL`，无法延迟写入；提前生成对 Runtime 调用阶段无副作用，Runtime 提案直接读取使用。

- **Decision**: `input_text` 截断使用 `text.chars().take(512).collect::<String>()`（Unicode 字符边界安全）。
  - **Rationale**: 与现有 4096 字符校验的 `chars().count()` 方式一致，避免字节边界问题。

## Risks / Trade-offs

- `uq_message_events_inbound_dedup` 已包含在 `00001_init.sql`（初始化迁移）中，不存在独立的 `00004`/`00005` 迁移文件。如果迁移根本未执行，所有表（`bots`、`sessions` 等）均不存在，其他功能也会全面失败——幂等去重失效不会“静默”发生，而是 DB 错误 → 503。无需对此进行额外起动检查。
- `mark_processing` 紧跟 `insert_pending`，两步之间若进程崩溃，该记录会停在 `pending`。当前 MVP 阶段不做幂等重试恢复，这是已知的可接受权衡。

## Migration Plan

无新迁移。`message_events` 表和 `uq_message_events_inbound_dedup` 表达式唯一索引已完整包含在 `SSoT/schema/migrations/00001_init.sql` 中。

## Open Questions

- ~~`reply_id` 的前缀格式~~（**已解决**）：统一使用 UUID v4（`Uuid::new_v4().to_string()`），与 `event_id` 保持一致，满足 `UNIQUE NOT NULL` 约束，降低代码复杂度。`api_strategy.md §3.3` 的 `rep_20260412_xxx` 示例格式不再使用。
