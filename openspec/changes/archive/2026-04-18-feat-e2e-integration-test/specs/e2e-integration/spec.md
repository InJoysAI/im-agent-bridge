## ADDED Requirements

### Requirement: 端到端主链路闭环验证
系统必须（MUST）在完整部署环境下（Docker Compose 一键启动）支持 Telegram 文本消息经 Matterbridge → Gateway → NanoBot → Shopify MCP → 回写的完整主链路闭环，且端到端 P95 响应时间 ≤ 5s。

#### Scenario: 文本消息完整闭环
- **WHEN** 系统完整部署（Telegram Bot + Matterbridge + Gateway + NanoBot + Shopify MCP），用户在 Telegram 发送文本消息
- **THEN** 用户在 Telegram 收到 AI 回复
- **AND** `message_events.reply_status = "success"`
- **AND** 端到端响应时间 P95 ≤ 5s（`criterion.md §8`）

#### Scenario: DoD 成功率达标（接入 / 回写 / MCP）
- **WHEN** 完成 20 条独立文本消息端到端采样验证（P95 ≤ 5s 基准轮）
- **THEN** 接入成功率（Gateway 成功收到并处理）≥ 95%（`testing_strategy.md §DoD`）
- **AND** 回写成功率（Telegram 成功收到 AI 回复）≥ 95%（`testing_strategy.md §DoD`）
- **AND** MCP 工具调用成功率（正常 MCP 场景下）≥ 90%（`testing_strategy.md §DoD`）

#### Scenario: 非文本消息不进入主链路
- **WHEN** 用户在 Telegram 发送图片或贴纸等非文本消息
- **THEN** 消息不进入主处理链路，系统不产生异常，不调用 Runtime

---

### Requirement: Docker Compose 全服务健康启动
系统必须（MUST）在执行 `docker compose up -d` 后，全部服务（Gateway、NanoBot、PostgreSQL、Matterbridge）通过各自 health check，Gateway 日志无 DB 连接失败或 Bearer Token 配置缺失。

#### Scenario: 一键启动后全服务健康
- **WHEN** 执行 `docker compose up -d`
- **THEN** Gateway health check 返回 HTTP 200
- **AND** NanoBot 可接受请求
- **AND** PostgreSQL 连接正常
- **AND** Matterbridge 已连接 Telegram（Edge Server）

---

### Requirement: P0 BDD 场景验证——消息标准化与 bot_id 解析
系统必须（MUST）通过 `testing_strategy.md` 模块 3 定义的标准化场景：标准消息对象包含全部必须字段（`event_id / platform / chat_id / chat_type / user_id / session_id / text / timestamp / bot_id`），bot_id 由 `channel_bindings` 查询解析，不接受外部传入。

#### Scenario: bot_id 由 channel_bindings 解析
- **WHEN** Gateway 收到入站消息，根据 `(platform, bridge_gateway_name, bridge_channel_name)` 查询 `channel_bindings`
- **THEN** 解析出 `bot_id`，不接受外部请求直接传入 `bot_id`（BR-004）

#### Scenario: 标准消息对象字段完整
- **WHEN** Bridge 转发一条 Telegram 文本消息至 Gateway
- **THEN** 标准化后的消息对象包含所有必须字段：`event_id / platform / chat_id / chat_type / user_id / session_id / text / timestamp / bot_id`

#### Scenario: 入站超长消息拒绝
- **WHEN** 用户发送长度超过 4096 字符的文本消息
- **THEN** Gateway 拒绝处理，返回"消息过长，请缩短后重试"提示，记录日志含原始消息长度（BR-002）
- **AND** 消息不进入主处理链路，不调用 Runtime

---

### Requirement: P0 BDD 场景验证——会话边界管理
系统必须（MUST）通过 `testing_strategy.md` 模块 6 定义的会话边界场景：私聊与群聊 session_id 正确生成且严格隔离。

#### Scenario: 私聊 session_id 生成
- **WHEN** 用户在私聊中发送消息，Gateway 生成 session_id
- **THEN** `session_id = "telegram:private:{chat_id}"`（BR-010）

