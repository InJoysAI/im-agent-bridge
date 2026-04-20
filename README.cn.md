# IM-Agent-Bridge

**您的 Shopify 店铺 7×24 小时 AI 助理——在 Telegram 里查订单、盯库存、写客服回复，每周为您节省数小时重复工作。全部自托管，数据始终留在您自己的服务器上。**

专为跨境 Shopify 卖家打造——无需依赖云平台，无 SaaS 按量计费，客户数据不出境。

[English README](README.md) · [官网](https://cbec.injoys.ai/) · [反馈 Issue](../../issues)

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![阶段: MVP v1.1](https://img.shields.io/badge/stage-MVP%20v1.1-orange.svg)](#功能状态)
[![自托管](https://img.shields.io/badge/hosting-self--hosted-blueviolet.svg)](#快速部署)
[![Telegram](https://img.shields.io/badge/渠道-Telegram-26A5E4.svg)](#架构概览)
[![Shopify MCP](https://img.shields.io/badge/工具-Shopify%20MCP-96BF48.svg)](#架构概览)

---

## 实际效果

以下是对接真实 Shopify 开发店铺的示例——非模拟，非演示数据：

**示例 1：查询订单状态和发货情况**

```
用户：  订单 #US-20456 到哪儿了？

Bot：   订单 #US-20456 已确认并通过 DHL 发货。
        快递单号：DHLEU123456789
        预计送达：2026-04-25
        客户支付金额：$287.50
```

**示例 2：库存预警**

```
用户：  哪些 SKU 库存低于 10 件？

Bot：   ⚠️ 库存预警（3 项）：
        • SKU-B123（无线耳机）— 剩余 7 件
        • SKU-X987（手机壳）— 剩余 4 件 ← 紧急
        • SKU-K445（充电线）— 剩余 9 件
```

**示例 3：起草客服回复**

```
用户：  帮我给订单 #EU-8821 写退款回复

Bot：   以下是一封专业、隐私安全的回复草稿：

        尊敬的客户您好，

        感谢您就订单 #EU-8821 与我们联系。
        您的退款 $129.00 已处理完成，预计 3-5 个工作日内
        退回至您的原支付方式。

        如有其他问题，欢迎随时回复。

        祝好，
        您的店铺团队
```

> 以上为 NanoBot + Shopify MCP 的真实输出。Bot 调用的是 Shopify 官方 API，返回干净、可操作的回复。

**真实测试会话截图：**

| 查询高价订单 | 扫描商品目录 |
|:-:|:-:|
| ![卖家查询 200 美元以上订单——Bot 返回订单分析、定价洞察和下一步建议](resource/order.png) | ![卖家查询 200 美元以上商品——Bot 返回按价格区间分组的库存明细，标记问题项，给出修复优先级](resource/product.png) |

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
| Linux VPS | 单店铺 1 vCPU / 1-2 GB RAM 即可 |
| Docker & Docker Compose v2 | 推荐方式，部署最简单 |
| Telegram Bot Token | 2 分钟内在 [@BotFather](https://t.me/BotFather) 申请 |
| Shopify OAuth 凭证 | 每个店铺一组，在 [Shopify Partners 后台](https://partners.shopify.com/) 申请 |
| LLM API Key | OpenAI-compatible（GPT-4o、Claude 等） |

---

### 方式 A — 推荐：一键全栈启动 ✨

```bash
git clone https://github.com/InJoysAI/im-agent-bridge.git
cd im-agent-bridge
./quickstart.sh
```

脚本会引导您复制 `.env` 配置文件并一键拉起所有服务。

启动完成后验证：

```bash
curl http://localhost:8080/health   # → {"status":"ok"}
```

打开 Telegram，向 Bot 发消息，开始使用。

---

### 方式 B — 手动分步启动 *（适合开发者 / 高级用户）*

<details>
<summary>点击展开手动步骤</summary>

**1. PostgreSQL**
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

**2. NanoBot Runtime**
```bash
cd deploy/internal-server/nanobot
cp .env.example .env && cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md
# 填写 LLM_API_KEY + Shopify 凭证
docker compose up -d
```

**3. Gateway**
```bash
cd gateway
cp .env.example .env   # GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run              # 或 docker build + run
```

**4. Matterbridge**
```bash
cd deploy/edge-server
cp .env.example .env   # TELEGRAM_BOT_TOKEN / GATEWAY_URL / GATEWAY_BEARER_TOKEN
docker compose up -d
```

</details>

> **提示**：如果您希望最简单的体验，请使用**方式 A**。手动步骤主要面向需要自定义或调试单个组件的开发者。

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
