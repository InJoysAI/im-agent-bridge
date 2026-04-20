# API 策略 (API Strategy)

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-13 13:52`
> - **Generator**: `Context-Agent v1.0`

---

## 接口总览

| 接口 | 方向 | 协议 | 认证 | 幂等键 |
|------|------|------|------|--------|
| `GET {BRIDGE_URL}/api/messages` | Gateway → Matterbridge（轮询） | HTTP（私有网络） | Bearer Token | N/A |
| `POST /gateway/inbound` | Gateway 内部适配器 → Gateway | HTTP（本机） | Bearer Token | `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id)` |
| `POST {BRIDGE_URL}/api/message` | Gateway → Matterbridge（回写 wire） | HTTP（私有网络） | Bearer Token | 无（at-most-once，见 §2 偏差注） |
| _(SSoT 契约)_ `POST /bridge/reply` | Gateway → Bridge 代理层（保留） | HTTP（私有网络） | Bearer Token | `reply_id` |
| _(内部)_ `bots.runtime_endpoint` | Gateway → Runtime (via RuntimeAdapter) | HTTP (内网) | 无 (MVP，内网) | N/A |

---

## 1. POST /gateway/inbound

### 1.1 接口描述

当前实现中，Matterbridge 负责接收 Telegram 消息并暴露 `GET {BRIDGE_URL}/api/messages`（轮询，`feat-runtime-reply-bridge` 联调后由 `/api/stream` 改为轮询模式）。Gateway 内建 `adapters::matterbridge` 后台任务定期轮询拉取消息，并将其转换为 `POST /gateway/inbound` 的标准请求。入站消息必须包含渠道来源标识，Gateway 根据来源标识查询 `channel_bindings` 解析 `bot_id`。

### 1.2 请求方法

POST `/gateway/inbound`

### 1.3 输入参数

**Header**

| 参数名称 | 必选 | 类型 | 描述 |
|---------|------|------|------|
| Authorization | 是 | String | `Bearer <token>` |

**Body 参数**

| 参数名称 | 必选 | 类型 | 描述 |
|---------|------|------|------|
| platform | 是 | String | 渠道平台标识，如 `telegram` |
| bridge_gateway_name | 是 | String | Bridge 网关名称 |
| bridge_channel_name | 否 | String | Bridge 频道名称（可空，退化匹配） |
| raw_message | 是 | Object | 原始消息对象 |
| raw_message.chat_id | 是 | String | 聊天 ID |
| raw_message.chat_type | 是 | String | `private` / `group` |
| raw_message.user_id | 是 | String | 发送者 ID |
| raw_message.message_type | 是 | String | `"text"` \| `"image"` \| `"audio"` \| `"video"` \| `"file"` \| `"other"`（以 `SSoT/api/main.tsp` 为准） |
| raw_message.text | 否 | String | 消息文本（仅 `message_type=text` 时携带；非文本类型 Gateway 返回 400 + 忽略提示，BR-001） |
| raw_message.timestamp | 是 | String | ISO 8601 时间戳 |
| raw_message.message_id | 是 | String | 平台原始消息 ID（持久化字段名：`bridge_message_id`） |
| raw_message.sender_name | 否 | String | 发送者名称 |

### 1.4 响应状态码

| 状态码 | 语义 |
|--------|------|
| `200 OK` | 消息已接收并处理、重复消息已跳过，或群聊无 mention 被预期过滤（`status` 区分） |
| `400 Bad Request` | 请求格式错误或缺少必要字段 |
| `401 Unauthorized` | Bearer Token 无效 |
| `404 Not Found` | 未找到匹配的 `channel_bindings` 记录 |
| `409 Conflict` | 重复的 bridge_message_id（严格幂等模式，可选） |
| `429 Too Many Requests` | 超过限流阈值（5 msg/sec/chat_id） |
| `500 Internal Server Error` | 服务器内部错误 |
| `502 Bad Gateway` | Runtime 不可达 |
| `503 Service Unavailable` | PostgreSQL 不可用，短路熄断 |

### 1.5 当前实现说明

- Gateway 通过 `BRIDGE_URL` 轮询 Matterbridge `GET /api/messages`
- `gateway/src/adapters/matterbridge.rs` 过滤 `protocol="api"` 或 `account` 以 `api.` 开头的消息，避免回环
- 适配器将消息转为内部 `POST http://localhost:8080/gateway/inbound`

### 1.6 幂等策略

