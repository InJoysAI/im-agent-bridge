# IM-Agent-Bridge

**Your Shopify store's 24/7 AI assistant on Telegram — saves hours every week on order lookups, inventory alerts, and customer replies. Fully self-hosted. All data stays with you.**

Purpose-built for cross-border Shopify sellers who want real AI automation without cloud lock-in or SaaS fees.

[中文文档](README.cn.md) · [Official Website](https://cbec.injoys.ai/) · [Report an Issue](../../issues)

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Stage: MVP v1.1](https://img.shields.io/badge/stage-MVP%20v1.1-orange.svg)](#feature-status)
[![Self-hosted](https://img.shields.io/badge/hosting-self--hosted-blueviolet.svg)](#quick-deploy)
[![Telegram](https://img.shields.io/badge/channel-Telegram-26A5E4.svg)](#architecture)
[![Shopify MCP](https://img.shields.io/badge/tools-Shopify%20MCP-96BF48.svg)](#architecture)

---

## See It in Action

Real Telegram conversations with a live Shopify store — no staging data, no mocks:

**Ask your bot about pending high-value orders:**

![Seller asks "查询 200 美元以上订单" — bot queries Shopify, returns order fulfillment analysis with pricing insights and next actions](resource/order.png)

**Scan your entire product catalog for pricing and stock issues:**

![Seller asks "查询 200 美元以上商品信息" — bot returns full product list categorised by price tier, flags zero-stock and mis-priced items, recommends priority fixes](resource/product.png)

> Captured from a real test session using NanoBot runtime with Shopify MCP connected to a development store.

---

## What Your Bot Can Do Right Now

| Ask it in plain language | What happens under the hood |
|-------------------------|-----------------------------|
| `Where is order #US-20456?` | Live Shopify lookup → shipment location, carrier, ETA |
| `Which SKUs have fewer than 10 units left?` | Inventory scan → prioritised low-stock alert list |
| `List all products over $200` | Catalog pull → price tiers, stock levels, status flags |
| `Draft a refund reply for order #EU-8821` | AI-written response, professional tone, no customer PII exposed |
| `Summarise today's open support issues` | Aggregates orders + customer data into a triage-ready digest |

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
| Linux VPS | 1 vCPU / 1 GB RAM is enough for a single store |
| Docker & Docker Compose v2 | `docker compose version` should show v2.20+ |
| Telegram Bot Token | Create one at [@BotFather](https://t.me/BotFather) in 2 minutes |
| Shopify OAuth credentials | From your [Shopify Partners dashboard](https://partners.shopify.com/) |
| LLM API key | Any OpenAI-compatible provider (e.g. GPT-4o) |

---

### Option A — One-Command Stack ✨ *(recommended)*

```bash
git clone https://github.com/your-org/im-agent-bridge.git
cd im-agent-bridge
./quickstart.sh
```

`quickstart.sh` will:
1. Copy all `.env.example` files and open each one for editing
2. Copy NanoBot's `config.json.example` and `MEMORY.md.example`
3. Run `docker compose up -d --build` — starts all 5 services in the correct dependency order

Once the script completes:

```bash
curl http://localhost:8080/health   # → {"status":"ok"}
```

Open Telegram, message your bot, and start querying your store.

---

### Option B — Manual Step-by-Step *(for developers)*

<details>
<summary>Expand manual setup</summary>

**Step 1 — PostgreSQL**
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

**Step 2 — NanoBot Runtime**
```bash
cd deploy/internal-server/nanobot
cp .env.example .env && cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md
# Edit .env: LLM_API_KEY + SHOPIFY_STORE1_* credentials
docker compose up -d
```

**Step 3 — Gateway**
```bash
cd gateway
cp .env.example .env   # GATEWAY_BEARER_TOKEN, DATABASE_URL, BRIDGE_URL
cargo run              # or: docker build + run
```

**Step 4 — Matterbridge**
```bash
cd deploy/edge-server
cp .env.example .env   # TELEGRAM_BOT_TOKEN, GATEWAY_URL, GATEWAY_BEARER_TOKEN
docker compose up -d
```

</details>

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
