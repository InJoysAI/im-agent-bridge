# bridge-reply Specification

## Purpose
TBD

## Requirements
### Requirement: Bridge 回写 HTTP 调用
系统必须（MUST）通过 HTTP POST 并携带 `Authorization: Bearer <BRIDGE_BEARER_TOKEN>` 将回复文本回写至 Matterbridge（BR-031，criterion.md §3.4，api_strategy.md §2）。

> **端点与 payload**：当前实现 wire 端点为 `POST {BRIDGE_URL}/api/message`（Matterbridge 1.26 原生），wire payload 为 `{gateway: bridge_gateway_name, channel: chat_id, text, username?}`（`config.Message` 子集）。`channel` 字段 MUST 等于来源 `BridgeReplyPayload.chat_id`，禁止省略或使用固定值——省略导致 Matterbridge 在同 gateway 多 inout 场景广播，违反 BR-012 隔离语义（fix-bridge-reply-chat-routing, 2026-04-19）。Gateway 内部 `BridgeReplyPayload` 仍按 SSoT `ReplyRequest` 字段组织（含 `reply_id` 用于日志追踪）。

#### Scenario: 回写成功
- **WHEN** Gateway 调用回写端点，Bridge 返回 HTTP 2xx（当前 wire：`POST /api/message` → 200）
- **THEN** `message_events.reply_status` 更新为 `"success"`
- **AND** `reply_write_success_total` 计数器递增 1

#### Scenario: 回写返回 409（幂等成功）
- **WHEN** Gateway 调用回写端点，对方返回 HTTP 409（重复 reply_id）
- **THEN** 视为回写已完成，`message_events.reply_status` 更新为 `"success"`
- **AND** 不触发后续重试

> **联调注**：Matterbridge 1.26 `/api/message` 不识别 `reply_id`，不会返回 409；此 Scenario 在当前 wire 下不可观测，保留作为 Bridge 代理层（`mb-adapter`）恢复后的契约。`bridge_client` 代码仍保留 409 防御性分支对接单测 mock。

---

### Requirement: channel 定向路由（禁止 gateway 级广播）
系统必须（MUST）在向 Matterbridge 发送回写请求时，将 `channel` 字段绑定为来源消息的 `chat_id`，确保回复只投递到对应 `inout`，不得向同 gateway 下其他 `inout` 广播（BR-012，api_strategy.md §2.2）。

#### Scenario: 群聊触发只回写到群聊 inout
- **GIVEN** Matterbridge gateway 同时配置群聊 inout（channel=GROUP_CHAT_ID）与私聊 inout（channel=PRIVATE_CHAT_ID）
- **WHEN** 用户在群聊（chat_id=GROUP_CHAT_ID）中触发 Bot 回复
- **THEN** `POST /api/message` 请求体中 `channel = GROUP_CHAT_ID`
- **AND** 群聊收到 Bot 回复，私聊不收到

#### Scenario: 私聊触发只回写到私聊 inout
- **GIVEN** 同上双 inout 环境
- **WHEN** 用户在私聊（chat_id=PRIVATE_CHAT_ID）中触发 Bot 回复
- **THEN** `POST /api/message` 请求体中 `channel = PRIVATE_CHAT_ID`
- **AND** 私聊收到 Bot 回复，群聊不收到

---

### Requirement: 指数退避重试（仅对可重试错误）
系统必须（MUST）在 Bridge 回写遇到**可重试错误**（网络错误 / 超时 / HTTP 5xx / 429）时按 1s → 2s → 4s 指数退避重试 3 次（初始调用 + 3 次重试 = 最多 4 次总尝试），4 次全部失败后标记失败；对**不可重试错误**（HTTP 400 / 401）必须立即失败，不得重试（BR-062，criterion.md §3.4，cross_cutting_concepts.md 回写重试规范）。

> **说明**：HTTP 5xx / 429 不在 `SSoT/api/main.tsp` 声明的 `/bridge/reply` 响应码集合（200/400/401/409）内，但实际网络与中间件仍可能返回这些状态码，本实现将其作为非契约内的意外响应进行健壮处理，统一归入"可重试错误"分支走退避逻辑。