#### Scenario: 群聊共享 session_id
- **WHEN** 同一群聊中两个用户分别发送消息，Gateway 分别生成 session_id
- **THEN** 两条消息的 `session_id` 相同，均为 `"telegram:group:{chat_id}"`（BR-011/013）

#### Scenario: 私聊与群聊上下文严格隔离
- **WHEN** 同一用户在私聊和群聊分别发送消息
- **THEN** 私聊 `session_id = "telegram:private:{chat_id}"`，群聊 `session_id = "telegram:group:{chat_id}"`，两个上下文彼此隔离（BR-012）

---

### Requirement: P0 BDD 场景验证——幂等去重
系统必须（MUST）通过 `testing_strategy.md` 模块 8 场景 1：相同幂等键的入站消息到达时，识别为重复，不重复写入 `message_events`，不重复调用 Runtime（BR-042）。

#### Scenario: 重复消息幂等去重
- **WHEN** 相同 `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id)` 的消息再次到达 Gateway
- **THEN** Gateway 识别为重复，返回 `ignored_duplicate`
- **AND** 不重复写入 `message_events`，不调用 Runtime（BR-042）

---

### Requirement: P0 BDD 场景验证——限流 429
系统必须（MUST）通过 `testing_strategy.md` 模块 8 场景 2：同一 `chat_id` 在 1 秒内第 6 条消息触发 Token Bucket 限流，返回 HTTP 429，不调用 Runtime，不写 `message_events`（BR-055）。

#### Scenario: 限流触发 429
- **WHEN** 同一 `chat_id` 在 1 秒内发送第 6 条消息
- **THEN** Gateway 返回 HTTP 429 Too Many Requests
- **AND** 该消息不进入主处理链路，不调用 Runtime，不写 `message_events`（BR-055）

---

### Requirement: 异常注入——Runtime 超时
系统必须（MUST）在 NanoBot 停止时，Gateway 调用 Runtime 超过 15s 无响应后触发 `RUNTIME_TIMEOUT`，向用户回写错误提示"抱歉，当前无法处理您的请求，请稍后再试。"，并记录错误日志（含 trace_id，不含敏感凭证）。

#### Scenario: NanoBot 停止触发 Runtime 超时
- **WHEN** NanoBot 容器停止，Gateway 调用 Runtime 超过 15s 无响应
- **THEN** Gateway 触发 `RUNTIME_TIMEOUT`，向 Telegram 回写"抱歉，当前无法处理您的请求，请稍后再试。"
- **AND** 记录错误日志（含 trace_id，不含用户原文或敏感凭证）（BR-051）

#### Scenario: RUNTIME_SESSION_NOT_FOUND 会话重建
- **WHEN** NanoBot 重启导致会话失效，Gateway 收到 `RUNTIME_SESSION_NOT_FOUND`
- **THEN** Gateway 清空 `sessions.runtime_session_key` 并重建会话，下一条消息可正常处理

---

### Requirement: 异常注入——DB 熔断 503
系统必须（MUST）通过 `testing_strategy.md` 模块 8 场景 3：PostgreSQL 不可达时，Gateway 立即短路熔断，所有入站请求返回 HTTP 503，向用户返回"系统暂时不可用，请稍后重试"，记录系统级告警日志（BR-041）。

#### Scenario: PostgreSQL 停止触发熔断 503
- **WHEN** PostgreSQL 服务停止，Gateway 收到任一入站消息
- **THEN** Gateway 立即返回 HTTP 503 Service Unavailable
- **AND** 向用户返回"系统暂时不可用，请稍后重试"（BR-041）
- **AND** 不继续处理任何业务请求（短路熔断）
- **AND** 记录系统级告警日志

---

### Requirement: 超长回复截断（BR-003）
系统必须（MUST）在 Runtime 返回超过 4096 字符的回复时，截断至 4096 字符并追加省略标记后发送给用户，不崩溃，不静默丢失（BR-003 / `criterion.md §3.4`）。

