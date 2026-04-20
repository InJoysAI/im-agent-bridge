## ADDED Requirements

### Requirement: 群聊 @mention 过滤（Gateway 入口层）

Gateway 必须（MUST）在 Bearer Token 校验、字段校验通过且按 `bot_id` 加载到 Bot 配置后，对 `platform = "telegram"` 且 `raw_message.chat_type = "group"` 且 `bot.require_mention = true` 的入站消息执行 @mention 过滤：按 `@{bot.telegram_username}` 模式在 `raw_message.text` 中做大小写不敏感子串匹配（匹配模式包含 `@` 前缀，避免词素误判），未命中则返回 HTTP 200 + JSON Body `{"status": "ignored_no_mention"}`，且 MUST NOT 写入 `message_events`、MUST NOT 调用 Runtime、MUST NOT 触发 Bridge 回写；私聊（`chat_type = "private"`）与 `bot.require_mention = false` 的 Bot 不受此过滤影响（退化为全量响应，等价于 BR-032 既有语义）。Bot 配置的加载 MUST 通过 `bots::get_by_id(bot_id)`，禁止全表扫（BR-032）。过滤命中时 MUST 记录日志 `inbound skipped: group_no_mention`，含 `bot_id` / `chat_id`，但 MUST NOT 记录 `telegram_username` 明文。

#### Scenario: 群聊未 @ 机器人的消息被忽略

- **WHEN** Bot A 配置 `telegram_username = "CBECOpsBot"` 且 `require_mention = true`
- **AND** `channel_bindings` 映射当前群聊到 Bot A
- **AND** 用户在群内发送纯文本 `"今天天气不错"`（不含 `@CBECOpsBot`）
- **THEN** Gateway 返回 HTTP 200
- **AND** 响应体为 `{"status": "ignored_no_mention"}`
- **AND** `message_events` 无新增行
- **AND** Runtime 未被调用
- **AND** 未触发任何 Bridge 回写

#### Scenario: 群聊 @ 机器人消息正常进入主链路

- **WHEN** Bot A 配置 `telegram_username = "CBECOpsBot"` 且 `require_mention = true`
- **AND** 用户发送 `"@CBECOpsBot 查询订单 123"`
- **THEN** Gateway 通过 mention 过滤层
- **AND** 继续执行主链路（写 `message_events` pending、调用 Runtime）
- **AND** 响应体为 `{"status": "accepted"}`

#### Scenario: 群聊 @ 大小写不敏感命中

- **WHEN** Bot A 配置 `telegram_username = "CBECOpsBot"` 且 `require_mention = true`
- **AND** 用户发送 `"@cbecopsbot hi"`（全小写）
- **THEN** Gateway 视为命中 mention
- **AND** 请求进入主链路
- **AND** 响应体为 `{"status": "accepted"}`

#### Scenario: 私聊不受 mention 过滤影响

- **WHEN** Bot A 配置 `telegram_username = "CBECOpsBot"` 且 `require_mention = true`
- **AND** 用户通过 1:1 私聊发送 `"你好"`（`chat_type = "private"`，无 @）
- **THEN** Gateway 绕过 mention 过滤层
- **AND** 请求进入主链路
- **AND** 响应体为 `{"status": "accepted"}`

#### Scenario: require_mention=false 的 Bot 保持全量响应

- **WHEN** Bot B 配置 `require_mention = false`（`telegram_username` 可为 NULL）
- **AND** 用户在群内发送 `"随便聊几句"`（不含任何 @）
- **THEN** Gateway 跳过 mention 过滤层
- **AND** 请求进入主链路
- **AND** 响应体为 `{"status": "accepted"}`

#### Scenario: Bot 配置通过 bot_id 精确加载（BR-032）

