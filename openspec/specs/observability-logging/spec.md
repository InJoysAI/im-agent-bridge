# observability-logging Specification

## Purpose
TBD

## Requirements
### Requirement: JSON 结构化日志输出
系统必须（MUST）使用 tracing-subscriber 以 JSON 格式输出结构化日志，日志级别覆盖 INFO、WARN、ERROR，通过 `RUST_LOG` 环境变量控制（默认 `info`）。

#### Scenario: 正常消息处理时日志为 JSON 格式
- **WHEN** Gateway 处理一条入站文本消息
- **THEN** 标准输出中写出的日志行为合法 JSON 对象（含 `level`、`message`、`timestamp` 字段）
- **AND** 可通过 `jq .` 无报错解析

#### Scenario: DEBUG 日志在 INFO 级别下不输出
- **WHEN** 环境变量 `RUST_LOG=info`
- **THEN** DEBUG 级别日志不出现在标准输出中
- **AND** INFO / WARN / ERROR 级别日志正常输出

---

### Requirement: 敏感字段脱敏
系统必须（MUST）在 JSON 日志输出中屏蔽所有高敏感凭证，禁止任何日志级别输出以下字段的明文值：`GATEWAY_BEARER_TOKEN`、`BRIDGE_BEARER_TOKEN`、`TELEGRAM_BOT_TOKEN`、`SHOPIFY_CLIENT_SECRET`（及各 MCP 实例等价变量）、`DATABASE_URL`（含密码部分）、`POSTGRES_PASSWORD`。

#### Scenario: Bearer Token 不出现在日志中
- **WHEN** Gateway 收到一条携带有效 Bearer Token 的入站请求
- **THEN** 所有输出日志行均不包含该 Token 的明文值
- **AND** 日志中敏感字段显示为 `[REDACTED]` 或被完全省略

#### Scenario: 错误日志中也不泄露凭证
- **WHEN** Gateway 遭遇 Bearer Token 无效（401）错误并写出 ERROR 日志
- **THEN** 该 ERROR 日志不包含原始 Token 值

---

### Requirement: event_id 贯穿主链路
系统必须（MUST）在每次消息处理主链路的各阶段日志中携带同一 `event_id`。`event_id` 即 TAD §11.3 所定义的 `trace_id` 概念，MVP 阶段以 `event_id` 字段名统一实现。覆盖范围（系统级）：TAD §11.1 定义 9 类必选事件，其中 Gateway 负责 8 类（消息接入 / 标准化完成 / session命中 / Runtime调用 / 回写结果 / 错误 / 被限流请求 / DB不可用）＋附加埋点（Bearer Token认证 / Channel解析）；MCP 调用（第 5 类）归属 Runtime 侧独立 change 实现，需携带相同 `event_id`。

#### Scenario: 入站日志携带 event_id
- **WHEN** Gateway 接收到入站消息并生成 `event_id`
- **THEN** 入站 INFO 日志包含 `event_id` 字段

#### Scenario: 回写日志携带同一 event_id
- **WHEN** Gateway 完成 Runtime 调用并发起回写
- **THEN** 回写阶段日志中的 `event_id` 与同次请求的入站阶段 `event_id` 相同

#### Scenario: 主链路 5 个核心场景均有日志
- **WHEN** 一次完整消息处理链路（入站→认证→Channel解析→Runtime→回写）成功执行
- **THEN** 日志中依次出现涵盖 5 个场景的 INFO 日志条目
- **AND** 所有条目均携带同一 `event_id`

#### Scenario: 跨切面事件日志携带 event_id
- **WHEN** 入站请求触发限流（超过 5 msg/sec/chat_id）
- **THEN** 写出 WARN 日志，字段含 `event_id`、`chat_id`、时间戳

#### Scenario: DB 不可用时写出结构化 ERROR
- **WHEN** Gateway 检测到 PostgreSQL 不可用并触发熔断
- **THEN** 写出 ERROR 日志，字段含 `event_id`

---

### Requirement: 日志必覆盖场景（来自 TAD §11.1）
系统必须（MUST）在以下事件发生时写出对应级别日志：

| 日志事件 | 级别 | 必含字段 |
|---------|------|----------|
| 消息接入（入站消息到达 Gateway） | INFO | `event_id`、`platform`、`chat_id` |
| 标准化完成（生成 event_id / session_id） | DEBUG | `event_id`、`session_id` |
| session_id 生成/命中 | INFO | `event_id`、`session_id` |
| Runtime 调用（请求发出 + 延迟） | INFO | `event_id`、延迟(ms) |
| MCP 调用（工具名称 + 延迟） | INFO | 归属 Runtime 侧实现，本 change 不要求（見 Note） |
| 回写结果（成功/失败 + reply_id） | INFO | `event_id`、`reply_id` |
| 错误（所有异常场景） | ERROR | `event_id`、`error_code` |
| 被限流请求（chat_id + 时间戳） | WARN | `event_id`、`chat_id` |
| DB 不可用（系统级告警） | ERROR | `event_id` |

> **Note**: MCP 调用日志发生在 Runtime 内部（.context/architecture/runtime_view.md），Gateway 无法直接观测。MVP 阶段该事件由 Runtime 侧独立 change 实现，并确保携带相同 `event_id`。

#### Scenario: Runtime 调用日志包含延迟
- **WHEN** Gateway 调用 Runtime 并收到响应
- **THEN** 对应 INFO 日志包含本次调用延迟（毫秒）字段
