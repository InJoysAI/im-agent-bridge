# 运行时视图 (Runtime View)

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-13 13:52`
> - **Generator**: `Context-Agent v1.0`

---

## 🎯 场景选择原则

仅记录具有**架构显著性**的运行时场景：
- 核心业务链路（消息接入主链路）
- 异常与恢复流程（Runtime 异常、MCP 失败、回写失败、DB 不可用）
- 超时预算链路

---

## 📊 场景 1: 消息接入主链路

### 序列图

```mermaid
sequenceDiagram
    autonumber
    participant U as Telegram User
    participant MB as Matterbridge
    participant GW as Gateway
    participant DB as PostgreSQL
    participant RA as Runtime Adapter
    participant NB as NanoBot
    participant MCP as Shopify MCP

    U->>MB: 发送文本消息
    GW->>MB: GET /api/messages（轮询）
    MB-->>GW: 返回消息数组（读取后清空缓冲区）
    GW->>GW: 适配器内部 POST /gateway/inbound + 校验 Bearer Token
    GW->>DB: 查询 channel_bindings 解析 bot_id
    GW->>GW: 标准化消息 / 生成 session_id
    GW->>DB: 查询 Bot 配置、Session 映射
    GW->>RA: 按 runtime_type 分发到 NanoBotAdapter
    RA->>NB: POST {runtime_endpoint}（session_id 隔离会话）
    NB->>MCP: 基于 MEMORY.md 选择并调用 MCP
    MCP-->>NB: 返回结果
    NB-->>RA: 返回文本响应
    RA-->>GW: 标准回复对象（超长则截断至 4096 字符）
    GW->>DB: 写入 message_events / runtime_logs
    GW->>MB: POST /api/message（回写 wire）
    MB-->>U: Telegram 收到回复
    Note over GW,MB: SSoT 内部契约保留为 `POST /bridge/reply`（当前实现直接对接 Matterbridge `/api/message`）
```

### 关键决策点

| 步骤 | 决策 | 失败处理 |
|------|------|---------|
| 3 | Bearer Token 校验 | 无效 → 401 |
| 4 | `channel_bindings` 查找 | 未找到 → 404，拒绝处理 |
| 4 | 幂等去重检查 | 重复 → 200 + `ignored_duplicate` |
| 5 | 限流检查 (5 msg/sec/chat_id) | 超限 → 429 |
| 7 | Runtime 超时 (Gateway hard timeout: 15s) | 超时 → 错误回写（NanoBot 服务端自身超时为 120s） |
| 12 | 回写能力 | 回写失败 → 指数退避重试（1s/2s/4s）后标记 reply_failed |

---

## 📊 场景 2: Runtime 异常

### 序列图

```mermaid
sequenceDiagram
    autonumber
    participant MB as Matterbridge
    participant GW as Gateway
    participant RA as Runtime Adapter
    participant NB as NanoBot

    GW->>MB: GET /api/messages
    MB-->>GW: 返回消息数组
    GW->>RA: 调用 Runtime
    RA->>NB: 请求处理
    NB--xRA: Gateway hard timeout(15s) / 失败
    RA-->>GW: 统一错误对象（RUNTIME_TIMEOUT / RUNTIME_UNAVAILABLE）
    Note over GW,MB: 错误提示通过 `POST /api/message` 回写到 Telegram
```

### 关键决策点

| 步骤 | 决策 | 失败处理 |
|------|------|---------|
| 4 | Gateway 侧 NanoBot 超时 (15s hard timeout，NanoBot 内部为 120s) | 统一返回“抱歉，当前无法处理您的请求，请稍后再试。” |
| 4 | Runtime 不可达 | `RUNTIME_UNAVAILABLE` 错误码 |
| 4 | Runtime 响应格式异常 | `RUNTIME_BAD_RESPONSE` 错误码 |

---

## 场景 3: MCP 调用失败

### 序列图

```mermaid
sequenceDiagram
    autonumber
    participant GW as Gateway
    participant RA as Runtime Adapter
    participant NB as NanoBot
    participant MCP as Shopify MCP

    GW->>RA: 调用 Runtime
    RA->>NB: 请求处理
    NB->>MCP: 调用目标 MCP
    MCP--xNB: 不可达(10s超时) / 报错
    NB-->>RA: 工具失败文本或结构化错误
    RA-->>GW: 标准错误回复（MCP_TIMEOUT / MCP_UNAVAILABLE）
