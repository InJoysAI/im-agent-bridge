# 跨切面概念 (Cross-cutting Concepts)

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-13 13:52`
> - **Generator**: `Context-Agent v1.0`

---

## 📊 可观测性 (Observability)

### 日志规范 (TAD §11.1)

日志至少覆盖以下事件：

| 日志事件 | 级别 | 说明 |
|---------|------|------|
| 消息接入 | INFO | 入站消息到达 Gateway |
| 标准化完成 | DEBUG | 消息标准化后的 event_id / session_id |
| session_id 生成/命中 | INFO | 首次生成 vs 已存在 session |
| Runtime 调用 | INFO | 请求发出 + 延迟 |
| MCP 调用 | INFO | 工具名称 + 延迟 |
| 回写结果 | INFO | 成功/失败 + reply_id |
| 错误 | ERROR | 所有异常场景 |
| 被限流请求 | WARN | chat_id + 时间戳 |
| DB 不可用 | ERROR | 系统级告警 |

> ⚠️ 日志禁止记录以下高敏感凭证（任何级别均不得输出明文，违者字段值替换为 `[REDACTED]`）：
> `GATEWAY_BEARER_TOKEN`、`BRIDGE_BEARER_TOKEN`、`TELEGRAM_BOT_TOKEN`、`SHOPIFY_CLIENT_SECRET`、`DATABASE_URL`（含密码部分）、`POSTGRES_PASSWORD`
> 脱敏实现：集中定义 `SENSITIVE_FIELDS` 常量，由 `tracing::Layer` 在序列化前统一遮蔽（`feat-observability-logging`）

### 指标监控 (TAD §11.2)

| 指标名 | 类型 | 说明 |
|--------|------|------|
| `messages_received_total` | Counter | 入站消息总数 |
| `messages_replied_total` | Counter | 成功回复总数 |
| `runtime_call_success_total` | Counter | Runtime 调用成功数 |
| `runtime_call_timeout_total` | Counter | Runtime 超时数 |
| `mcp_call_success_total` | Counter | MCP 调用成功数 |
| `mcp_call_error_total` | Counter | MCP 调用失败数 |
| `reply_write_success_total` | Counter | 回写成功数 |
| `reply_write_error_total` | Counter | 回写失败数 |
| `rate_limited_total` | Counter | 被限流请求数 |
| `db_unavailable_total` | Counter | DB 不可用计数 |
| `runtime_log_write_failures_total` | Counter | runtime_logs 写入失败计数（`feat-persist-runtime-logs` 引入；写入失败不阻断主链路，供运维告警收敛） |

### 分布式追踪 (TAD §11.3)

| 配置项 | 值 |
|--------|------|
| **Trace ID** | 每次消息处理生成统一 `trace_id`；MVP 阶段以 `event_id` 字段名实现（`feat-observability-logging`） |
| **覆盖范围** | Bridge 推送 → Gateway 标准化 → Runtime 调用 → MCP 调用 → 回写 |
| **Gateway 负责** | 8 类 TAD §11.1 必选事件（消息接入/标准化完成/session命中/Runtime调用/回写结果/错误/被限流请求/DB不可用）＋附加埋点（认证/Channel解析） |
| **Runtime 负责** | MCP 调用日志（携带相同 `event_id`，由独立 change 实现） |

---

## ❌ 错误处理 (Error Handling)

### 统一错误码

| error_code | 说明 | 用户提示 |
|-----------|------|---------|
| `RUNTIME_TIMEOUT` | Gateway → Runtime 超时 (15s) | "抱歉，当前无法处理您的请求，请稍后再试。" |
| `RUNTIME_UNAVAILABLE` | Runtime 不可达 | 同上 |
| `RUNTIME_BAD_RESPONSE` | Runtime 响应格式异常 | 同上 |
| `RUNTIME_SESSION_NOT_FOUND` | Runtime 侧会话失效 | Gateway 清空 runtime_session_key 并重建 |
| `MCP_TIMEOUT` | Runtime → MCP 超时 (10s) | "工具暂不可用" |
| `MCP_UNAVAILABLE` | MCP 不可达 | "工具暂不可用"（不暴露技术细节） |

### 重试策略

| 场景 | 策略 | 参数 |
|------|------|------|
| 回写失败（可重试：5xx/429/transport） | 指数退避 | 基础 1s，最大 3 次 (1s/2s/4s)；不可重试（400/401）立即失败不重试；at-most-once 语义（Matterbridge 1.26 不识别 reply_id，见 api_strategy.md §2.5） |
| Matterbridge 入站轮询错误 | 错误退避 | 轮询失败时退避后重试（`feat-runtime-reply-bridge` 联调将 `/api/stream` 改为 `/api/messages` 定期轮询） |
| Runtime session 失效 | 重建一次 | 清空 runtime_session_key → 重新调用 |
| DB 不可用 | 短路熄断 | 不重试，直接 503 |

### 熔断器: PostgreSQL 不可用

| 行为 | 说明 |
|------|------|
| 触发条件 | PostgreSQL 连接失败 |
| 处理方式 | 短路熄断（Circuit Break），所有入站请求返回 503 |
| 禁止行为 | 绝对禁止在无 DB 时向 Runtime 传递上下文 |
| 设计原则 | "宁可报错不可错乱" (PRD §2.3 场景 I) |

---

## 🔐 配置与密钥管理

| 类型 | 存储位置 | 更新方式 |
|------|---------|---------|
| Bridge Bearer Token | 环境变量 | 手动轮换 |
| Telegram Token | 环境变量 / Secret Manager | 手动轮换 |
| PostgreSQL 密码 | 环境变量 | 手动轮换 |
| Shopify MCP 凭证 | `.env` / Secret | 跟随 Runtime/MCP 运行环境 |
| Bot 配置 | PostgreSQL `bots` 表 | SQL / 管理接口 |
| Channel 绑定 | PostgreSQL `channel_bindings` 表 | SQL / 管理接口 |
| MCP 实例声明 | `MEMORY.md` | 手动编辑 |

> ⚠️ PostgreSQL 不存 MCP 配置/密钥引用。密钥跟随 Runtime/MCP 运行环境管理。

---

## 📨 消息标准化规范

### 消息长度约束

| 方向 | 上限 | 处理 |
|------|------|------|
| **入站 (input_text)** | 4096 字符 (Telegram 上限) | Gateway 标准化阶段校验，**超长时拒绝进入主链路 + 返回用户提示 + 记录长度日志**（不截断入站消息，见 BR-002） |
| **出站 (reply text)** | 4096 字符 | Runtime Adapter 硬截断 + 追加截断提示 |
| **持久化 (input_text/output_text)** | 512 字符 | 写入 message_events 时截断（数据最小化） |

### session_id 生成规则

| 场景 | 格式 | 说明 |
|------|------|------|
| 私聊 | `telegram:private:{chat_id}` | 独立 session |
| 群聊 | `telegram:group:{chat_id}` | 共享 session（同一群共用上下文） |

### 非文本消息处理

- 忽略，或返回"当前仅支持文本消息"

---

## 🚦 限流策略

| 维度 | 算法 | 阈值 | 超限行为 |
|------|------|------|---------|
| `chat_id` | Token Bucket | 5 msg/sec/chat_id | 丢弃 + 429 + 记录日志 |

---

## AI 引用指南

当 AI 实现跨切面功能时：
1. 日志必须使用 JSON 结构化格式（`tracing-subscriber` JSON layer）；脱敏 filter 必须屏蔽 `SENSITIVE_FIELDS` 所有高敏感凭证键名（GATEWAY_BEARER_TOKEN / BRIDGE_BEARER_TOKEN / TELEGRAM_BOT_TOKEN / SHOPIFY_CLIENT_SECRET / DATABASE_URL / POSTGRES_PASSWORD），字段值替换为 `[REDACTED]`
2. 必须实现所有 11 个指标 Counter（含 `runtime_log_write_failures_total`）
3. 必须为每次消息处理生成 `trace_id`（MVP 字段名为 `event_id`，贯穿 Gateway 侧 8 类 TAD 必选事件及附加埋点）
4. 错误处理必须使用统一 error_code 枚举
5. 回写重试必须实现指数退避
6. DB 不可用必须短路熄断
7. 入站消息必须实现 chat_id 维度限流
