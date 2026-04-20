# IM-Agent-Bridge

**您的 Shopify 店铺 7×24 小时 AI 助理——在 Telegram 里查订单、盯库存、写客服回复，每周节省数小时重复工作。全部自托管，数据不出境。**

专为跨境 Shopify 卖家打造，真正的 AI 自动化，无云平台锁定，无 SaaS 按量计费。

[English README](README.md) · [官网](https://cbec.injoys.ai/) · [反馈 Issue](../../issues)

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![阶段: MVP v1.1](https://img.shields.io/badge/stage-MVP%20v1.1-orange.svg)](#功能状态)
[![自托管](https://img.shields.io/badge/hosting-self--hosted-blueviolet.svg)](#快速部署)
[![Telegram](https://img.shields.io/badge/渠道-Telegram-26A5E4.svg)](#架构概览)
[![Shopify MCP](https://img.shields.io/badge/工具-Shopify%20MCP-96BF48.svg)](#架构概览)

---

## 实际效果

以下是连接真实 Shopify 测试店铺的 Telegram 对话截图——非模拟，非演示数据：

**向 Bot 查询待处理的高价订单：**

![卖家发送"查询 200 美元以上订单"，Bot 查询 Shopify 后返回订单履单状态分析、定价洞察和下一步行动建议](resource/order.png)

**扫描整个商品目录，发现定价和库存问题：**

![卖家发送"查询 200 美元以上商品信息"，Bot 返回按价格区间分类的完整商品列表，标记零库存和异常定价，给出优先修复建议](resource/product.png)

> 截图来自使用 NanoBot Runtime + Shopify MCP 对接 Shopify 开发店铺的真实测试会话。

---

## 您的 Bot 现在能做什么？

| 用自然语言提问 | 背后发生了什么 |
|--------------|-------------|
| `订单 #US-20456 到哪儿了？` | 实时查询 Shopify → 物流位置、承运商、预计到达时间 |
| `哪些 SKU 库存低于 10 件？` | 库存扫描 → 按紧急程度排序的预警列表 |
| `列出所有价格超过 200 美元的商品` | 拉取商品目录 → 价格分层、库存量、上架状态 |
| `帮我给订单 #EU-8821 写退款回复` | AI 生成专业语气回复，客户隐私信息不外泄 |
| `汇总今天未处理的客服问题` | 整合订单 + 客户数据，生成可直接分配的处理清单 |

---

## 为什么选择自托管？

| | IM-Agent-Bridge | 典型 SaaS AI 工具 |
|--|----------------|-----------------|
| 客户数据 | ✅ 始终留在您的服务器 | ❌ 上传至供应商云端 |
| AI 模型选择 | ✅ GPT-4o、Claude、本地模型——您决定 | ❌ 锁定供应商 |
| 月度成本 | ✅ 仅 VPS + LLM API 费用 | ❌ 按席位或按消息计费 |
| Shopify 数据 | ✅ 通过官方 API 的真实 MCP 调用 | ⚠️ 通常为模拟或受限 |
| 可审计性 | ✅ 所有日志您完全掌控 | ❌ 取决于供应商策略 |

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

核心设计：Bridge 与 Runtime 完全解耦（两侧可独立替换）。Shopify 凭证仅存于 Runtime `.env`，绝不写入数据库。Runtime 可插拔——默认搭载 NanoBot。

---

## 快速部署

### 您需要准备

| 工具 / 资源 | 说明 |
|------------|------|
| Linux VPS | 单店铺 1 vCPU / 1 GB RAM 即可 |
| Docker & Docker Compose v2 | `docker compose version` 需显示 v2.20+ |
| Telegram Bot Token | 2 分钟内在 [@BotFather](https://t.me/BotFather) 申请 |
| Shopify OAuth 凭证 | 每个店铺一组，在 [Shopify Partners 后台](https://partners.shopify.com/) 申请 |
| LLM API Key | 任意 OpenAI-compatible 供应商（如 GPT-4o） |

---

### 方式 A — 一键全栈启动 ✨ *（推荐）*

```bash
git clone https://github.com/your-org/im-agent-bridge.git
cd im-agent-bridge
./quickstart.sh
```

`quickstart.sh` 会自动：
1. 复制所有 `.env.example` 文件，逐个打开供您填写凭证
2. 复制 NanoBot 的 `config.json.example` 和 `MEMORY.md.example`
3. 执行 `docker compose up -d --build` — 按依赖顺序启动全部 5 个服务

脚本完成后：

```bash
curl http://localhost:8080/health   # → {"status":"ok"}
```

打开 Telegram，向 Bot 发消息，开始查询您的店铺。

---

### 方式 B — 手动分步启动 *（适合开发者）*

<details>
<summary>展开手动步骤</summary>

**第一步 — PostgreSQL**
```bash
cd deploy/postgres
cp .env.example .env   # 设置 POSTGRES_USER / POSTGRES_PASSWORD / POSTGRES_DB
docker compose up -d
```

执行迁移（需 [Goose](https://pressly.github.io/goose/)）：
```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

**第二步 — NanoBot Runtime**
```bash
cd deploy/internal-server/nanobot
cp .env.example .env && cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md
# 编辑 .env：填写 LLM_API_KEY + SHOPIFY_STORE1_* 凭证
docker compose up -d
```

**第三步 — Gateway**
```bash
cd gateway
cp .env.example .env   # GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run              # 或 docker build + run
```

**第四步 — Matterbridge**
```bash
cd deploy/edge-server
cp .env.example .env   # TELEGRAM_BOT_TOKEN / GATEWAY_URL / GATEWAY_BEARER_TOKEN
docker compose up -d
```

</details>

---

## ⚠️ 已知限制（MVP v1.1）

上线前请务必了解：

| 限制 | 实际影响 |
|------|---------|
| **仅支持文本消息** | 图片、语音、文件、贴纸会被静默忽略 |
| **仅支持 Telegram** | WhatsApp、LINE、微信本版本不支持 |
| **群聊上下文共享** | 群内所有成员共用一个 Agent 会话，无单用户对话隔离 |
| **暂无 @提及过滤** | 群聊中 Bot 会响应所有消息，而非仅响应被 @ 的消息 |
| **单 Runtime 实例** | 每次部署仅一个 NanoBot，未内置水平扩展 |
| **自行管理基础设施** | VPS 配置、升级和备份需您自行处理 |

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
| 统一 `docker-compose.yml` | ✅ 已上线 |
| 群聊 @提及过滤 | 🔄 规划中 |
| 富媒体（图片、文件、语音） | 🔄 CBECOps Pro |
| 多店铺路由 | 🔄 CBECOps Pro |
| WhatsApp / LINE / 微信渠道 | 🔄 CBECOps Pro |
| SSO & 团队权限管理 | 🔄 CBECOps Pro |
| 托管服务选项 | 🔄 CBECOps Pro |

---

## 仓库结构

```
im-agent-bridge/
├── docker-compose.yml       # ← 一键全栈启动
├── quickstart.sh            # ← 首次部署引导脚本
├── gateway/                 # Rust Gateway — 路由、会话、Runtime 调度
│   └── Dockerfile           # 多阶段 Rust 构建
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

开源骨架**永久免费**，适合单 Telegram 渠道、单 Shopify 店铺的生产环境。当您的业务规模扩大，**[CBECOps Pro](https://cbec.injoys.ai/)** 提供团队级增强：

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
