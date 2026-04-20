# IM-Agent-Bridge

**让 Telegram 成为您的 Shopify 运营中枢——用自然语言查订单、盘库存、写客服回复，全部跑在您自己的服务器上。**

无供应商绑定。客户数据不离境。AI Runtime 随时可换。

[English README](README.md) · [官网](https://cbec.injoys.ai/) · [反馈 Issue](../../issues)

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![阶段: MVP v1.1](https://img.shields.io/badge/stage-MVP%20v1.1-orange.svg)](#功能状态)
[![自托管](https://img.shields.io/badge/hosting-self--hosted-blueviolet.svg)](#快速启动)
[![Telegram](https://img.shields.io/badge/渠道-Telegram-26A5E4.svg)](#架构概览)
[![Shopify MCP](https://img.shields.io/badge/工具-Shopify%20MCP-96BF48.svg)](#架构概览)

---

## 实际效果长什么样？

以下是真实 Telegram 对话截图，数据来自真实 Shopify 测试店铺，非模拟：

**查询昨天的大额订单：**

![用户发送"查询 200 美元以上订单"，Bot 返回履单状态、价格分析和可执行建议](resource/order.png)

**扫描整个商品目录，发现定价和库存问题：**

![用户发送"查询 200 美元以上商品信息"，Bot 返回完整库存明细、异常定价和优先行动项](resource/product.png)

> 截图来自使用 NanoBot Runtime + Shopify MCP 对接 Shopify 开发店铺的真实测试会话。

---

## 您现在就可以做什么？

部署完成后，直接在 Telegram 里用中文（或英文）向 Bot 提问——它会调用真实 Shopify API 并秒速回复：

| 向 Bot 发送 | 发生什么 |
|------------|---------|
| `订单 #US-20456 到哪儿了？` | 实时查询 Shopify 履单状态 + 物流单号 |
| `哪些 SKU 库存低于 10 件？` | 执行库存查询 → 返回按紧急程度排序的预警列表 |
| `列出所有价格超过 200 美元的商品` | 拉取完整目录，含价格、库存量和上架状态 |
| `帮我给订单 #EU-8821 写退款回复` | 生成专业语气、PII 安全的客服回复，可直接发送 |
| `汇总今天未处理的客服问题` | 整合订单 + 客户数据，生成待办事项摘要 |

---

## 为什么选择自托管？

| | IM-Agent-Bridge | 典型 SaaS AI 工具 |
|--|----------------|-----------------|
| 客户 PII 数据存放在哪？ | ✅ 留在您自己的服务器 | ❌ 上传至供应商云端 |
| AI 模型选择 | ✅ 随时替换 — GPT-4o、Claude、本地模型均可 | ❌ 锁定供应商 |
| 月度运行成本 | ✅ 仅 VPS + LLM API 费用 | ❌ 按席位或按消息计费 |
| Shopify 数据访问方式 | ✅ 通过官方 API 的真实 MCP 调用 | ⚠️ 通常为模拟、受限或选择性 |
| 日志可审计性 | ✅ 您完全掌控 | ❌ 取决于供应商策略 |

---

## 架构概览

```
Telegram ──► Matterbridge（Edge）──► Gateway（Rust）──► Runtime（NanoBot）
                                           │                    │
                                      PostgreSQL           Shopify MCP
```

| 层级 | 组件 | 职责 |
|------|------|------|
| **Channel 层** | Telegram + Matterbridge | 消息进出 — 纯边缘节点，不承载业务逻辑 |
| **Bridge 层** | Matterbridge poller | 在 Telegram 与 Gateway 之间纯中继 |
| **Core 层** | Gateway (Rust) + Runtime + PostgreSQL | 全部路由、会话管理、工具调度 |

**让这套架构便于扩展的三个设计决策：**

- Bridge 与 Runtime 完全解耦 — 替换任意一侧无需改动另一侧
- Shopify 凭证仅存于 Runtime `.env` — 绝不写入数据库，保障 PII 安全
- Runtime 可插拔：默认搭载 NanoBot，通过一个 Adapter 文件即可替换

---

## 快速启动

### 您需要准备

| 工具 / 资源 | 说明 |
|------------|------|
| Linux VPS | 单店铺 1 vCPU / 1 GB RAM 即可 |
| Docker & Docker Compose | 运行所有服务，非开发者无需 Rust |
| Telegram Bot Token | 2 分钟内在 [@BotFather](https://t.me/BotFather) 申请 |
| Shopify OAuth 凭证 | 每个店铺一组，在 [Shopify Partners 后台](https://partners.shopify.com/) 申请 |
| LLM API Key | 任意 OpenAI-compatible 供应商（如 GPT-4o） |
| [Goose](https://pressly.github.io/goose/)（仅开发环境） | 本地 Schema 变更时使用 |

---

### 第一步 — 启动 PostgreSQL

```bash
cd deploy/postgres
cp .env.example .env
# 编辑 .env：填写 POSTGRES_USER / POSTGRES_PASSWORD / POSTGRES_DB
docker compose up -d
```

初始化数据库 Schema（仅首次，需要 Goose）：

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

### 第二步 — 配置 NanoBot（AI Runtime + Shopify MCP）

```bash
cd deploy/internal-server/nanobot
cp .env.example .env            # ← 在这里填写您的密钥
cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md   # 可选：自定义 Bot 人设
docker compose up -d
```

单店铺的 `.env` 示例：

```dotenv
LLM_API_KEY=sk-your-openai-key

SHOPIFY_STORE1_CLIENT_ID=your-client-id
SHOPIFY_STORE1_CLIENT_SECRET=your-client-secret
SHOPIFY_STORE1_DOMAIN=yourstore.myshopify.com
```

需要接入第二家店铺？再追加三行即可，格式见 `.env.example`。

### 第三步 — 启动 Gateway

```bash
cd gateway
cp .env.example .env
# 必填：GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run
```

### 第四步 — 通过 Matterbridge 接入 Telegram

```bash
cd deploy/edge-server
cp .env.example .env
# 必填：TELEGRAM_BOT_TOKEN / GATEWAY_URL / GATEWAY_BEARER_TOKEN
docker compose up -d
```

**完成。** 打开 Telegram，向 Bot 发消息，开始查询您的店铺。

```bash
curl http://localhost:8080/health   # → {"status":"ok"}
```

---

## ⚠️ 已知限制（MVP v1.1）

上线前请仔细阅读当前 MVP 的能力边界：

| 限制 | 实际影响 |
|------|---------|
| **仅支持文本消息** | 图片、语音、文件、贴纸会被静默忽略 |
| **仅支持 Telegram** | WhatsApp、LINE、微信本版本不支持 |
| **群聊上下文共享** | 群内所有成员共用一个 Agent 会话，无单用户对话隔离 |
| **暂无 @提及过滤** | 群聊中 Bot 会响应所有消息，而非仅响应被 @ 的消息 |
| **单 Runtime 实例** | 每次部署仅一个 NanoBot，未内置水平扩展 |
| **手动运维** | VPS 配置、升级和备份需自行管理 |

---

## 功能状态

| 功能 | 状态 |
|------|------|
| Telegram 文本消息 | ✅ 已上线 |
| 入站路由 & 会话管理 | ✅ 已上线 |
| PostgreSQL 持久化 | ✅ 已上线 |
| NanoBot Runtime 适配器 | ✅ 已上线 |
| Shopify MCP 工具调用 | ✅ 已上线 |
| `/health` + Prometheus `/metrics` | ✅ 已上线 |
| 群聊 @提及过滤 | 🔄 规划中 |
| 富媒体（图片、文件、语音） | 🔄 CBECOps Pro |
| 多店铺路由 | 🔄 CBECOps Pro |
| WhatsApp / LINE / 微信渠道 | 🔄 CBECOps Pro |
| SSO & 团队权限管理 | 🔄 CBECOps Pro |
| 托管 / 云端部署选项 | 🔄 CBECOps Pro |

---

## 仓库结构

```
im-agent-bridge/
├── gateway/                 # Rust Gateway — 路由、会话、Runtime 调度
├── deploy/
│   ├── edge-server/         # Matterbridge — Telegram ↔ Gateway 中继
│   ├── internal-server/     # NanoBot Runtime + Shopify MCP 配置
│   └── postgres/            # PostgreSQL + pg_cron 数据保留设置
├── resource/                # 截图和演示资源
└── SSoT/
    ├── schema/migrations/   # Goose SQL 迁移（Schema 权威来源）
    └── api/                 # TypeSpec API 契约（端点权威来源）
```

---

## 需要更多能力？— CBECOps Pro

开源骨架**永久免费**，适合单 Telegram 渠道、单 Shopify 店铺的生产环境。当您的业务规模扩大，**[CBECOps Pro](https://cbec.injoys.ai/)** 提供团队级增强能力：

| | 社区版（开源） | CBECOps Pro | Enterprise |
|--|--------------|-------------|------------|
| Telegram 文本 + Shopify MCP | ✅ | ✅ | ✅ |
| 自托管部署 | ✅ | ✅ | ✅ |
| 富媒体（图片、文件、语音） | ❌ | ✅ | ✅ |
| 多店铺路由 | ❌ | ✅ | ✅ |
| WhatsApp / LINE / 微信 | ❌ | ✅ | ✅ |
| SSO & 角色权限管理 | ❌ | ✅ | ✅ |
| 审计日志 & 合规 | ❌ | ✅ | ✅ |
| 托管服务选项 | ❌ | ✅ | ✅ |
| 优先支持 & SLA | 社区 | ✅ | ✅ 专属 |
| 定制开发 | ❌ | ❌ | ✅ |

→ **[访问 cbec.injoys.ai](https://cbec.injoys.ai/)** 了解详情或预约演示

---

## 参与贡献

欢迎提交 Bug 报告、文档改进、新 Runtime 适配器和 MCP 工具模板。

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。  
安全漏洞请**不要**公开 Issue，见 [SECURITY.md](SECURITY.md) 通过邮件私下报告。

---

## License

Apache 2.0 — 详见 [LICENSE](LICENSE)。  
Copyright 2026 InJoys AI
