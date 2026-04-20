# 安全策略 (Security Policy)

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-13 13:52`
> - **Generator**: `Context-Agent v1.0`

---

## 🔐 认证方案 (Authentication)

### 服务间认证

| 通信对 | 方案 | 实现 |
|--------|------|------|
| **Bridge ↔ Gateway** | Bearer Token | `Authorization: Bearer <token>`，环境变量注入 |
| **Gateway ↔ Runtime** | 无认证 (MVP) | 内网运行，后续可加 Token 或 mTLS |

> MVP 阶段不涉及用户认证（终端用户通过 Telegram 接入，身份由 Telegram 平台管理）。

### Token 管理

| Token 类型 | 存储位置 | 生命周期 |
|-----------|---------|---------|
| Bridge Bearer Token | 环境变量 | 长期有效，手动轮换 |

---

## 🛡️ 授权模型 (Authorization)

### 权限边界

| 组件 | 可访问目标 | 禁止访问 |
|------|-----------|---------|
| **Bridge (Matterbridge)** | Gateway API | Runtime, PostgreSQL, Telegram (直连) |
| **Gateway** | PostgreSQL, Runtime Adapter, Bridge API | 模型推理, MCP 选择 |
| **Runtime (NanoBot)** | Shopify MCP | Bridge, Telegram, PostgreSQL |
| **Shopify MCP** | Shopify API | 仅由 Runtime 调用 |

> MVP 阶段无 RBAC/ABAC 权限体系，无独立管理后台。

---

## 🔒 数据加密

### 传输层加密

| 通信对 | 要求 | 说明 |
|--------|------|------|
| **Bridge ↔ Gateway** | HTTP + Bearer Token（私有网络，MVP） | 跨服务器私有网络（VPN/云 VPC）；禁止公网暴露；生产环境应升级 HTTPS（TD-007） |
| **Gateway ↔ Runtime** | HTTP (内网同服务器) | 建议后续加 Token/mTLS（TD-001） |
| **Gateway ↔ PostgreSQL** | TCP (内网同服务器) | 建议启用 SSL |

### 存储层加密

| 数据类型 | 处理方式 |
|---------|---------|
| **Telegram Token** | 环境变量 / Secret Manager，禁止代码仓库 |
| **Bridge Bearer Token** | 环境变量，禁止明文入库 |
| **PostgreSQL 密码** | 环境变量 |
| **Shopify MCP 凭证** (client_id / client_secret / domain) | `.env` 或 Secret 注入，跟随 Runtime/MCP 运行环境管理 |

---

## 🚨 敏感数据处理

### 数据分类

| 分类 | 示例 | 处理要求 |
|------|------|---------|
| **高敏感** | Bearer Token (GATEWAY/BRIDGE)、Telegram Bot Token、Shopify client_secret、PG 密码/DATABASE_URL | 环境变量/Secret，禁止日志（脱敏 filter 集中屏蔽，替换为 `[REDACTED]`），禁止代码仓库 |
| **中敏感** | user_id, chat_id, input_text | 脱敏存储（input_text 截断至 512 字符），runtime_logs 仅错误时写入且脱敏 PII |
| **低敏感** | bot_name, platform, event_id | 常规保护 |

### 数据最小化策略 (TAD §10.5)

| 字段 | 策略 |
|------|------|
| `message_events.input_text` / `output_text` | 短期排障用途，截断至 512 字符 |
| `runtime_logs.request_payload` / `response_payload` | 仅 `status = 'error'` 时写入，脱敏 PII |

### 数据保留期

| 数据表 | 保留期 | 清理方式 |
|--------|--------|---------|
| `message_events` | 30 天 | 定时任务 / PG 分区 |
| `runtime_logs` | 14 天 | 定时任务 |
| `sessions` | 无自动过期 | 可按 `updated_at` 手动清理 |

### 访问控制

- `input_text` / `output_text` / `request_payload` / `response_payload` 仅允许系统开发者/运维级别访问
- 日志查询应记录审计日志

---

## 🛡️ 安全防护

### API 安全

| 防护措施 | 实现 |
|---------|------|
| **Bearer Token 校验** | Gateway 入站必须校验 `Authorization: Bearer <token>` |
| **Matterbridge API Token** | Matterbridge API 模式对外暴露 REST 接口，通过 `Token` + `Authorization: Bearer <token>` 头保护（外部参考，TAD 未定义端口/鉴权形态，以实际 Matterbridge 文档为准） |
| **Runtime API 接口** | Gateway → Runtime 使用裸 HTTP（MVP 内网运行），仅暴露 localhost 或受控内网 |
| **来源白名单** | Bridge API / NanoBot API 仅暴露 localhost 或受控内网 |
| **限流** | Token Bucket，5 msg/sec/chat_id |
| **幂等** | 入站: `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id)`（协议字段 `raw_message.message_id` 落库为 `bridge_message_id`）；回写: `reply_id` |
| **公网隔离** | Bridge API（:4242）仅私有网络可达；Gateway（:8080）仅 Internal Server 内网；NanoBot API 禁止公网暴露 |

### 防注入策略 (TAD §10.4)

| 措施 | 说明 |
|------|------|
| Bearer Token 校验 | Bridge API 必须校验 |
| 来源校验 | Gateway 必须校验消息来源合法性 |
| 字段过滤 | 仅允许受控字段进入 Runtime |
| 异常记录 | 记录异常来源与拒绝日志 |

---

## 📋 合规性要求

N/A – 源文档未提供 GDPR / PCI-DSS 等合规性要求。但遵循数据最小化原则（§10.5），input_text 截断存储、runtime_logs 脱敏处理。

---

## AI 引用指南

当 AI 生成安全相关代码时：
1. Bridge ↔ Gateway 通信必须使用 Bearer Token（私有网络 HTTP + Bearer Token，MVP；生产应升级 HTTPS）
2. 所有敏感凭证必须通过环境变量注入，禁止硬编码
3. PostgreSQL 不存 MCP 凭证 / 密钥引用
4. input_text / output_text 必须截断至 512 字符
5. runtime_logs payload 仅错误时写入且脱敏
6. 日志禁止记录 Bearer Token / Shopify client_secret