- **幂等键**: `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id)`
- **落地**: `message_events` 表唯一索引 `uq_message_events_inbound_dedup`（含 COALESCE 降级匹配）
- **字段映射**: 入站协议字段 `raw_message.message_id` → 持久化字段 `bridge_message_id`
- **重复处理**: 默认返回 `200 OK` + `{ "status": "ignored_duplicate" }`
- **群聊 mention 过滤**: 当 `chat_type=group` 且 bot 配置 `require_mention=true`，消息未命中 `@{telegram_username}` 时返回 `200 OK` + `{ "status": "ignored_no_mention" }`，并且不写 `message_events`、不调用 Runtime

### 1.8 群聊无 @mention 示例（`ignored_no_mention`）

```bash
curl -X POST http://localhost:8080/gateway/inbound \
  -H "Authorization: Bearer <gateway_token>" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "telegram",
    "bridge_gateway_name": "tg-gateway",
    "raw_message": {
      "chat_id": "-1001234567890",
      "chat_type": "group",
      "user_id": "u-10001",
      "message_type": "text",
      "text": "大家下午好",
      "timestamp": "2026-04-17T10:00:00Z",
      "message_id": "tg-msg-9001"
    }
  }'
```

响应示例：

```json
{
  "status": "ignored_no_mention"
}
```

### 1.7 bot_id 解析逻辑

1. 优先按 `platform + bridge_gateway_name + bridge_channel_name` 查找 `channel_bindings`
2. 若 `bridge_channel_name` 为空，退化为 `platform + bridge_gateway_name`
3. 若仍找不到，拒绝处理并记录绑定缺失错误日志

---

## 2. POST /bridge/reply

### 2.1 接口描述

Gateway 调用 Bridge API 将文本消息回写到 Telegram。

> **实现状态（更新 2026-04-17）**: `feat-runtime-reply-bridge` 已完成回写链路。由于 `mb-adapter` 代理层在联调阶段确认不引入，实际 wire 端点改为 Matterbridge 1.26 原生 `POST {BRIDGE_URL}/api/message`（详见 §2.2 偏差注与 §2.5 at-most-once 说明）。`POST /bridge/reply` 保留为 SSoT 内部契约，待独立 change `fix-bridge-reply-ssot-align` 对齐。

### 2.2 请求方法

**SSoT 契约**: POST `/bridge/reply`

> **联调偏差注（feat-runtime-reply-bridge, 2026-04-17；fix-bridge-reply-chat-routing, 2026-04-20）**: 实际 wire 端点为 `POST {BRIDGE_URL}/api/message`（Matterbridge 1.26 原生）。Wire payload: `{gateway: bridge_gateway_name, channel: chat_id, text, username?}`（`config.Message` 子集）。`channel` 字段 MUST 等于来源 `BridgeReplyPayload.chat_id`（语义标注/与 inout 配置保持一致性），禁止省略或使用固定值（如 `"api"`）。**E2E 验证确认** Matterbridge 1.26 intra-gateway 路由对来自 `api.*` 的消息会广播至同一 gateway 下所有 telegram `inout`，且 `channel` 字段不用于 inout 过滤；因此 **BR-012 隔离** 需通过 `matterbridge.toml` 将群聊/私聊拆分为独立 gateway（每个 gateway 仅含一个 telegram `inout` + 一个 `api.myapi` `inout`）从结构上消除跨 chat 广播。Gateway 内部 `BridgeReplyPayload` 仍按 SSoT `ReplyRequest` 字段（reply_id/chat_id/platform/bridge_gateway_name/bridge_channel_name）组织，由 `to_matterbridge_message` 在发送前做字段映射；`reply_id` 仅用于日志追踪，Matterbridge 1.26 不识别该字段。

### 2.3 输入参数

**Body 参数**

| 参数名称 | 必选 | 类型 | 描述 |
|---------|------|------|------|
| reply_id | 是 | String | 回复 ID（幂等键，Gateway 生成） |
| chat_id | 是 | String | 目标聊天 ID |
| platform | 是 | String | 目标平台 |
| text | 是 | String | 回复文本（≤ 4096 字符） |
| bridge_gateway_name | 是 | String | 目标 Bridge 网关 |
| bridge_channel_name | 否 | String | 目标 Bridge 频道 |

### 2.4 响应状态码

