# Change: Matterbridge 部署与配置

## Why

IM Agent Bridge 需要一个 Bridge Layer 将 Telegram 消息桥接到 Gateway。Matterbridge 作为 Bridge Layer 的唯一实现，负责：

1. **入站**：接收 Telegram 消息（polling 模式）并通过 `POST /gateway/inbound` 主动推送到 Gateway（私有网络 HTTP + Bearer Token）
2. **出站**：接收 Gateway 的回写调用（`POST /bridge/reply`），将回复消息转发回 Telegram

没有 Matterbridge，整个消息链路无法建立。然而 Matterbridge 原生提供 Pull API（`GET /api/stream`），而 TAD 定义的是 Push 模型（Matterbridge → `POST /gateway/inbound`），存在适配差距（RISK-007）。

本提案交付：(1) Edge Server 上 Matterbridge 的完整部署配置；(2) 在 Gateway 内实现 Matterbridge 适配器模块（`gateway/src/adapters/matterbridge.rs`），解决 RISK-007——将 Matterbridge Pull API 桥接为 Gateway 内部 `POST /gateway/inbound` 调用。

> **架构决策（实施中确认）**：原方案设计独立 `mb-adapter` Python 进程，实施后发现该进程与 Matterbridge 须同机部署但与 Gateway 业务逻辑属于同一关注点，引入额外进程和网络跳跃无实质收益。最终决定将适配逻辑内化为 Gateway 的 `adapters::matterbridge` 模块，以 `tokio::spawn` 后台任务运行，消除 `mb-adapter` 独立容器。

## What Changes

### 新增功能

- `deploy/edge-server/docker-compose.yml`：Edge Server 服务编排（仅 matterbridge，mb-adapter 已废弃）
- `deploy/edge-server/matterbridge/matterbridge.toml`：Matterbridge 配置模板（Telegram polling + API 网关，API 监听 `0.0.0.0:4242`）
- `deploy/edge-server/.env.example`：Edge Server 环境变量示例（`TELEGRAM_BOT_TOKEN`、`TELEGRAM_CHAT_ID`、`GATEWAY_URL`、`GATEWAY_BEARER_TOKEN`、`EDGE_PRIVATE_IP`）
- `gateway/src/adapters/matterbridge.rs`：Gateway 内建 Matterbridge 适配器模块，实现：
  - 入站：`tokio::spawn` 后台任务轮询 `GET /api/stream`（SSE 长连接）→ 构造 `InboundRequest`（含 `message_type`）→ 内部调用 `POST /gateway/inbound`
  - 断线自动重连（3s/5s 退避）
  - 过滤 `api.` 协议消息防止回环

### 技术实现

- Matterbridge：`42wim/matterbridge:1.26.0`（版本 tag 锁定，见 RISK-005；禁止生产使用 `latest`）；`matterbridge.toml` Volume 挂载；凭证 `.env` 注入；代理环境变量（`HTTP_PROXY`/`HTTPS_PROXY`）通过 `environment` 块注入容器
- Matterbridge API 端口 `:4242`：宿主机端口映射绑定 `${EDGE_PRIVATE_IP}:4242:4242`（私网 IP，不绑 0.0.0.0/localhost），Internal Server 通过私有网络访问，禁止公网暴露；Matterbridge API 不启用 Token 鉴权（网络隔离作为安全边界）
- Gateway 适配器（`adapters::matterbridge`）：Rust 异步模块，`tokio::spawn` 后台任务；连接 `BRIDGE_URL/api/stream`（SSE 长连接，`no_proxy()` 绕过系统代理）；消息映射逻辑与原 mb-adapter 方案一致；出站回写（`POST /bridge/reply` → `POST {BRIDGE_URL}/api/message`）待下一提案实现
- Bridge ↔ Gateway：私有网络 HTTP + Bearer Token（MVP）；生产升级 HTTPS（TD-007，阶段性例外，见下方风险说明)

## Impact

### Out-of-scope

