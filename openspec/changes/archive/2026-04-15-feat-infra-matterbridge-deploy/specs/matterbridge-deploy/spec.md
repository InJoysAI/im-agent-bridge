## ADDED Requirements

### Requirement: Matterbridge Docker 容器编排
系统必须（MUST）在 Edge Server 上通过 Docker Compose 编排 Matterbridge 容器，挂载配置文件，通过 `.env` 注入凭证，并配置自动重启与健康检查。

#### Scenario: Matterbridge 容器正常启动
- **WHEN** Edge Server 执行 `docker compose up -d`（已配置 `.env` 文件中的 `TELEGRAM_BOT_TOKEN`、`GATEWAY_URL`、`GATEWAY_BEARER_TOKEN`）
- **THEN** `matterbridge` 容器状态变为 healthy
- **AND** `docker compose ps` 显示容器 running，health = healthy

#### Scenario: Matterbridge 容器异常退出后自动恢复
- **WHEN** Matterbridge 容器进程意外终止
- **THEN** Docker 自动重启容器（`restart: unless-stopped`）
- **AND** 容器恢复正常运行，不需要人工干预

---

### Requirement: matterbridge.toml 配置模板
系统必须（MUST）提供 `deploy/edge-server/matterbridge/matterbridge.toml` 配置模板，声明 Telegram polling 账户和 API 网关，并将 Telegram Bot Token 通过环境变量注入而非硬编码。

#### Scenario: Telegram polling 账户配置正确
- **WHEN** `matterbridge.toml` 中 `[telegram.mytelegram]` Token 字段使用环境变量引用（非硬编码值）
- **THEN** Matterbridge 启动时通过容器 `.env` 正确加载 `TELEGRAM_BOT_TOKEN`
- **AND** Matterbridge 成功与 Telegram Bot API 建立 polling 连接

#### Scenario: API 网关监听端口配置正确
- **WHEN** `matterbridge.toml` 中 `[api.myapi]` `BindAddress` 设置为 `0.0.0.0:4242`
- **THEN** Matterbridge API 在容器内 `:4242` 端口监听
- **AND** 该端口仅对 Internal Server 私有网络可达，不暴露到公网

---

### Requirement: Gateway 入站消息适配器（轮询 Matterbridge 并推送 /gateway/inbound）
系统必须（MUST）由 Gateway 内建 `adapters::matterbridge` 模块轮询 Matterbridge `GET /api/stream`（SSE 长连接），将其消息构造为符合 SSoT 模型的 `InboundRequest` 后内部调用 `POST /gateway/inbound`。

> **架构说明**：原方案为独立 `mb-adapter` Python 进程，实施中合并入 Gateway Rust 代码（`gateway/src/adapters/matterbridge.rs`），以 `tokio::spawn` 后台任务运行，消除独立容器。

#### Scenario: Telegram 文本消息经 Gateway 适配器推送 /gateway/inbound
- **WHEN** Matterbridge 收到 Telegram 文本消息，Gateway `adapters::matterbridge` 后台任务正在轮询 `GET {BRIDGE_URL}/api/stream`
- **THEN** Gateway 适配器内部调用 `POST http://localhost:8080/gateway/inbound`，携带 `Authorization: Bearer <GATEWAY_BEARER_TOKEN>`
- **AND** 请求体符合 `InboundRequest` schema：`platform`、`bridge_gateway_name`、`raw_message`（含 `chat_id`、`chat_type`、`user_id`、`message_type`、`text`、`timestamp`、`message_id`）
- **AND** 对于文本消息 `message_type = "text"`，`text` 字段非空

#### Scenario: 私有网络跨服务器通信验证
- **WHEN** Gateway 运行在 Internal Server，Matterbridge 运行在 Edge Server 私有网络（`BRIDGE_URL=http://<edge-private-ip>:4242`）
- **THEN** Gateway 能通过 `BRIDGE_URL/api/stream` 成功连接并接收消息流
- **AND** `/gateway/inbound` 端点返回 HTTP 200

---

### Requirement: Gateway 回写 Matterbridge 并转发到 Telegram（⏳ 待下一提案实现）
系统必须（MUST）由 Gateway 暴露 `POST /bridge/reply` 端点，接收回写请求后调用 Matterbridge `POST /api/message` 将消息转发到 Telegram。

> **状态**：本提案仅完成入站方向（Telegram → Gateway）。`POST /bridge/reply` 端点的实现推迟到下一提案（`feat-logic-gateway-reply` 或同类变更）。

#### Scenario: Gateway 回写经 Matterbridge 转发到 Telegram
- **WHEN** Gateway 向自身 `POST /bridge/reply` 发送 `ReplyRequest`（含 `reply_id`、`chat_id`、`platform`、`text`、`bridge_gateway_name`）
- **THEN** Gateway 将 `text` 和 `bridge_gateway_name` 构造成 Matterbridge 消息体，`POST {BRIDGE_URL}/api/message`
- **AND** Matterbridge 将回复消息转发到对应的 Telegram chat
- **AND** `POST /bridge/reply` 返回 HTTP 200

---

### Requirement: 凭证环境变量注入与安全隔离
系统必须（MUST）提供 `deploy/edge-server/.env.example`，声明所有必要的环境变量占位符，且配置文件中禁止硬编码任何凭证值。

#### Scenario: .env.example 包含必要变量
- **WHEN** 查看 `deploy/edge-server/.env.example`
- **THEN** 文件包含 `TELEGRAM_BOT_TOKEN`、`TELEGRAM_CHAT_ID`、`GATEWAY_URL`、`GATEWAY_BEARER_TOKEN`、`EDGE_PRIVATE_IP` 五个变量（值为占位符，非真实凭证）
- **AND** `matterbridge.toml` 中无任何明文 Token 或 Bearer Token 值

#### Scenario: Matterbridge API 端口仅对私网可达
- **WHEN** Edge Server 启动并运行，`EDGE_PRIVATE_IP` 已配置为 Edge Server 的内网 IP
- **THEN** Matterbridge API 端口 `:4242` 在宿主机仅绑定 `${EDGE_PRIVATE_IP}:4242`（私网 IP，非 0.0.0.0/localhost）
- **AND** Internal Server 可通过私有网络访问 `${EDGE_PRIVATE_IP}:4242`
- **AND** 公网无法直接访问该端口