| 状态码 | 语义 |
|--------|------|
| `200 OK` | 回写成功 |
| `400 Bad Request` | 请求格式错误 |
| `401 Unauthorized` | Bearer Token 无效 |
| `404 Not Found` | 投递目标不可用 |
| `409 Conflict` | 重复 reply_id（视为成功语义） |
| `500 Internal Server Error` | Bridge 内部错误 |
| `502 Bad Gateway` | Bridge 无法发送到 Telegram |

> **联调偏差注（2026-04-17）**: 当前 wire 端点 `POST /api/message`（Matterbridge 1.26）实际返回 200（成功）、401（未授权）、5xx（服务错误）；**不返回 409**（不识别 `reply_id`）。上表状态码为 SSoT 契约，代理层恢复后生效。

### 2.5 幂等策略

- **幂等键**: `reply_id`（SSoT 契约；当前 wire 端点 `/api/message` 不识别此字段）
- **行为**: Bridge 对同一 `reply_id` 跳过重复投递（代理层恢复后生效）
- **409 语义**: Gateway 将 `409` 视为"回写已完成/无需重试"
- **当前 at-most-once 保证**: 仅对可重试错误（5xx/429/transport 错误）触发指数退避重试（1s/2s/4s）；验收中未观测到重复消息；若后续发现重复，应恢复 `mb-adapter` 或在 Matterbridge 侧引入 dedup

---

## 3. Gateway ↔ Runtime 调用策略 (TAD §9.2)

### 3.1 调度机制

Gateway **不定义中间抽象端点**，而是通过 `bots.runtime_type` + `bots.runtime_endpoint` **直接调用 Runtime 原生 HTTP 接口**。

```
bots 表:
  runtime_type     = "nanobot"
  runtime_endpoint = "http://nanobot:8900/v1/chat/completions"
```

Gateway 内部按 `runtime_type` 分发到对应的 `RuntimeAdapter` 实现（Strategy Pattern）：

| runtime_type | Adapter 实现 | 调用的原生端点 |
|-------------|-------------|--------------|
| `nanobot` | `NanoBotAdapter` | `POST {runtime_endpoint}` (OpenAI-style response format，**服务端自管理对话历史**) |
| _(未来)_ `zeroclaw` | `ZeroClawAdapter` | ZeroClaw 原生 API |

### 3.2 NanoBotAdapter 协议适配 (MVP 默认)

> **关键机制**：NanoBot 服务端通过 `session_id` **自动维护完整对话历史**（含短期上下文 + 长期记忆整合）。Gateway 每次仅发送当前消息，**无需在 Gateway 侧存储或重放历史记录**。

| Gateway 内部字段 | NanoBot 请求字段 | 约束 |
|----------------|-----------------|------|
| `text` | `messages: [{"role": "user", "content": text}]` | **严格限 1 条**，传多条或 0 条返回 HTTP 400 |
| `session_id` | `session_id` | **必传**，缺省时 NanoBot 退为 `"api:default"`，导致所有会话串扰 |
| _(可选)_ `model` | `model` | 若传入必须与 NanoBot 服务端配置的 `model_name` 一致（默认 `"nanobot"`）；不匹配返回 HTTP 400 |
| 响应 | `choices[0].message.content` | 从 NanoBot 返回的 OpenAI-style 结构中提取 |

> **注意**：NanoBot `/v1/chat/completions` 当前**不支持 Streaming**，传入 `stream: true` 服务端直接返回 **HTTP 400**。

#### 请求示例 (Gateway → NanoBot)

```http
POST http://nanobot:8900/v1/chat/completions
Content-Type: application/json

{
  "model": "nanobot",
  "messages": [
    { "role": "user", "content": "今天的销售额是多少？" }
  ],
  "session_id": "telegram:private:123456789"
}
```

> `session_id` 在 NanoBot 内部以 `api:{session_id}` 作为会话键存储（即 `api:telegram:private:123456789`），Gateway 传原始值即可。

#### NanoBot 原始响应示例

```json
{
  "id": "chatcmpl-a1b2c3d4e5f6",
  "object": "chat.completion",
  "created": 1744516800,
  "model": "nanobot",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "今天共有 23 单，销售额为 1,250 USD。"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": { "prompt_tokens": 0, "completion_tokens": 0, "total_tokens": 0 }
}
```

> - Gateway 提取 `choices[0].message.content` 作为回复文本，封装为 §3.3 标准回复对象后发送给 Bridge。
> - `usage` 字段始终为 **全零**（NanoBot 服务端不计算 token 用量）。
> - 若 NanoBot 返回空内容，服务端会自动重试一次，仍为空则返回内置 fallback 文案。

#### NanoBot 错误响应格式

