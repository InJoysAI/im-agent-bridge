# 实施任务清单

> feat-infra-matterbridge-deploy — Matterbridge 部署与配置（Phase 0，预计 2 天）
> 前置依赖：`feat-infra-gateway-scaffold`（已 archived）
> SSoT 先行检查：本提案不涉及 DB Schema 或 API 合约变更，无需新增 Goose 迁移或修改 `SSoT/api/main.tsp`（SSoT 未更改）。

## 1. SSoT 先行检查（不涉及变更，确认即可）

- [x] 1.1 确认本提案不引入新 DB 表或字段变更（无需 Goose 迁移）
- [x] 1.2 确认本提案不修改 `SSoT/api/main.tsp` 中的 API 契约（`POST /gateway/inbound` 和 `POST /bridge/reply` 端点已在 feat-infra-gateway-scaffold 中定义）
- [x] 1.3 在提案内注明"SSoT 未更改"

## 2. 目录结构与环境变量（0.25 天）

- [x] 2.1 创建 `deploy/edge-server/` 目录结构
  - `deploy/edge-server/matterbridge/`（存放 `matterbridge.toml`）
- [x] 2.2 编写 `deploy/edge-server/.env.example`
  - 包含 `TELEGRAM_BOT_TOKEN=<your-telegram-bot-token>`
  - 包含 `GATEWAY_URL=http://<internal-server-ip>:8080`
  - 包含 `GATEWAY_BEARER_TOKEN=<your-gateway-bearer-token>`
  - 确认无任何真实凭证值
- [x] 2.3 在 `.gitignore` 中确认 `deploy/edge-server/.env` 已被排除（非 `.env.example`）

## 3. matterbridge.toml 配置模板（0.5 天）

- [x] 3.1 编写 `deploy/edge-server/matterbridge/matterbridge.toml`
  - `[telegram.mytelegram]` 节：`Token` 使用环境变量（`${TELEGRAM_BOT_TOKEN}`）；`RemoteNickFormat="{NICK}"`
  - `[api.myapi]` 节：`BindAddress="0.0.0.0:4242"`；`Buffer=1000`；`RemoteNickFormat="{NICK}"`；`Token` 字段留空（或使用占位符，Matterbridge API Token 与 Gateway Bearer Token 分开管理）
  - `[[gateway]]` 节：`name="gateway1"`；`enable=true`
  - `[[gateway.inout]]` 账户 `telegram.mytelegram`，channel 配置（参照 deployment_view.md）
  - `[[gateway.inout]]` 账户 `api.myapi`，channel=`"api"`
- [x] 3.2 验证 `matterbridge.toml` 格式合法（TOML 语法检查）
- [x] 3.3 在注释中标注 Gateway inbound URL 示例：`http://<internal-server-ip>:8080/gateway/inbound`

## 4. Docker Compose 编排（0.25 天）

- [x] 4.1 编写 `deploy/edge-server/docker-compose.yml`
  - 服务名：`matterbridge`；镜像：`42wim/matterbridge:1.26.0`（禁止 `latest`）
  - Volume 挂载：`./matterbridge/matterbridge.toml:/etc/matterbridge/matterbridge.toml:ro`
  - 环境变量：从 `.env` 文件加载（`env_file: .env`）；代理变量通过 `environment` 块注入
  - 重启策略：`restart: unless-stopped`
  - 健康检查：检测 Matterbridge API 端口可达（`healthcheck`）
  - 端口映射：绑定 Edge Server 内网 IP（`${EDGE_PRIVATE_IP}:4242:4242`，不绑 0.0.0.0/localhost）；Internal Server 通过私有网络访问，公网无法达到
  - `mb-adapter` 服务已废弃，适配逻辑合并入 Gateway
- [x] 4.2 确认 docker-compose.yml 不包含任何硬编码凭证

## 5. Gateway Matterbridge 适配器实现（原 mb-adapter，架构调整后合并入 Gateway）

- [x] 5.1 实现 `gateway/src/adapters/matterbridge.rs`
  - `tokio::spawn` 后台任务，`loop` 持续重连
  - `GET {BRIDGE_URL}/api/stream` SSE 长连接，`no_proxy()` 绕过系统代理
  - 将 Matterbridge 消息映射为 `InboundRequest`：
    - `platform = "telegram"`（固定）
    - `bridge_gateway_name` 来自 message.gateway（默认 `"gateway1"`）
    - `raw_message.chat_id` 来自 message.channel
    - `raw_message.chat_type`：默认 `"group"`
    - `raw_message.user_id` 来自 message.userid（降级到 message.username）
    - `raw_message.message_type`：message.text 非空 → `"text"`，否则 `"other"`
    - `raw_message.text`：仅 `message_type=text` 时携带
  - 过滤 `protocol=="api"` 或 `account.starts_with("api.")` 消息防止回环
  - 断线后 3s（正常关闭）/ 5s（错误）退避重连
- [x] 5.2 实现 `gateway/src/handlers/inbound.rs`（`POST /gateway/inbound` 端点）
  - Bearer Token 鉴权
  - 仅接受 `message_type=="text"`，否则返回 400（BR-001）
  - 返回 `{"status": "accepted"}`
- [x] 5.3 注册路由并在 `main.rs` spawn poller 任务
- [ ] 5.4 实现出站端点 `POST /bridge/reply`（待下一提案）
  - 接收 `ReplyRequest`，`POST {BRIDGE_URL}/api/message` 到 Matterbridge
- [x] 5.5 不引入任何硬编码凭证（`BRIDGE_URL`、`GATEWAY_BEARER_TOKEN` 均通过环境变量读入）

## 6. 验证（0.5 天）

- [x] 6.1 在 Edge Server 执行 `docker compose up -d`，验证 `matterbridge` 容器启动正常（mb-adapter 已合并入 Gateway）
- [x] 6.2 在 Telegram 发送文本消息，验证 Matterbridge 日志收到入站消息
- [x] 6.3 验证 Gateway 内建 poller → `POST /gateway/inbound`（Bearer Token）请求格式符合 `InboundRequest` schema
  - 检查请求体包含：`platform`、`bridge_gateway_name`、`raw_message`（`chat_id`、`chat_type`、`user_id`、**`message_type`**、`text`、`timestamp`、`message_id`）
  - 验证文本消息 `message_type = "text"`，`text` 非空
- [ ] 6.4 验证 Gateway `POST /bridge/reply` → Matterbridge `POST /api/message` → Telegram（回写链路，待实现）
- [x] 6.5 验证 Matterbridge API 端口（:4242）仅对 `${EDGE_PRIVATE_IP}` 私网监听，公网无法访问

## 7. 验证与归档

- [x] 7.1 运行 `specflow validate feat-infra-matterbridge-deploy --strict`（完整命令：`node design/context-dev/tools/specflow/specflow.mjs validate feat-infra-matterbridge-deploy --strict`）
- [x] 7.2 运行 `specflow archive feat-infra-matterbridge-deploy --yes`（完整命令：`node design/context-dev/tools/specflow/specflow.mjs archive feat-infra-matterbridge-deploy --yes`）
