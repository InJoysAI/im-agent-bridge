# IM-Agent-Bridge

[English](README.md) | [官网](https://cbec.injoys.ai/) | [Issues](../../issues)

面向跨境电商场景的轻量、可自托管 IM + AI Agent 桥接骨架。核心解决"把任意 Agent Runtime（NanoBot、Accio-like、自建等）快速接入 Telegram，并调用真实 Shopify MCP 工具"的问题。

项目采用三层架构（Channel Layer → Bridge Layer → Core Layer），当前以 Telegram 作为 MVP 首个渠道，目标是打通"消息接入 → Runtime 处理 → 工具调用 → 回写"的可替换链路。

## 项目定位

- Channel Layer: Telegram（消息入口/出口）
- Bridge Layer: Matterbridge（仅桥接，不承载业务）
- Core Layer: Gateway（Rust）+ Runtime（NanoBot）+ PostgreSQL
- 工具调用: Runtime 内通过 Shopify MCP 执行

## 当前实现范围（MVP）

- 仅支持文本消息
- Gateway 提供 `POST /gateway/inbound`、`GET /health`、`GET /metrics`
- Gateway 内建 Matterbridge poller（`GET /api/messages`）与回写（`POST /api/message`）
- PostgreSQL 持久化：`bots`、`channel_bindings`、`sessions`、`message_events`、`runtime_logs`
- Runtime 默认 NanoBot，可通过 Runtime Adapter 替换

## 仓库结构

```text
im-agent-bridge/
├── .context/                # AI 上下文资产（权威摘要与约束）
├── SSoT/
│   ├── schema/migrations/   # Goose SQL 迁移
│   └── api/                 # TypeSpec API 契约
├── gateway/                 # Rust Gateway 实现
├── deploy/                  # Edge/Internal/Postgres 部署配置
├── openspec/                # 需求提案与规格管理
└── design/                  # Context-Dev 工具链
```

## 权威文档入口（先读）

- `.context/criterion.md`：项目强约束（MUST / MUST NOT）
- `.context/context-manifest.json`：上下文资产与同步状态
- `.context/architecture/README.md`：架构索引
- `.context/domain/README.md`：业务规则索引
- `.context/db/README.md`：数据库索引

如果 README 与 `.context/source` 或 SSoT 冲突，以以下内容为准：

1. `source/` 源文档（PRD/TAD）
2. `SSoT/`（Goose/TypeSpec）
3. `.context/` 汇总文件

## 快速启动（本地开发）

### 1. 启动 PostgreSQL

```bash
cp deploy/postgres/.env.example deploy/postgres/.env
docker compose -f deploy/postgres/docker-compose.yml up -d --build postgres
```

### 2. 执行数据库迁移（Goose）

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://<user>:<password>@127.0.0.1:<port>/<db>?sslmode=disable'
make db-migrate-up
```

### 3. 启动 NanoBot Runtime（可选但建议）

```bash
cd deploy/internal-server/nanobot
cp .env.example .env
cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md
docker compose up -d
```

### 4. 启动 Gateway

```bash
cd gateway
cp .env.example .env
# 编辑 .env: GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run
```

### 5. 启动 Matterbridge（Edge）

```bash
cd deploy/edge-server
# 准备 .env 与 matterbridge.toml 后启动
docker compose up -d
```

## 开发约束（必须遵守）

- 需求变更优先走 OpenSpec：先提案，再实现（`openspec/`）
- 数据库变更先改 `SSoT/schema/migrations/`
- API 变更先改 `SSoT/api/main.tsp`
- 禁止跨层调用：Bridge 不直接调 Runtime，Runtime 不直接连 Telegram
- 禁止在数据库存储 MCP 凭证/实例配置

## 常用命令

```bash
# TypeSpec 编译
make api-compile

# 生成 Rust OpenAPI 类型
make api-gen-rs

# Goose 迁移状态
make db-migrate-status

# Gateway 测试
cd gateway && cargo test
```

## 相关文档

- NanoBot 部署说明：`deploy/internal-server/nanobot/README.md`
- Postgres 保留策略：`deploy/postgres/README.md`

---

## 商业版（CBECOps Pro）

开源骨架覆盖核心桥接能力。生产级功能（富媒体、多店铺路由、SSO、审计日志、托管服务）请查看 **[CBECOps Pro](https://cbec.injoys.ai/)**。

| 版本 | 月费 | 主要功能 |
|------|------|---------|
| Starter | $29/店铺 | 基础骨架 + 监控 + 邮件支持 |
| Pro | $79/店铺 | 富媒体 + 多店铺 + 优先支持 |
| Enterprise | $199+/店铺 | 多 IM 渠道 + SSO + 审计日志 + 定制开发 |

## 参与贡献

欢迎提交 Bug 报告、文档改进、Runtime 适配器和 MCP 模板。详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

安全漏洞请**不要**公开 Issue，参见 [SECURITY.md](SECURITY.md) 通过邮件私下报告。

## License

Apache 2.0 — 详见 [LICENSE](LICENSE)。

Copyright 2026 InJoys AI
