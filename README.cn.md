# IM-Agent-Bridge

> **为您的 Shopify 店铺配备一个 7×24 小时 AI 助理，在 Telegram 上自托管、隐私安全、Runtime 随时可换。**

[English](README.md) · [官网](https://cbec.injoys.ai/) · [Issues](../../issues) · [License: Apache 2.0](LICENSE)

![构建状态](https://img.shields.io/badge/build-passing-brightgreen) ![License](https://img.shields.io/badge/license-Apache%202.0-blue) ![阶段](https://img.shields.io/badge/stage-MVP%20v1.1-orange) ![自托管](https://img.shields.io/badge/hosting-self--hosted-purple)

---

## 为什么选择 IM-Agent-Bridge？

跨境 Shopify 卖家每天要处理大量重复性问题：*"我的订单到哪儿了？""这个 SKU 还有库存吗？""帮我写一封退款回复。"*

IM-Agent-Bridge 解决的是 **"最后一公里"问题**：把任意 AI Agent Runtime 接入 Telegram，并赋予它调用**真实 Shopify MCP 工具**的能力——无需依赖任何云平台，无需向第三方共享客户数据，无需被绑定在特定 AI 供应商。

- 🔌 **Runtime 可替换** — 默认搭载 NanoBot，通过一个 Adapter 即可换成任意 OpenAI-compatible Runtime
- 🔒 **隐私优先** — 所有流量在您自己的服务器上运行，客户 PII 数据不离境
- 🛒 **真实 Shopify MCP 调用** — 订单查询、库存检查、客户数据，均在 Runtime 内执行，非模拟
- ⚡ **轻量骨架** — Rust Gateway 处理路由与会话管理，资源占用极低

---

## 架构概览

```
Telegram ──► Matterbridge（Edge）──► Gateway（Rust）──► Runtime（NanoBot）
                                           │                    │
                                      PostgreSQL           Shopify MCP
```

| 层级 | 组件 | 职责 |
|------|------|------|
| **Channel 层** | Telegram + Matterbridge | 消息进出 — 仅边缘节点 |
| **Bridge 层** | Matterbridge poller | 纯中继 — 不承载任何业务逻辑 |
| **Core 层** | Gateway (Rust) + Runtime + PostgreSQL | 全部路由、会话、工具调度 |

**核心设计原则：**
- Bridge 不直接调用 Runtime → 两侧均可独立替换
- MCP 凭证仅存在于 Runtime `.env` 中 — 绝不写入数据库
- Gateway 是会话状态与 Runtime 调度的唯一权威

---

## 快速启动

### 环境依赖

| 工具 | 用途 |
|------|------|
| Docker & Docker Compose | 运行所有服务 |
| [Goose](https://pressly.github.io/goose/) | 数据库迁移 |
| Telegram Bot Token | 通过 [@BotFather](https://t.me/BotFather) 申请 |
| Shopify OAuth 凭证 | 每个店铺一组，在 Partners 后台申请 |
| LLM API Key | OpenAI-compatible（如 GPT-4o 等） |

> **提示：** 仅本地 Gateway 开发需要 Rust，Docker Compose 覆盖其他所有服务。

---

### 第一步 — 启动 PostgreSQL

```bash
cd deploy/postgres
cp .env.example .env
# 编辑 .env：设置 POSTGRES_USER、POSTGRES_PASSWORD、POSTGRES_DB
docker compose up -d
```

### 第二步 — 执行数据库迁移

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

### 第三步 — 配置 NanoBot Runtime

```bash
cd deploy/internal-server/nanobot
cp .env.example .env           # 填写 LLM_API_KEY + 各店铺 Shopify 凭证
cp config.json.example config.json  # 按店铺配置 MCP Server 条目
cp memory/MEMORY.md.example memory/MEMORY.md  # 自定义 Agent 人设
docker compose up -d
```

### 第四步 — 启动 Gateway

```bash
cd gateway
cp .env.example .env
# 必填：GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run
# 端点：POST /gateway/inbound  |  GET /health  |  GET /metrics
```

### 第五步 — 启动 Matterbridge（Edge）

```bash
cd deploy/edge-server
cp .env.example .env
# 必填：TELEGRAM_BOT_TOKEN、GATEWAY_URL、GATEWAY_BEARER_TOKEN
docker compose up -d
```

**健康检查：**
```bash
curl http://localhost:8080/health
# → {"status":"ok"}
```

---

## 真实使用场景示例

部署完成后，您的 Telegram 机器人可以处理以下自然语言请求，数据均来自真实 Shopify：

| Telegram 消息 | 底层执行逻辑 |
|--------------|------------|
| `订单 #12345 到哪儿了？` | Runtime 调用 Shopify MCP → 查询履单状态 → 返回物流跟踪信息 |
| `SKU WIDGET-BLK-XL 还有库存吗？` | Shopify MCP 库存查询 → 返回当前库存数量 |
| `汇总今天的未处理工单` | Agent 整合订单 + 客户数据生成摘要 |
| `帮我给订单 #98765 写一封退款回复` | Agent 生成符合品牌语气、PII 信息安全的回复文本 |

---

## 功能状态（MVP v1.1）

| 功能 | 状态 | 备注 |
|------|------|------|
| Telegram 文本消息 | ✅ 已完成 | 通过 Matterbridge edge |
| Gateway 入站路由 | ✅ 已完成 | `POST /gateway/inbound` |
| 会话管理（PostgreSQL） | ✅ 已完成 | 按聊天隔离 |
| NanoBot Runtime 适配器 | ✅ 已完成 | 默认 Runtime |
| Shopify MCP 工具调用 | ✅ 已完成 | 订单、库存、客户 |
| 健康检查端点 | ✅ 已完成 | `GET /health` |
| Prometheus 指标 | ✅ 已完成 | `GET /metrics` |
| 群聊 @提及过滤 | 🔄 规划中 | 仅响应 @mention |
| 富媒体支持（图片、文件） | 🔄 规划中 | CBECOps Pro 路线图 |
| 多店铺路由 | 🔄 规划中 | CBECOps Pro 路线图 |
| WhatsApp / LINE 渠道 | 🔄 规划中 | CBECOps Pro 路线图 |
| SSO / 团队权限 | 🔄 规划中 | CBECOps Pro 路线图 |
| 托管服务选项 | 🔄 规划中 | CBECOps Pro 路线图 |

---

## 已知限制

我们对当前 MVP 的边界保持诚实：

- **仅支持文本** — 图片、语音、文件、贴纸均不处理
- **仅支持 Telegram** — WhatsApp、LINE、微信等其他 IM 渠道暂未支持
- **群聊上下文共享** — 群内所有成员共享同一个 Agent 会话（无单用户隔离）
- **暂无 @提及过滤** — 群聊中机器人会响应所有消息（过滤功能在路线图中）
- **单 Runtime 实例** — 每次部署仅一个 NanoBot，未实现多 Runtime 负载均衡
- **手动扩容** — 本骨架不包含自动化基础设施扩展能力

---

## 仓库结构

```
im-agent-bridge/
├── gateway/                 # Rust Gateway（Core 层）— 路由、会话、调度
├── deploy/
│   ├── edge-server/         # Matterbridge — Telegram ↔ Gateway 桥接
│   ├── internal-server/     # NanoBot Runtime + Shopify MCP 配置
│   └── postgres/            # PostgreSQL + pg_cron 设置
├── SSoT/
│   ├── schema/migrations/   # Goose SQL 迁移（Schema 权威来源）
│   └── api/                 # TypeSpec API 契约（端点权威来源）
├── openspec/                # 功能提案与变更规格
└── .context/                # AI 上下文资产（项目强约束）
```

---

## 开发约束（必须遵守）

- **API 变更** → 先改 `SSoT/api/main.tsp`，编译后再实现
- **DB 变更** → 先在 `SSoT/schema/migrations/` 添加 Goose 迁移
- **禁止跨层调用** — Bridge 不直接调 Runtime；Runtime 不直接连 Telegram
- **MCP 凭证** — 只能存在于 Runtime `.env`，禁止写入数据库

```bash
make api-compile          # TypeSpec → OpenAPI
make api-gen-rs           # OpenAPI → Rust 类型
make db-migrate-up        # 应用待执行迁移
make db-migrate-status    # 查看迁移状态
cd gateway && cargo test  # 运行 Gateway 单元 + 集成测试
```

---

## 商业版 — CBECOps Pro

开源骨架覆盖核心桥接能力，永久免费。生产级增强功能请查看 **[CBECOps Pro](https://cbec.injoys.ai/)**。

| | 社区版（开源） | CBECOps Pro | Enterprise |
|--|--------------|-------------|------------|
| **价格** | 免费 | 联系我们 | 联系我们 |
| **Telegram 文本** | ✅ | ✅ | ✅ |
| **Shopify MCP** | ✅ | ✅ | ✅ |
| **自托管** | ✅ | ✅ | ✅ |
| **富媒体（图片、文件）** | ❌ | ✅ | ✅ |
| **多店铺路由** | ❌ | ✅ | ✅ |
| **WhatsApp / LINE 渠道** | ❌ | ✅ | ✅ |
| **SSO & 团队权限** | ❌ | ✅ | ✅ |
| **审计日志** | ❌ | ✅ | ✅ |
| **托管服务选项** | ❌ | ✅ | ✅ |
| **定制开发** | ❌ | ❌ | ✅ |
| **SLA & 优先支持** | ❌ | ✅ | ✅ |

→ **[了解更多：cbec.injoys.ai](https://cbec.injoys.ai/)**

---

## 参与贡献

欢迎提交 Bug 报告、文档改进、新 Runtime 适配器和 MCP 工具模板。

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

安全漏洞请**不要**公开 Issue，见 [SECURITY.md](SECURITY.md) 通过邮件私下报告。

---

## License

Apache 2.0 — 详见 [LICENSE](LICENSE)。

Copyright 2026 InJoys AI
