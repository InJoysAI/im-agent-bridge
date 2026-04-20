# IM-Agent-Bridge

**Your Shopify store's 24/7 AI assistant on Telegram — saves you hours every week on order lookups, low-stock alerts, and professional customer replies. Fully self-hosted. All your data and credentials stay on your own server.**

Built for cross-border Shopify sellers who want reliable, real Shopify MCP tool calls without vendor lock-in, SaaS fees, or sending customer data to the cloud.

[中文文档](README.cn.md) · [Official Website](https://cbec.injoys.ai/) · [Report an Issue](../../issues)

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Stage: MVP v1.1](https://img.shields.io/badge/stage-MVP%20v1.1-orange.svg)](#feature-status)
[![Self-hosted](https://img.shields.io/badge/hosting-self--hosted-blueviolet.svg)](#quick-deploy)
[![Telegram](https://img.shields.io/badge/channel-Telegram-26A5E4.svg)](#architecture)
[![Shopify MCP](https://img.shields.io/badge/tools-Shopify%20MCP-96BF48.svg)](#architecture)

---

## See It in Action

Real examples from a live Shopify development store — no mocks, no staging data:

**Example 1: Check order status and fulfillment**

```
User:  Where is order #US-20456?

Bot:   Order #US-20456 is confirmed and shipped via DHL.
       Tracking number: DHLEU123456789
       Expected delivery: 2026-04-25
       Customer paid $287.50.
```

**Example 2: Low stock alert**

```
User:  Which SKUs have fewer than 10 units left?

Bot:   ⚠️ Low stock alert (3 items):
       • SKU-B123 (Wireless Earbuds) — 7 units left
       • SKU-X987 (Phone Case) — 4 units left ← Priority
       • SKU-K445 (Charging Cable) — 9 units left
```

**Example 3: Draft a customer reply**

```
User:  Draft a refund reply for order #EU-8821

Bot:   Here's a professional, PII-safe reply you can send:

       Dear valued customer,

       Thank you for reaching out regarding order #EU-8821.
       We have processed your refund request. The amount of $129.00
       will be returned to your original payment method within
       3–5 business days.

       If you have any other questions, feel free to reply.

       Best regards,
       Your Store Team
```

> These are real outputs from NanoBot + Shopify MCP. The bot calls actual Shopify APIs and returns clean, actionable responses in seconds.

**Screenshots from a live test session:**

| Querying high-value orders | Scanning product catalog |
|:-:|:-:|
| ![Seller asks about orders over $200 — bot returns order analysis with pricing insights and next-step suggestions](resource/order.png) | ![Seller asks about products over $200 — bot returns inventory breakdown grouped by price tier, flags issues, recommends priority fixes](resource/product.png) |

---

## Why Self-Host?

| | IM-Agent-Bridge | Typical SaaS AI tools |
|--|----------------|----------------------|
| Customer data | ✅ Never leaves your server | ❌ Uploaded to vendor cloud |
| AI model choice | ✅ GPT-4o, Claude, local LLMs — your call | ❌ Locked to vendor |
| Monthly cost | ✅ VPS + LLM API only | ❌ Per-seat or per-message fees |
| Shopify data | ✅ Real MCP calls via official APIs | ⚠️ Often mocked or rate-limited |
| Auditability | ✅ You own every log | ❌ Depends on vendor policy |

---

## Architecture

```
Telegram ──► Matterbridge (Edge) ──► Gateway (Rust) ──► Runtime (NanoBot)
                                           │                    │
                                      PostgreSQL           Shopify MCP
```

| Layer | Component | Role |
|-------|-----------|------|
| **Channel** | Telegram + Matterbridge | Message in/out — edge only, zero business logic |
| **Bridge** | Matterbridge poller | Pure relay between Telegram and Gateway |
| **Core** | Gateway (Rust) + Runtime + PostgreSQL | All routing, sessions, and tool dispatch |

Key design properties: Bridge and Runtime are fully decoupled (swap either independently). Shopify credentials live only in the Runtime `.env`, never in the database. The Runtime is pluggable — NanoBot ships by default.

---

## Quick Deploy

### What You Need

| Requirement | Notes |
|-------------|-------|
| Linux VPS | 1 vCPU / 1–2 GB RAM is sufficient for one store |
| Docker & Docker Compose v2 | Recommended for easiest setup |
| Telegram Bot Token | Create in 2 minutes at [@BotFather](https://t.me/BotFather) |
| Shopify OAuth credentials | From your [Shopify Partners dashboard](https://partners.shopify.com/) |
| LLM API key | OpenAI-compatible (GPT-4o, Claude, etc.) |

---

### Option A — Recommended: One-Command Setup ✨

```bash
git clone https://github.com/InJoysAI/im-agent-bridge.git
cd im-agent-bridge
./quickstart.sh
```

The script helps copy `.env` files and starts all services.

After setup, test with:

```bash
curl http://localhost:8080/health   # → {"status":"ok"}
```

Then open Telegram and start chatting with your bot.

> **Note**: If `./quickstart.sh` is not yet available in your version, please use **Option B** below.

---

### Option B — Manual Setup *(for developers / advanced users)*

<details>
<summary>Click to expand manual step-by-step instructions</summary>

**1. PostgreSQL**
```bash
cd deploy/postgres
cp .env.example .env   # set POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
docker compose up -d
```

Run migrations (requires [Goose](https://pressly.github.io/goose/)):
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
# Fill in LLM_API_KEY + Shopify credentials
docker compose up -d
```

**3. Gateway**
```bash
cd gateway
cp .env.example .env   # GATEWAY_BEARER_TOKEN, DATABASE_URL, BRIDGE_URL
cargo run              # or: docker build + run
```

**4. Matterbridge**
```bash
cd deploy/edge-server
cp .env.example .env   # TELEGRAM_BOT_TOKEN, GATEWAY_URL, GATEWAY_BEARER_TOKEN
docker compose up -d
```

</details>

> **Tip**: If you want the simplest experience, use **Option A**. Manual setup is mainly for developers who want to customize or debug individual components.

---

## ⚠️ Known Limitations (MVP v1.1)

Read this before deploying in production:

| Limitation | What it means in practice |
|-----------|--------------------------|
| **Text messages only** | Images, voice notes, files, and stickers are silently ignored |
| **Telegram only** | WhatsApp, LINE, WeChat are not supported in this release |
| **Shared group context** | Everyone in a group shares one agent session — no per-user conversation isolation |
| **No @mention filter yet** | In group chats the bot replies to every message, not just messages directed at it |
| **Single runtime instance** | One NanoBot per deployment; no horizontal scaling built in |
| **Self-managed infrastructure** | You handle VPS provisioning, updates, and backups |

---

## Feature Status

| Feature | Status |
|---------|--------|
| Telegram text messages | ✅ Shipped |
| Inbound routing & session management | ✅ Shipped |
| PostgreSQL persistence | ✅ Shipped |
| NanoBot runtime adapter | ✅ Shipped |
| Shopify MCP tool calls | ✅ Shipped |
| `/health` + Prometheus `/metrics` | ✅ Shipped |
| Unified `docker-compose.yml` | ✅ Shipped |
| Group @mention filter | 🔄 Planned |
| Rich media (images, files, voice) | 🔄 CBECOps Pro |
| Multi-store routing | 🔄 CBECOps Pro |
| WhatsApp / LINE / WeChat channels | 🔄 CBECOps Pro |
| SSO & team access management | 🔄 CBECOps Pro |
| Managed hosting | 🔄 CBECOps Pro |

---

## Repository Layout

```
im-agent-bridge/
├── docker-compose.yml       # ← Unified one-command stack
├── quickstart.sh            # ← First-time setup helper
├── gateway/                 # Rust Gateway — routing, sessions, runtime dispatch
│   └── Dockerfile           # Multi-stage Rust build
├── deploy/
│   ├── edge-server/         # Matterbridge — Telegram ↔ Gateway relay
│   ├── internal-server/     # NanoBot runtime + Shopify MCP config
│   └── postgres/            # PostgreSQL + pg_cron retention setup
├── resource/                # Screenshots and demo assets
└── SSoT/
    ├── schema/migrations/   # Goose SQL migrations (authoritative schema)
    └── api/                 # TypeSpec API contracts (authoritative endpoints)
```

---

## Need More? — CBECOps Pro

This skeleton is **free forever** and production-ready for one Telegram channel and one Shopify store. As your operation grows, **[CBECOps Pro](https://cbec.injoys.ai/)** adds team-grade capabilities on top:

| | Community (Open Source) | CBECOps Pro | Enterprise |
|--|------------------------|-------------|------------|
| Telegram text + Shopify MCP | ✅ | ✅ | ✅ |
| Self-hosted | ✅ | ✅ | ✅ |
| Rich media (images, files, voice) | ❌ | ✅ | ✅ |
| Multi-store routing | ❌ | ✅ | ✅ |
| WhatsApp / LINE / WeChat | ❌ | ✅ | ✅ |
| SSO & role-based access | ❌ | ✅ | ✅ |
| Audit logs & compliance | ❌ | ✅ | ✅ |
| Managed hosting option | ❌ | ✅ | ✅ |
| Priority support & SLA | Community | ✅ | ✅ Dedicated |
| Custom development | ❌ | ❌ | ✅ |

→ **[Visit cbec.injoys.ai](https://cbec.injoys.ai/)** to learn more or book a demo

---

## Contributing

Bug reports, runtime adapters, MCP tool templates, and documentation improvements are all welcome.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.  
Security vulnerabilities: please **do not** open a public issue — see [SECURITY.md](SECURITY.md).

---

## License

Apache 2.0 — see [LICENSE](LICENSE).  
Copyright 2026 InJoys AI
