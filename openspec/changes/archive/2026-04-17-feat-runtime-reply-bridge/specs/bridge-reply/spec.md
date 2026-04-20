## ADDED Requirements

> **实施偏差脚注（Implementation Deviation, 联调 2026-04-17）**：SSoT `SSoT/api/main.tsp` 所声明的 `Bridge.reply` 端点 `POST {BRIDGE_URL}/bridge/reply` 仍是本规范的**内部契约**（代表 Gateway 发出的语义回写请求），用于保留未来恢复 Bridge 代理层的接缝。但由于 `feat-infra-matterbridge-deploy` 阶段取消了 `mb-adapter` 中间层，本 change 的实际 wire 协议直接对接 Matterbridge 1.26 原生端点 `POST {BRIDGE_URL}/api/message`，payload 被映射为 Matterbridge `config.Message` 子集 `{gateway, text, username?}`。下列 Requirements/Scenarios 的语义在两种端点下同时适用；Scenario 中的 HTTP 状态码以实际 wire 响应为准（当前 Matterbridge 1.26 返回 200 成功，401/404/5xx 失败；不会返回 409）。SSoT 对齐留给后续独立 change（详见 proposal.md §实施偏差说明）。

### Requirement: Bridge 回写 HTTP 调用
系统必须（MUST）通过 HTTP POST 并携带 `Authorization: Bearer <BRIDGE_BEARER_TOKEN>` 将回复文本回写至 Matterbridge（BR-031，criterion.md §3.4，api_strategy.md §2）。

> **端点与 payload**：当前实现 wire 端点为 `POST {BRIDGE_URL}/api/message`（Matterbridge 1.26 原生），wire payload 为 `{gateway: bridge_gateway_name, text, username?}`。Gateway 内部 `BridgeReplyPayload` 仍按 SSoT `ReplyRequest` 字段组织（含 `reply_id` 用于日志追踪）。

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
