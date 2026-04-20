# message-pipeline Specification

## Purpose
TBD

## Requirements
### Requirement: StandardMessage 构建
系统必须（MUST）在 channel 解析和 session 生成完成后，使用入站字段构建 `StandardMessage` struct，`event_id` 字段由 Gateway 生成 UUID v4，所有必填字段填充完整，不允许 null（BR-004，domain_model.md §1）。

#### Scenario: 正常消息完整构建 StandardMessage
- **WHEN** 入站消息通过所有前置校验（认证、限流、字段校验、非文本拦截、长度检查、DB 健康、channel 解析、session upsert）
- **THEN** Gateway 构建 `StandardMessage`，字段 `event_id`（UUID v4）、`platform`、`chat_id`、`chat_type`、`user_id`、`session_id`、`text`（≤4096 字符）、`timestamp`、`bot_id` 均不为 null
- **AND** `event_id` 格式符合 UUID v4 规范（`xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx`）

#### Scenario: StandardMessage.text 不超 4096 字符
- **WHEN** 入站 text 通过 4096 字符长度检查（已由 inbound-gate 保证）
- **THEN** `StandardMessage.text` = 入站 `raw_message.text`，长度 ≤ 4096 字符
- **AND** 不对 text 做截断处理（截断仅发生在 `input_text` 落库时）

---

### Requirement: 入站幂等去重
系统必须（MUST）在写入 `message_events` 前，通过 `uq_message_events_inbound_dedup` 唯一索引（`platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id`）检查是否为重复消息；重复时返回 HTTP 200 + `{"status":"ignored_duplicate"}`，不写入新行，不调用 Runtime（BR-042，api_strategy.md §1.6）。

#### Scenario: 重复消息幂等返回 ignored_duplicate
- **WHEN** 相同 `(platform, bridge_gateway_name, bridge_channel_name, bridge_message_id)` 的消息再次到达（前次已成功写入 `message_events`）
- **THEN** Gateway 返回 HTTP 200
- **AND** 响应体为 `{"status":"ignored_duplicate"}`
- **AND** `message_events` 表中无新行插入
- **AND** Runtime 不被调用
- **AND** `sessions.updated_at` 可能已被刷新（session upsert 在去重检查前执行，属已知可接受权衡）

#### Scenario: 首次到达的消息不被视为重复
- **WHEN** `(platform, bridge_gateway_name, bridge_channel_name, bridge_message_id)` 在 `message_events` 表中无对应记录
- **THEN** 幂等检查通过，继续写入 `message_events`

#### Scenario: bridge_channel_name 为 NULL 时幂等键正确处理
- **WHEN** 入站消息的 `bridge_channel_name` 为 NULL（网关级绑定）
- **AND** 相同 `(platform, bridge_gateway_name, NULL→'', bridge_message_id)` 已存在记录
- **THEN** 同样返回 `{"status":"ignored_duplicate"}`（COALESCE 将 NULL 视为空字符串参与唯一约束）

---

### Requirement: message_events 状态机写入
系统必须（MUST）在幂等检查通过后，以 `status = pending` 将消息事件写入 `message_events` 表，随后更新 `status = processing`；所有写操作必须携带 `bot_id` 过滤/关联条件（BR-032，domain_model.md §5，business_rules.md §BR-042）。

#### Scenario: 首次入站写入 pending 状态
- **WHEN** 幂等检查通过，消息为首次到达
- **THEN** `message_events` 表插入一条新记录，`status = 'pending'`
- **AND** `event_id` = `StandardMessage.event_id`（UUID v4）
- **AND** `bot_id` = 解析出的 bot_id（非 null，BR-032）
- **AND** `reply_id` 为 Gateway 新生成的 UUID v4（回写幂等键，预留给 Runtime 调用阶段）

#### Scenario: pending 写入后更新为 processing
- **WHEN** `message_events` pending INSERT 成功
- **THEN** 调用 `mark_processing(event_id, bot_id)` 将 `status` 更新为 `'processing'`
- **AND** 更新操作的 WHERE 条件包含 `bot_id`（BR-032）

#### Scenario: insert_pending DB 写入失败时 503 熔断
- **WHEN** `message_events` INSERT 时 PostgreSQL 不可达或返回非幂等冲突错误
- **THEN** Gateway 返回 HTTP 503
- **AND** 不调用 Runtime
- **AND** `db_unavailable_total` 计数器递增（与 pool.rs 现有熔断行为一致）

#### Scenario: mark_processing DB 失败时 503 熔断
- **WHEN** `insert_pending` 成功（`Some(id)` 返回）后，`mark_processing` 执行时 PostgreSQL 不可达
- **THEN** Gateway 返回 HTTP 503
- **AND** `db_unavailable_total` 计数器递增
- **AND** 此时 `message_events` 记录停留在 `status='pending'`（已知可接受权衡，MVP 阶段不做自动恢复）

#### Scenario: BR-032 所有 message_events 操作携带 bot_id
- **WHEN** 执行任意 `message_events` 的 INSERT 或 UPDATE
- **THEN** SQL 语句中包含 `bot_id = $n` 参数（INSERT 时作为字段，UPDATE 时作为 WHERE 条件）

---

### Requirement: input_text 512 字符截断落库
系统必须（MUST）在写入 `message_events` 时，将入站 text 按 Unicode 字符数截断至最多 512 字符后存入 `input_text` 字段，不存储超出部分（BR-070，db/schema_design.md message_events 表）。

#### Scenario: text ≤ 512 字符时完整存储
- **WHEN** 入站 `raw_message.text` 长度 ≤ 512 字符
- **THEN** `message_events.input_text` = 完整 text（无截断）

#### Scenario: text > 512 字符时截断存储
- **WHEN** 入站 `raw_message.text` 长度为 600 字符
- **THEN** `message_events.input_text` = 前 512 个 Unicode 字符
- **AND** 第 513 至 600 字符不写入 `message_events`

#### Scenario: 截断不影响 StandardMessage.text
- **WHEN** text 为 600 字符，同时构建 StandardMessage 和写入 message_events
- **THEN** `StandardMessage.text` = 完整 600 字符（供 Runtime 调用使用）
- **AND** `message_events.input_text` = 前 512 字符（仅排障存储，BR-070）