#### Scenario: 首次失败后重试并最终成功
- **WHEN** Bridge 第 1 次调用返回可重试错误（如 HTTP 503），等待 1s 后第 2 次调用返回 HTTP 200
- **THEN** `message_events.reply_status` 更新为 `"success"`
- **AND** 重试过程中不重复写入 `message_events` 记录

#### Scenario: 4 次尝试全部失败后标记 reply_failed
- **WHEN** Bridge 经历初始调用 + 3 次重试（等待序列 1s → 2s → 4s）均返回可重试错误
- **THEN** `message_events.reply_status` 更新为 `"reply_failed"`
- **AND** 记录错误日志（含 reply_id 与最后一次错误信息）
- **AND** `reply_write_error_total` 计数器递增 1

#### Scenario: 不可重试错误立即失败
- **WHEN** Bridge 返回 HTTP 401（Unauthorized）或 HTTP 400（Bad Request）
- **THEN** 立即标记 `message_events.reply_status = "reply_failed"`，不进入重试循环
- **AND** 记录错误日志（含 reply_id 与 HTTP 状态码），提示配置错误需人工介入
- **AND** `reply_write_error_total` 计数器递增 1

---

### Requirement: output_text 落库截断
系统必须（MUST）在将回复文本写入 `message_events.output_text` 前截断至 512 字符，不得全量存储超长内容（BR-070，criterion.md §4 数据治理）。

#### Scenario: 回复文本超 512 字符时截断落库
- **WHEN** NanoBot 返回的回复文本长度超过 512 字符
- **THEN** `message_events.output_text` 存储内容不超过 512 字符
- **AND** 截断操作不破坏 UTF-8 字符边界

#### Scenario: 回复文本未超 512 字符时原样落库
- **WHEN** NanoBot 返回的回复文本长度 ≤ 512 字符
- **THEN** `message_events.output_text` 完整存储，无截断

---

### Requirement: 回写失败错误可见性
系统必须（MUST）在回写最终失败时输出可追溯的错误日志，不得静默失败（BR-063，criterion.md §4）。

#### Scenario: 回写失败后日志可追溯
- **WHEN** Bridge 回写最终失败（不可重试立即失败或 4 次尝试均失败）
- **THEN** 错误日志中包含 reply_id、最后一次错误原因（HTTP 状态码或网络错误类型）
- **AND** `reply_failed` 状态可通过 `message_events` 表查询验证

---

### Requirement: 日志脱敏（禁止记录 Bearer Token）
系统必须（MUST NOT）在任何日志、指标标签、错误响应或事件记录中写入 `Authorization` 请求头或 `BRIDGE_BEARER_TOKEN` 的值（BR-030，criterion.md §4 安全约束，RISK-006 凭证泄露）。

#### Scenario: 请求日志不得包含 Bearer Token
- **WHEN** Gateway 记录 Bridge 回写的请求/响应日志
- **THEN** 日志字段不包含 `Authorization` 头值或 `BRIDGE_BEARER_TOKEN` 字面值
- **AND** 日志包含 reply_id 以便追溯

---

### Requirement: 回写 payload text ≤ 4096 字符
系统必须（MUST）保证发送至 `POST /bridge/reply` 的 `text` 字段长度不超过 4096 字符（BR-003，criterion.md §3.4/§3.5）。若 RuntimeAdapter 已截断则信任其结果；否则 Gateway 在 bridge_client 入口兜底截断并附加截断提示。

#### Scenario: 入参超 4096 字符时兜底截断
- **WHEN** 传入 `bridge_client::post_reply` 的 text 长度超过 4096 字符
- **THEN** payload `text` 字段截断至 4096 字符（含截断提示）后再发送
- **AND** 不破坏 UTF-8 字符边界

#### Scenario: 入参未超 4096 字符时原样发送
- **WHEN** 传入 text 长度 ≤ 4096 字符
- **THEN** payload `text` 原样发送，无截断