- **WHEN** inbound 请求携带 `bridge_gateway_name`，经 `channel_bindings` 解析出 `bot_id`
- **THEN** Gateway 调用 `bots::get_by_id(bot_id)` 一次加载 Bot 配置（含 `telegram_username` / `require_mention`）
- **AND** 不执行 `SELECT * FROM bots` 全表扫描或基于 `telegram_username` 的反查

#### Scenario: 迁移不破坏既有 Bot 行（Expand-Contract 向后兼容）

- **WHEN** 执行 Goose 迁移 `00005_bots_mention_filter.sql`（`ADD COLUMN IF NOT EXISTS`）
- **THEN** `bots` 表中所有迁移前存在的行，其 `telegram_username` 值为 `NULL`，`require_mention` 值为 `false`
- **AND** 行数不减少
- **AND** 已绑定该 Bot 的既有 `channel_bindings` 记录不受影响
- **AND** Gateway 以 `require_mention = false` 加载这些 Bot 时，继续执行全量响应链路（不触发 mention 过滤）

#### Scenario: Mention 过滤位于 Token Bucket 限流之前（BR-055 顺序约束）

- **WHEN** 某 `chat_id` 在群内 1 秒内连续发送 10 条无 @ 的闲聊消息（均不含 `@CBECOpsBot`）
- **AND** 同一秒内再发送 1 条 `@CBECOpsBot 查询订单` 真正 mention 消息
- **THEN** 前 10 条消息均返回 HTTP 200 `{"status": "ignored_no_mention"}`，不消耗该 `chat_id` 的 Token Bucket 令牌
- **AND** 第 11 条 mention 消息正常进入限流判定（此时令牌桶未被闲聊耗尽），返回 HTTP 200 `{"status": "accepted"}`
- **AND** 验证顺序：Mention 过滤必须先于 Token Bucket 限流执行（对应 `design.md > Decision 6`）

#### Scenario: Mention 过滤位于空文本校验之后（BR-001 顺序约束）

- **WHEN** 群聊发送 `raw_message.text = ""`（空字符串）且 Bot 配置 `require_mention = true`
- **THEN** Gateway 在进入 Mention 过滤分支前，先由空文本校验返回 HTTP 400
- **AND** 不执行 `@{telegram_username}` 子串匹配

---

### Requirement: InboundResponse 状态枚举扩展

Gateway 返回入站响应时，`InboundResponse.status` 允许值 MUST 为 `"accepted" | "ignored_duplicate" | "ignored_no_mention"` 三者之一。其中 `ignored_no_mention` 仅在「群聊 @mention 过滤」未命中时使用，对应 HTTP 200（语义为"请求合法且被预期过滤"，非错误），Bridge 侧 MUST NOT 对该状态执行重试；SSoT (`SSoT/api/main.tsp`) 的 `InboundStatus` union 与 codegen 产物 MUST 与本 Requirement 保持一致。

#### Scenario: ignored_no_mention 返回 HTTP 200 且语义为成功过滤

- **WHEN** 群聊 mention 过滤命中（未 @ 机器人）
- **THEN** 响应 HTTP 状态码为 200
- **AND** `Content-Type` 为 `application/json`
- **AND** 响应体 JSON 结构为 `{"status": "ignored_no_mention"}`
- **AND** 上游 Bridge 不得将该响应视为瞬时故障进行重试

#### Scenario: accepted 与 ignored_duplicate 语义保持不变

- **WHEN** 请求正常进入主链路
- **THEN** 响应体为 `{"status": "accepted"}`
- **AND** 当命中幂等去重时返回 `{"status": "ignored_duplicate"}`（既有语义不变）

#### Scenario: SSoT 与 codegen 保持对齐

- **WHEN** 查看 `SSoT/api/main.tsp` 中 `InboundResponse.status` 的 union 定义
- **THEN** 定义为 `"accepted" | "ignored_duplicate" | "ignored_no_mention"`
- **AND** `tsp compile` 产出的 OpenAPI / 生成的 Rust 客户端枚举包含这三个取值