#### Scenario: 超长回复截断与提示
- **WHEN** NanoBot 返回超过 4096 字符的回复内容
- **THEN** Gateway 截断至 4096 字符并追加“…[内容已截断]”标记
- **AND** Telegram 成功收到截断后的消息，无异常（BR-003）
- **AND** Gateway 日志记录截断事件（含 trace_id）

---

### Requirement: 回写幂等与重试
系统必须（MUST）在回写 Bridge 时使用 `reply_id` 作为幂等键，指数退避最多 3 次（1s/2s/4s），HTTP 409 视为成功；同一 `reply_id` 不得重复发送（BR-062）。

#### Scenario: 回写指数退避重试
- **WHEN** Gateway 回写 Bridge 首次失败（非 409）
- **THEN** Gateway 按 1s / 2s / 4s 指数退避重试，最多 3 次
- **AND** HTTP 409 响应视为成功，不继续重试

#### Scenario: 回写幂等防重复
- **WHEN** Gateway 因异常再次使用相同 `reply_id` 尝试回写
- **THEN** 检测到 `reply_id` 唯一约束冲突，不重复发送
- **AND** 记录幂等重复回写告警日志

---

### Requirement: 安全边界验证——Bearer Token 认证拦截
系统必须（MUST）拦截不携带有效 Bearer Token 或 Authorization 头的入站请求，返回 HTTP 401 Unauthorized，请求不进入主处理链路（BR-031）。

#### Scenario: 未授权请求返回 401
- **WHEN** 向 Gateway 发送不含 Authorization 头或携带无效 Bearer Token 的请求
- **THEN** Gateway 返回 HTTP 401 Unauthorized
- **AND** 请求不进入主处理链路（不写 `message_events`，不调用 Runtime）（BR-031）

---

### Requirement: 可观测性验证——trace_id 全链路贯通与指标覆盖
系统必须（MUST）在每次消息处理时生成 trace_id 贯通完整链路（Bridge → Gateway → RuntimeAdapter → MCP → 回写），且全部 10 个 Counter 指标在完整联调后均有数据写入。

#### Scenario: trace_id 全链路贯通
- **WHEN** 处理一条完整的文本消息（主链路闭环）
- **THEN** 日志中 Gateway → RuntimeAdapter → 回写均携带相同 `trace_id`（`cross_cutting_concepts.md`）

#### Scenario: 10 个 Counter 指标可查询
- **WHEN** 完成 BDD 场景执行（含正常、限流、超时、DB 熔断场景）
- **THEN** 全部 10 个 Counter 可查询且有数据：`messages_received_total` / `messages_replied_total` / `runtime_call_success_total` / `runtime_call_timeout_total` / `mcp_call_success_total` / `mcp_call_error_total` / `reply_write_success_total` / `reply_write_error_total` / `rate_limited_total` / `db_unavailable_total`

---

### Requirement: specflow 门禁——提案归档完整性
系统必须（MUST）在 `feat-e2e-integration-test` 执行前确认：（1）全部直接前置提案（`feat-runtime-reply-bridge`, `feat-nanobot-shopify-mcp`, `feat-observability-logging`, `feat-infra-matterbridge-deploy`）已完成 `specflow validate + archive` 归档；（2）`openspec/changes/archive/` 目录与路线图实施清单一致（criterion.md §6 门禁要求）。

#### Scenario: 前置提案全部归档
- **WHEN** 开始执行 feat-e2e-integration-test 联调任务
- **THEN** 全部直接前置提案在 `openspec/changes/archive/` 目录下均有对应归档记录

#### Scenario: 路线图归档清单一致
- **WHEN** 核查 `openspec/changes/archive/` 与 `openspec/proposal-roadmap.md` 实施清单
- **THEN** archive 目录中的 change-id 集合覆盖路线图已标记为实施完成的全部提案
- **AND** 无路线图已完成但 archive 目录缺失的提案记录（criterion.md §6 全量核查要求）