```

---

## 📊 场景 4: 回写失败

### 序列图

```mermaid
sequenceDiagram
    autonumber
    participant GW as Gateway
    participant MB as Matterbridge

    GW->>MB: POST /api/message（第 1 次）
    MB--xGW: 服务不可达 / 网络失败 / 5xx / 429
    GW->>GW: 等待 1s（指数退避）
    GW->>MB: POST /api/message（第 1 次重试）
    MB--xGW: 失败
    GW->>GW: 等待 2s
    GW->>MB: POST /api/message（第 2 次重试）
    MB--xGW: 失败
    GW->>GW: 等待 4s
    GW->>MB: POST /api/message（第 3 次重试）
    MB--xGW: 失败
    GW->>GW: 标记 reply_failed + 记录错误日志
```

### 重试策略

| 参数 | 值 |
|------|-----|
| 最大重试次数 | 3 次（重试；总尝试最多 4 次） |
| 退避策略 | 指数退避（1s / 2s / 4s） |
| 终态标记 | `reply_failed` |
| 幂等保证 | `reply_id`（SSoT 内部契约；Matterbridge `/api/message` 不识别该字段） |
| 409 语义 | 视为"回写已完成"的成功状态（契约；当前 wire 端点通常不返回 409） |

---

## 📊 场景 5: PostgreSQL 不可用

```mermaid
sequenceDiagram
    autonumber
    participant MB as Matterbridge
    participant GW as Gateway
    participant DB as PostgreSQL

    MB->>GW: POST /gateway/inbound
    GW->>DB: 尝试操作
    DB--xGW: 不可用
    GW->>GW: 短路熄断（Circuit Break）
    GW->>MB: 503 Service Unavailable 或 "系统暂时不可用，请稍后重试"
    GW->>GW: 记录系统级告警 + 上报 db_unavailable_total
```

> **设计原则**: 宁可报错不可错乱 — 绝对禁止在无 DB 时向 Runtime 传递上下文或推进业务状态。

---

## ⏱️ 超时预算链路

| 阶段 | 超时上限 | 说明 |
|------|---------|------|
| Gateway poller 拉取 + Token 校验 + bot_id 解析 | ~200ms | 本地/内网通信 |
| Gateway → Runtime HTTP 调用 | ≤ 15s | hard timeout，含 Runtime 内部 MCP 调用 |
| Runtime → MCP 工具调用 | ≤ 10s | 嵌套在 Runtime 超时内 |
| Gateway 回写到 Bridge | ~500ms | 本地/内网通信 |
| **端到端 P95 目标** | **≤ 5s** | 正常场景的响应时间目标 |

### 约束关系

- 5s 为正常 P95 目标，15s 为极端 hard timeout
- 超过 5s 应上报慢响应指标
- MCP 10s 超时嵌套在 Runtime 15s 超时内
- 超时后 Gateway 不取消下游请求（MVP 简化）

---

## 🔄 状态机: 消息处理状态

```mermaid
stateDiagram-v2
    [*] --> received: 消息入站

    received --> processing: 标准化完成
    received --> rejected: Token无效 / 绑定缺失 / 限流

    processing --> runtime_success: Runtime 正常返回
    processing --> runtime_error: Runtime 超时/失败

    runtime_success --> reply_success: 回写成功
    runtime_success --> reply_failed: 回写失败（3次重试后）

    runtime_error --> reply_success: 错误提示回写成功
    runtime_error --> reply_failed: 错误提示回写也失败

    rejected --> [*]
    reply_success --> [*]
    reply_failed --> [*]
```

### 状态机 → message_events 字段映射

| 架构态（状态机） | `message_events.status` 写入值 | `message_events.reply_status` 写入值 | 写入时点 |
|----------------|-------------------------------|--------------------------------------|---------|
| `received` | `pending` | —（未写入） | 消息通过 Bearer Token 校验、幂等检查后落库 |
| `processing` | `processing` | —（未写入） | 开始调用 Runtime Adapter 时更新 |
| `runtime_success` | `done` | —（未写入） | Runtime 正常返回后更新 |
| `runtime_error` | `error` | —（未写入） | Runtime 超时/失败后更新 |
| `reply_success` | —（保持 `done`/`error`） | `success` | 回写 Bridge 成功后更新 |
| `reply_failed` | —（保持 `done`/`error`） | `reply_failed` | 回写能力落地后，3次重试均失败时更新 |
| `rejected` | 不写入 `message_events` | —（不写入） | 前置拒绝（限流/Token无效/绑定缺失）不产生事件记录 |

---

## AI 引用指南

当 AI 实现运行时场景时：
1. 主链路必须覆盖完整的 5 个场景
2. 超时预算必须严格遵循（P95 ≤ 5s, hard 15s, MCP 10s）
3. DB 不可用必须短路熄断，不得降级处理
4. 回写失败必须实现 3 次指数退避重试
5. 409 必须视为成功语义
