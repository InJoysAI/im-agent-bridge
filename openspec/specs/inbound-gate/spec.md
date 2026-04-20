# inbound-gate Specification

## Purpose
TBD

## Requirements
### Requirement: Bearer Token 认证
Gateway 必须（MUST）对所有 `POST /gateway/inbound` 请求校验 `Authorization: Bearer <token>` header，使用 constant-time 比较防止时序侧信道攻击；token 无效或缺失时返回 HTTP 401，不进入任何业务逻辑（BR-031，criterion.md §4）。

#### Scenario: 缺少 Authorization header 时拒绝请求
- **WHEN** Bridge 发送不含 `Authorization` header 的 POST /gateway/inbound 请求
- **THEN** Gateway 返回 HTTP 401
- **AND** 响应体包含 `{ "error": "..." }` 格式的错误信息
- **AND** 不执行任何业务处理（不查询 DB，不调用 Runtime）

#### Scenario: Authorization header 值无效时拒绝请求
- **WHEN** Bridge 发送 `Authorization: Bearer invalid-token` 的请求（token 与 GATEWAY_BEARER_TOKEN 不匹配）
- **THEN** Gateway 返回 HTTP 401
- **AND** 日志中不包含 GATEWAY_BEARER_TOKEN 明文值

#### Scenario: 有效 Bearer Token 时放行请求
- **WHEN** Bridge 发送包含正确 Bearer Token 的完整合法 InboundRequest
- **THEN** Gateway 通过认证层，继续执行后续处理流程（字段校验、限流检查等）

---

### Requirement: Token Bucket 入站限流
Gateway 必须（MUST）按 `chat_id` 维度实施 Token Bucket 限流，阈值 5 msg/sec/chat_id；超限时返回 HTTP 429，不调用 Runtime，不写入 `message_events`（BR-055，criterion.md §4）。

#### Scenario: 同一 chat_id 1 秒内第 6 条消息触发限流
- **WHEN** 同一 `chat_id` 在 1 秒内已成功接收 5 条消息
- **AND** 该 `chat_id` 在同一秒内发送第 6 条消息
- **THEN** Gateway 返回 HTTP 429 Too Many Requests
- **AND** 该消息不写入 `message_events`
- **AND** Runtime 不被调用

#### Scenario: 不同 chat_id 的限流互不干扰
- **WHEN** `chat_id_A` 已达到 5 msg/sec 限流阈值
- **AND** `chat_id_B` 发送请求（该秒内未超限）
- **THEN** `chat_id_B` 的请求被正常处理
- **AND** `chat_id_A` 的请求返回 429

#### Scenario: 超限后下一秒恢复处理
- **WHEN** 某 `chat_id` 在前一秒已触发限流
- **AND** 下一秒内该 `chat_id` 发送第 1 条消息
- **THEN** Gateway 正常接受该请求（不返回 429）

---

### Requirement: InboundRequest 反序列化与字段校验
Gateway 必须（MUST）将 `POST /gateway/inbound` 请求体反序列化为 `InboundRequest` struct，对必填字段（`platform`、`bridge_gateway_name`、`raw_message`、`raw_message.chat_id`、`raw_message.chat_type`、`raw_message.user_id`、`raw_message.message_type`、`raw_message.message_id`、`raw_message.timestamp`）执行存在性校验，缺失时返回 HTTP 400（BR-004）。

#### Scenario: 缺少必填字段 platform 时返回 400
- **WHEN** 请求体中不包含 `platform` 字段（其他字段完整）
- **THEN** Gateway 返回 HTTP 400 Bad Request
- **AND** 响应体包含错误描述

#### Scenario: 请求体格式非法 JSON 时返回 400
- **WHEN** 请求体为非合法 JSON 字符串
- **THEN** Gateway 返回 HTTP 400 Bad Request

#### Scenario: 全部必填字段完整时通过校验
- **WHEN** 请求体包含所有必填字段且格式合法（message_type = "text"，text 字段存在）
- **THEN** Gateway 通过字段校验，继续执行后续处理

---

### Requirement: 非文本消息类型拦截
Gateway 必须（MUST）在入口层拦截 `message_type ≠ "text"` 的消息，返回 HTTP 400 并附带忽略提示，不进入业务链路，不写入 `message_events`，不调用 Runtime（BR-001，criterion.md §3.2）。

#### Scenario: message_type 为 image 时拦截
- **WHEN** 请求体中 `raw_message.message_type = "image"`（Bearer Token 有效）
- **THEN** Gateway 返回 HTTP 400
- **AND** 响应体包含忽略提示（如 `"非文本消息类型，已忽略"`）
- **AND** 不写入 `message_events`
- **AND** 不调用 Runtime

#### Scenario: message_type 为 audio 时拦截
- **WHEN** 请求体中 `raw_message.message_type = "audio"`
- **THEN** Gateway 返回 HTTP 400 + 忽略提示
- **AND** 请求不进入主处理链路

#### Scenario: message_type 为 text 且 text 字段缺失时返回 400
- **WHEN** 请求体中 `raw_message.message_type = "text"` 但 `raw_message.text` 字段为 null 或缺失
- **THEN** Gateway 返回 HTTP 400（text 为必填，message_type=text 时）

#### Scenario: 空消息（text 为空字符串或仅含空白字符）时拒绝并视为可忽略输入
- **WHEN** 请求体中 `raw_message.message_type = "text"` 且 `raw_message.text` 为空字符串 `""` 或仅含空白字符（如 `"   "`）
- **THEN** Gateway 返回 HTTP 400
- **AND** 请求不进入主处理链路（不写 `message_events`，不调用 Runtime）（依据 `edge_cases.md:17-18` BR-001）
- **AND** 上游（如 mb-adapter）不应对此类请求进行重试（该 400 属于“可忽略输入”，非瞬时故障）

---

### Requirement: 入站文本长度超限拒绝
Gateway 必须（MUST）在字段校验通过后、进入主链路前，对 `message_type = text` 且 `raw_message.text` 长度 > 4096 字符的入站请求返回 HTTP 400 并附带用户提示，记录长度日志，不进入主链路（`cross_cutting_concepts.md:106-109`、`criterion.md:118-119`、BR-002）。

#### Scenario: 入站文本 > 4096 字符时拒绝并返回用户提示
- **WHEN** 请求体中 `raw_message.message_type = "text"` 且 `raw_message.text` 长度超过 4096 字符（Bearer Token 有效、其余字段完整）
- **THEN** Gateway 返回 HTTP 400
- **AND** 响应体包含用户可读提示（如“消息过长，请缩短后重试”）
- **AND** Gateway 记录长度日志（含实际字符数）
- **AND** 不写入 `message_events`，不调用 Runtime

#### Scenario: 入站文本 = 4096 字符时正常接收
- **WHEN** 请求体中 `raw_message.text` 恰好为 4096 字符
- **THEN** Gateway 正常接收（不返回 400）

---

### Requirement: 统一错误响应格式
Gateway 必须（MUST）对所有 4xx/5xx 错误响应统一使用 `{ "error": "<message>" }` JSON 格式，确保 mb-adapter 可解析（criterion.md §5.1 API 契约；cross_cutting_concepts.md 错误规范）。

#### Scenario: 401 错误响应格式正确
- **WHEN** Bearer Token 无效触发 401
- **THEN** 响应 Content-Type 为 `application/json`
- **AND** 响应体结构为 `{ "error": "<message>" }`

#### Scenario: 429 错误响应格式正确
- **WHEN** 限流触发 429
- **THEN** 响应 Content-Type 为 `application/json`
- **AND** 响应体结构为 `{ "error": "<message>" }`

---

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