- 多 Bot 多渠道配置（MVP 单 Bot，单 Telegram 渠道）
- Internal Server 服务编排（Gateway / NanoBot / PostgreSQL，属于其他提案范围）
- TLS 升级（阶段性例外 TD-007，后续变更 `feat-infra-tls-edge-gateway`）
- Telegram Webhook 模式（MVP 仅 polling）
- mb-adapter 精细错误重试、断路器、追踪模块（这些属于 Gateway 侧责任，不属于当前提案）

### 涉及的规范（Specs）

- **新增**：`specs/matterbridge-deploy/spec.md` — Matterbridge 部署与配置 capability delta

### 涉及的代码

- **新增**：
  - `deploy/edge-server/docker-compose.yml`
  - `deploy/edge-server/matterbridge/matterbridge.toml`
  - `deploy/edge-server/.env.example`
  - `gateway/src/adapters/matterbridge.rs`
  - `gateway/src/handlers/inbound.rs`
- **修改**：
  - `gateway/src/adapters/mod.rs`（导出 matterbridge 模块）
  - `gateway/src/handlers/mod.rs`（导出 inbound 模块）
  - `gateway/src/config.rs`（移除 `bridge_bearer_token`，保留 `bridge_url`）
  - `gateway/src/main.rs`（注册 `/gateway/inbound` 路由，spawn poller 任务）
  - `gateway/Cargo.toml`（新增 `futures-util`、`anyhow`，reqwest 开启 `stream` feature）
  - `gateway/.env.example`（`BRIDGE_URL` 指向 Edge Server 私网地址）
- **废弃**：`deploy/edge-server/mb-adapter/`（目录已删除）

### 依赖关系

- **依赖**：`feat-infra-gateway-scaffold`（已完成，archived）
- **被依赖**：`feat-e2e-integration-test`

### 风险与注意事项

- **RISK-005**（Matterbridge 桥接稳定性）：`restart: unless-stopped` + 健康检查；镜像 tag 锁定（`42wim/matterbridge:1.26.0`，禁止生产使用 `latest`）；Gateway 适配器断线后自动重连（3s/5s 退避），Matterbridge 容器继续运行不受影响
- **RISK-007**（Push/Pull 适配差距）：已通过 Gateway 内建 `adapters::matterbridge` 模块解决，不再遗留到下游变更
- **TD-007**（阶段性 HTTPS 例外）：`config.yaml:185` 要求 Bridge ↔ Gateway MUST 使用 HTTPS，MVP 阶段以私有网络 HTTP 作阶段性例外；升级触发条件：Edge Server 与 Internal Server 不在同一 VPC/内网时，或生产上线前安全评审要求时；后续变更 ID：`feat-infra-tls-edge-gateway`（待立项）

### 验证标准

- ✅ Matterbridge 容器 health = healthy
- ✅ Telegram 发消息 → Matterbridge 日志收到入站消息
- ✅ Gateway 内建 poller → `POST /gateway/inbound`：请求体符合 `InboundRequest` schema（含 `message_type`、Bearer Token），返回 `200 OK`
- ⏳ Gateway `POST /bridge/reply` → Matterbridge `POST /api/message` → Telegram（回写链路，待下一提案实现）
- ✅ Matterbridge API 端口 `:4242` 仅绑私网 IP，公网不可达

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.3 Bridge Layer 约束（MUST/MUST NOT）；§4 安全约束（凭证环境变量注入，禁止公网暴露） |
| architecture | `.context/architecture/deployment_view.md` | Matterbridge 配置格式、部署目录结构、API 端点规范、容器划分 |
| architecture | `.context/architecture/api_strategy.md` | §1 `POST /gateway/inbound`（Matterbridge 调用方）；§2 `POST /bridge/reply`（回写目标） |

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | `openspec/proposal-roadmap.md` |
| roadmap_source_supplement | N/A |
| phase | Phase 0 |
| business_goal | 配置 Matterbridge 将 Telegram 消息转发到 Gateway `POST /gateway/inbound`，并接收 Gateway 回写调用（`POST /bridge/reply`）；凭证通过环境变量注入 |
| dependencies | 前置：`feat-infra-gateway-scaffold`（已完成）；被依赖：`feat-e2e-integration-test` |
| acceptance_criteria | 容器 healthy；Telegram 消息→日志可见；入站推送格式正确（含 Bearer Token）；回写链路可达；私有网络跨服务器通信验证成功 |
