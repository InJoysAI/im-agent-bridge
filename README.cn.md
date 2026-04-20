# IM-Agent-Bridge

**在 Telegram 里用自然语言查订单、设库存预警、生成安全客服回复——数据在您自己的服务器上，运行成本极低。**

面向跨境 Shopify 卖家的轻量、可自托管 IM + AI Agent 桥接骨架。把任意 AI Runtime 接入 Telegram，并调用**真实 Shopify MCP 工具**——无需云平台绑定，无需向第三方共享客户数据。

[English](README.md) · [官网](https://cbec.injoys.ai/) · [Issues](../../issues) · [![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE) [![阶段](https://img.shields.io/badge/stage-MVP%20v1.1-orange)](CHANGELOG.md) [![自托管](https://img.shields.io/badge/hosting-self--hosted-purple)](#快速启动)

---

## 实际效果长什么样？

部署完成后，直接在 Telegram 里发消息给 Bot，它会调用真实的 Shopify API 并即时回复：

```
您  →  订单 #US-20456 到哪儿了？

Bot →  📦 订单 #US-20456 — 运输中
       承运商：DHL Express | 快递单：1234567890
       预计送达：4月23日（2个工作日）
       最新动态：4月19日 14:32 UTC，已从上海中转站发出
```

```
您  →  SKU WIDGET-BLK-XL 还有货吗？

Bot →  ✅ WIDGET-BLK-XL — 库存 47 件
       仓库：US-West | 数据更新：2小时前
```

```
您  →  哪些 SKU 库存低于 10 件？

Bot →  ⚠️ 库存预警 — my-gadgets-shop
       CABLE-USB-C    →  3件  🔴 紧急
       GADGET-RED-S   →  7件  ⚠️ 偏低
       CASE-BLK-M     →  9件  ⚠️ 偏低
       建议优先补货 CABLE-USB-C。
```

```
您  →  帮我给订单 #EU-8821 写一封退款确认邮件，专业语气。

Bot →  草稿如下 👇

       "您好，感谢您的联系。您的订单 #EU-8821 退款（$42.00）
       已处理完成，预计 5-7 个工作日内退回原支付账户。
       对给您带来的不便我们深感抱歉，期待再次为您服务。"

       需要调整语气或补充内容吗？
```

> 以上示例需要已配置 Shopify MCP 连接，详见[快速启动](#快速启动)。

---

## 为什么要自托管？

| | IM-Agent-Bridge | 典型 SaaS AI 工具 |
|--|----------------|-----------------|
| 客户 PII 数据 | ✅ 留在您的服务器 | ❌ 上传至供应商云端 |
| AI Runtime 选择 | ✅ 随时替换 | ❌ 锁定在供应商 |
| 月度成本 | ✅ 仅服务器成本 | ❌ 按席位/按消息计费 |
| Shopify MCP 调用 | ✅ 真实 API 调用 | ⚠️ 通常为模拟或受限 |
| 多店铺支持 | 🔄 路线图中 | ✅ 通常已支持 |

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

**核心设计理念：**

- Bridge 不直接调用 Runtime → 两侧均可独立替换
- Shopify 凭证仅存于 Runtime `.env` — 绝不写入数据库，保护 PII 安全
- Runtime 可插拔：默认搭载 NanoBot，通过一个 Adapter 文件即可替换

---

## 快速启动

### 环境依赖

| 工具 | 用途 |
|------|------|
| Docker & Docker Compose | 运行所有服务 |
| Telegram Bot Token | 通过 [@BotFather](https://t.me/BotFather) 申请 |
| Shopify OAuth 凭证 | 每个店铺一组，在 Partners 后台申请 |
| LLM API Key | OpenAI-compatible（如 GPT-4o） |
| [Goose](https://pressly.github.io/goose/)（仅开发环境） | 数据库迁移 |

> **提示：** 仅本地 Gateway 开发才需要 Rust，Docker Compose 覆盖其他所有服务。

---

### 第一步 — 启动 PostgreSQL

```bash
cd deploy/postgres
cp .env.example .env          # 设置 POSTGRES_USER / POSTGRES_PASSWORD / POSTGRES_DB
docker compose up -d
```

执行数据库迁移：

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

### 第二步 — 配置并启动 NanoBot（AI Runtime）

```bash
cd deploy/internal-server/nanobot
cp .env.example .env            # 填写 LLM_API_KEY + 各店铺 Shopify 凭证
cp config.json.example config.json   # 按店铺配置 MCP Server 条目
cp memory/MEMORY.md.example memory/MEMORY.md   # 可选：自定义 Bot 人设
docker compose up -d
```

**`.env` 中填写 Shopify 凭证（每家店铺一组）：**

```dotenv
LLM_API_KEY=sk-your-key

# 店铺 Slug 全大写，连字符替换为下划线
SHOPIFY_STORE1_CLIENT_ID=your-client-id
SHOPIFY_STORE1_CLIENT_SECRET=your-client-secret
SHOPIFY_STORE1_DOMAIN=store1.myshopify.com
```

### 第三步 — 启动 Gateway

```bash
cd gateway
cp .env.example .env
# 必填：GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run
```

可用端点：

- `POST /gateway/inbound` — 接收来自 Matterbridge 的消息
- `GET /health` — 健康检查
- `GET /metrics` — Prometheus 指标

### 第四步 — 启动 Matterbridge（Telegram Edge）

```bash
cd deploy/edge-server
cp .env.example .env
# 必填：TELEGRAM_BOT_TOKEN / GATEWAY_URL / GATEWAY_BEARER_TOKEN
docker compose up -d
```

**完成。** 在 Telegram 里向 Bot 发一条消息试试吧。

```bash
curl http://localhost:8080/health
# → {"status":"ok"}
```

---

## 已知限制（MVP v1.1）

上线前请了解当前 MVP 的边界：

| 限制 | 说明 |
|------|------|
| **仅支持文本** | 图片、语音、文件、贴纸均不处理 |
| **仅支持 Telegram** | WhatsApp、LINE、微信等暂未支持 |
| **群聊上下文共享** | 群内所有成员共用一个 Agent 会话，无单用户隔离 |
| **暂无 @提及过滤** | 群聊中 Bot 会响应所有消息（过滤功能在路线图中） |
| **单 Runtime 实例** | 每次部署仅一个 NanoBot，未实现多 Runtime 负载均衡 |
| **手动扩容** | 基础设施扩展需手动操作 |

---

## 功能状态

| 功能 | 状态 |
|------|------|
| Telegram 文本消息 | ✅ 已完成 |
| Gateway 入站路由 | ✅ 已完成 |
| 会话持久化（PostgreSQL） | ✅ 已完成 |
| NanoBot Runtime 适配器 | ✅ 已完成 |
| Shopify MCP 工具调用 | ✅ 已完成 |
| `/health` 健康检查 | ✅ 已完成 |
| Prometheus `/metrics` | ✅ 已完成 |
| 群聊 @提及过滤 | 🔄 规划中 |
| 富媒体（图片、文件） | 🔄 CBECOps Pro |
| 多店铺路由 | 🔄 CBECOps Pro |
| WhatsApp / LINE / 微信 | 🔄 CBECOps Pro |
| SSO & 团队权限管理 | 🔄 CBECOps Pro |
| 托管服务选项 | 🔄 CBECOps Pro |

---

## 仓库结构

```
im-agent-bridge/
├── gateway/                 # Rust Gateway — 路由、会话、Runtime 调度
├── deploy/
│   ├── edge-server/         # Matterbridge — Telegram ↔ Gateway 中继
│   ├── internal-server/     # NanoBot Runtime + Shopify MCP 配置
│   └── postgres/            # PostgreSQL + pg_cron 数据保留设置
└── SSoT/
    ├── schema/migrations/   # Goose SQL 迁移（Schema 权威来源）
    └── api/                 # TypeSpec API 契约（端点权威来源）
```

---

## 免费骨架之上 — CBECOps Pro

开源骨架适合单 Telegram 渠道、单 Shopify 店铺的生产环境，且**永久免费**。当您的业务规模扩大，**[CBECOps Pro](https://cbec.injoys.ai/)** 提供团队级增强能力：

| | 社区版（开源） | CBECOps Pro | Enterprise |
|--|--------------|-------------|------------|
| Telegram 文本 + Shopify MCP | ✅ | ✅ | ✅ |
| 自托管 | ✅ | ✅ | ✅ |
| 富媒体（图片、文件） | ❌ | ✅ | ✅ |
| 多店铺路由 | ❌ | ✅ | ✅ |
| WhatsApp / LINE / 微信 | ❌ | ✅ | ✅ |
| SSO & 角色权限管理 | ❌ | ✅ | ✅ |
| 审计日志 | ❌ | ✅ | ✅ |
| 托管服务选项 | ❌ | ✅ | ✅ |
| SLA & 优先技术支持 | 社区 | ✅ | ✅ 专属 |
| 定制开发 | ❌ | ❌ | ✅ |

→ **[cbec.injoys.ai](https://cbec.injoys.ai/)** — 联系我们获取定价和演示

---

## 参与贡献

欢迎提交 Bug 报告、文档改进、新 Runtime 适配器和 MCP 工具模板。

详见 [CONTRIBUTING.md](CONTRIBUTING.md)。

安全漏洞请**不要**公开 Issue，见 [SECURITY.md](SECURITY.md) 通过邮件私下报告。

---

## License

Apache 2.0 — 详见 [LICENSE](LICENSE)。  
Copyright 2026 InJoys AI
