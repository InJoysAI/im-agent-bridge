# Change: 消息标准化 + message_events 状态机

## Why

当前 `POST /gateway/inbound` 处理链路在完成 channel 解析和 session 生成后即返回，缺少以下能力：

1. **StandardMessage 构建缺失**：入站消息没有被标准化为内部统一对象（缺少 `event_id` 等字段），Runtime 调用链路无法以标准结构驱动。
2. **消息事件持久化缺失**：`message_events` 表未被写入，处理状态（pending→processing→done/error）无法追踪，也无法用于排障。
3. **入站幂等去重缺失**：`uq_message_events_inbound_dedup` 唯一索引已建立（迁移 `00001_init.sql`），但去重逻辑尚未接入处理链路，重复消息会被重复处理。
4. **input_text 截断未落库**：无处将入站文本按 512 字符截断后写入 `message_events.input_text`，违反数据最小化要求（BR-070）。

本提案实现这四项能力，构建 Runtime 调用前的完整消息准备层，并为 `feat-runtime-nanobot-adapter` 提供标准化输入结构。

## What Changes

### 新增功能
- `StandardMessage` struct（`event_id` UUID v4 + 全字段）及其构建函数 `StandardMessage::build()`
- `db/message_events.rs` 模块：
  - `insert_pending()`：原子执行幂等去重 + 写入（`INSERT ... ON CONFLICT (...COALESCE...) DO NOTHING RETURNING id`），返回 `Option<Uuid>`；`None` = 重复，`Some(id)` = 新行，携带 `bot_id`（BR-032）
  - `mark_processing()`：将 `status` 更新为 `processing`
  - `mark_done()` / `mark_error()`：终态更新（为 Runtime 调用提供钩子）
- 入站幂等去重：重复消息返回 `HTTP 200 + {"status":"ignored_duplicate"}`，不继续处理，不调用 Runtime，不重新写入 `message_events`

### 修改功能
- `handlers/inbound.rs`：在 session upsert 之后插入消息标准化 + message_events 写入 + 幂等去重逻辑

### 技术实现
- `event_id` 使用 `uuid` crate 生成 UUID v4（`Uuid::new_v4().to_string()`）
- 入站幂等去重通过 `insert_pending()` 原子实现：`INSERT ... ON CONFLICT (platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id) DO NOTHING RETURNING id`，无返回行（`None`）即为重复（详见 design.md）
- `input_text` 落库前按 Unicode 字符数截断至 512 字符（`text.chars().take(512).collect::<String>()`）
- 所有 `message_events` 的 INSERT / SELECT 携带 `bot_id` 过滤条件（BR-032）
- `reply_id` 在 INSERT 时同步生成（UUID v4），用于回写幂等（留给 Runtime 调用提案使用）

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/message-pipeline/spec.md` — StandardMessage 构建、message_events 状态机、入站幂等去重、input_text 截断

### 涉及的代码
- **新增**：
  - `gateway/src/models/standard_message.rs` — StandardMessage struct + build()
  - `gateway/src/db/message_events.rs` — DB 操作层
- **修改**：
  - `gateway/src/handlers/inbound.rs` — 集成去重检查 + message_events 写入
  - `gateway/src/models/mod.rs` — 导出 standard_message
  - `gateway/src/db/mod.rs` — 导出 message_events
  - `gateway/src/config.rs` — 测试环境锁的抗中毒处理（保证 `cargo test` 稳定性）

### 依赖关系
- **依赖**：`feat-gateway-channel-session`（已完成）— 提供 `bot_id` 解析和 session_id 生成
- **被依赖**：`feat-runtime-nanobot-adapter` — 使用 `StandardMessage` 驱动 Runtime 调用

### 风险与注意事项
- 幂等去重依赖 `uq_message_events_inbound_dedup` 唯一索引已存在于 DB Schema（迁移 `00001_init.sql`），实施前须确认迁移已执行
- `message_events.reply_id` 为 `UNIQUE NOT NULL`，INSERT 时必须同步生成（不可延迟到回写阶段）
- 状态机函数（`mark_processing` / `mark_done` / `mark_error`）本提案写出但仅在 `insert_pending` 后调用 `mark_processing`；`mark_done` / `mark_error` 由 `feat-runtime-nanobot-adapter` 调用，本提案只定义函数签名并添加单元测试

### 验证标准
- ✅ 入站 5000 字符消息 → 400 + "消息过长，请缩短后重试"（由已有 inbound-gate spec 覆盖，本提案不回归该逻辑）
- ✅ 正常消息 → `message_events` 插入一条 `status=pending` 记录，`event_id` 为 UUID v4 格式
- ✅ `message_events.input_text` = 入站 text 的前 512 字符（Unicode）
- ✅ 相同幂等键（`platform` / `bridge_gateway_name` / `bridge_channel_name` / `bridge_message_id`）重复到达 → HTTP 200 + `{"status":"ignored_duplicate"}`，`message_events` 无新行
- ✅ 所有 `message_events` INSERT/SELECT 携带 `bot_id` 条件（BR-032）
- ✅ `StandardMessage` 构建后字段完整：`event_id`、`platform`、`chat_id`、`chat_type`、`user_id`、`session_id`、`text`（≤4096）、`timestamp`、`bot_id` 均非空