NanoBot 对所有异常均以 JSON 错误体 + 对应 HTTP 状态码响应，**不使用 `choices` 字段**：

```json
{ "error": { "message": "Only a single user message is supported", "type": "invalid_request_error", "code": 400 } }
```

| HTTP 状态码 | 触发场景 |
|------------|--------|
| `400` | `messages` 数量 ≠ 1、`role` 非 `user`、`stream=true`、`model` 不匹配 |
| `504` | 单次请求超出 NanoBot 服务端超时（默认 **120s**） |
| `500` | NanoBot 内部异常 |

> **超时分层说明**：Gateway 对 NanoBot 的 HTTP 调用设置 **15s hard timeout**（`RUNTIME_TIMEOUT`），NanoBot 服务端自身超时为 120s。通常是 Gateway 先超时并返回错误，NanoBot 侧才到达其 120s 上限。

#### 其他可用端点

| 端点 | 方法 | 用途 |
|------|------|------|
| `/v1/chat/completions` | POST | 消息处理（主链路） |
| `/v1/models` | GET | 查询可用模型（返回配置的 `model_name`） |
| `/health` | GET | 健康检查（返回 `{"status": "ok"}`） |

### 3.3 标准回复对象 Schema

无论使用哪种 `runtime_type`，Adapter 必须将 Runtime 原生响应统一转换为以下格式：

```json
{
  "reply_id": "rep_20260412_xxx",
  "bot_id": "550e8400-e29b-41d4-a716-446655440000",
  "platform": "telegram",
  "chat_id": "123456789",
  "reply_type": "text",
  "text": "今天共有 23 单，销售额为 1,250 USD。",
  "session_id": "telegram:private:123456789",
  "status": "success",
  "metadata": {
    "runtime": "nanobot"
  }
}
```

### 3.4 错误响应 Schema

```json
{
  "reply_id": "rep_20260412_xxx",
  "bot_id": "550e8400-e29b-41d4-a716-446655440000",
  "platform": "telegram",
  "chat_id": "123456789",
  "reply_type": "text",
  "text": "抱歉，当前无法处理您的请求，请稍后再试。",
  "session_id": "telegram:private:123456789",
  "status": "error",
  "metadata": {
    "runtime": "nanobot",
    "error_code": "RUNTIME_TIMEOUT",
    "error_message": "Runtime did not respond within 15s"
  }
}
```

### 3.5 error_code 枚举 (MVP)

| error_code | 说明 |
|-----------|------|
| `RUNTIME_TIMEOUT` | Gateway 侧 HTTP 调用 NanoBot 超时（Gateway hard timeout: **15s**；NanoBot 服务端自身超时: 120s） |
| `RUNTIME_UNAVAILABLE` | Runtime 不可达/拒绝连接 |
| `RUNTIME_BAD_RESPONSE` | Runtime 响应格式不符合 Schema |
| `RUNTIME_SESSION_NOT_FOUND` | Runtime 侧会话不存在/已失效。Gateway 应按 §7.3.1 清空 `runtime_session_key` 并重建一次 |
| `MCP_TIMEOUT` | Runtime → MCP 超时（10s） |
| `MCP_UNAVAILABLE` | MCP 不可达/报错 |



## 4. 安全规范

### 4.1 Token / 认证

- 认证方式：Bearer Token（Bridge ↔ Gateway）
- Token 存储：环境变量
- Gateway ↔ Runtime：无认证 (MVP)，内网运行

### 4.2 请求安全

- 传输安全：HTTPS（Bridge ↔ Gateway）
- 来源限制：白名单 + localhost/内网
- 限流：Token Bucket，5 msg/sec/chat_id

### 4.3 数据安全

- 敏感字段 (input_text/output_text)：截断至 512 字符存储
- runtime_logs payload：仅错误时写入，脱敏 PII
- 审计日志：日志查询应记录审计日志

---

## AI 引用指南

当 AI 生成 API 相关代码时：
1. 入站接口必须实现幂等去重（唯一索引）
2. 回写接口必须实现 reply_id 幂等
3. 入站必须实现 Bearer Token 校验
4. 入站必须实现 chat_id 维度限流
5. bot_id 由 Gateway 从 channel_bindings 解析，不由外部传入
6. Runtime Adapter 负责超长文本截断（4096 字符）
7. NanoBotAdapter 调用时 `session_id` 字段**必传**，禁止省略；Gateway 无需维护对话历史，NanoBot 服务端按 session_id 自动管理
