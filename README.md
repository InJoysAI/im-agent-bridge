# IM-Agent-Bridge

**Turn Telegram into your Shopify command center — query orders, check inventory, and draft customer replies in plain English. All on your own server.**

No vendor lock-in. No customer data leaving your infrastructure. Swap the AI runtime anytime.

[中文文档](README.cn.md) · [Official Website](https://cbec.injoys.ai/) · [Report an Issue](../../issues)

[![License: Apache 2.0](https://img.shields.io/badge/license-Apache%202.0-blue.svg)](LICENSE)
[![Stage: MVP v1.1](https://img.shields.io/badge/stage-MVP%20v1.1-orange.svg)](#feature-status)
[![Self-hosted](https://img.shields.io/badge/hosting-self--hosted-blueviolet.svg)](#quick-start)
[![Telegram](https://img.shields.io/badge/channel-Telegram-26A5E4.svg)](#architecture)
[![Shopify MCP](https://img.shields.io/badge/tools-Shopify%20MCP-96BF48.svg)](#architecture)

---

## See It in Action

Real Telegram conversations with a live Shopify store — no mocking, no staging data:

**Query high-value orders from your store:**

![Query orders over $200 — bot returns fulfillment status, pricing analysis, and actionable suggestions](resource/order.png)

**Scan your entire product catalog for issues:**

![Query products over $200 — bot returns full inventory breakdown, pricing anomalies, and priority action items](resource/product.png)

> These screenshots are from a real test session against a Shopify development store using the NanoBot runtime with Shopify MCP.

---

## What You Can Do Today

Once deployed, message your bot in plain language — it calls real Shopify APIs and replies in seconds:

| Ask your bot | What happens |
|-------------|-------------|
| `Where is order #US-20456?` | Fetches live fulfillment status + tracking number from Shopify |
| `Which SKUs have fewer than 10 units?` | Runs inventory query → returns prioritised low-stock list |
| `Show me all products over $200` | Pulls full catalog with prices, stock levels, and status flags |
| `Draft a refund reply for order #EU-8821` | Generates a professional, PII-safe reply you can send immediately |
| `Summarise today's open support issues` | Compiles unresolved tickets from order + customer data |

---

## Why Self-Host Instead of Using a SaaS Tool?

| | IM-Agent-Bridge | Typical SaaS AI tools |
|--|----------------|----------------------|
| Where does customer PII go? | ✅ Stays on your server | ❌ Uploaded to vendor's cloud |
| AI model choice | ✅ Swap freely — GPT-4o, Claude, local LLMs | ❌ Locked to vendor |
| Running cost | ✅ Pay only for your VPS + LLM API | ❌ Per-seat or per-message SaaS fees |
| Shopify data access | ✅ Real MCP calls via official APIs | ⚠️ Often mocked, rate-limited, or selective |
| Auditability | ✅ You own the logs | ❌ Depends on vendor policy |

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

**Three things that make this architecture pleasant to extend:**
- Bridge and Runtime are fully decoupled — swap either without touching the other
- Shopify credentials live only in the Runtime `.env`, never in the database
- The Runtime is pluggable: NanoBot ships by default; replace it with a single adapter file

---

## Quick Start

### What You Need

| Requirement | Notes |
|-------------|-------|
| Linux VPS | 1 vCPU / 1 GB RAM is enough for a single store |
| Docker & Docker Compose | Runs all services — no Rust needed for non-developers |
| Telegram Bot Token | Create one at [@BotFather](https://t.me/BotFather) in 2 minutes |
| Shopify OAuth credentials | From your [Shopify Partners dashboard](https://partners.shopify.com/) |
| LLM API key | Any OpenAI-compatible provider (GPT-4o, etc.) |
| [Goose](https://pressly.github.io/goose/) | Only needed for local development / schema changes |

---

### Step 1 — Start PostgreSQL

```bash
cd deploy/postgres
cp .env.example .env
# Edit .env: set POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
docker compose up -d
```

Apply the schema (one-time, requires Goose):

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

### Step 2 — Configure NanoBot (AI Runtime + Shopify MCP)

```bash
cd deploy/internal-server/nanobot
cp .env.example .env            # ← fill in your keys here
cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md   # optional: customise bot persona
docker compose up -d
```

Your `.env` for a single store looks like:

```dotenv
LLM_API_KEY=sk-your-openai-key

SHOPIFY_STORE1_CLIENT_ID=your-client-id
SHOPIFY_STORE1_CLIENT_SECRET=your-client-secret
SHOPIFY_STORE1_DOMAIN=yourstore.myshopify.com
```

Adding a second store? Append three more lines — see `.env.example` for the pattern.

### Step 3 — Start the Gateway

```bash
cd gateway
cp .env.example .env
# Set: GATEWAY_BEARER_TOKEN, DATABASE_URL, BRIDGE_URL
cargo run
```

### Step 4 — Connect Telegram via Matterbridge

```bash
cd deploy/edge-server
cp .env.example .env
# Set: TELEGRAM_BOT_TOKEN, GATEWAY_URL, GATEWAY_BEARER_TOKEN
docker compose up -d
```

**That's it.** Open Telegram, message your bot, and start querying your store.

```bash
curl http://localhost:8080/health   # → {"status":"ok"}
```

---

## ⚠️ Known Limitations (MVP v1.1)

This is an honest summary of what the current MVP does **not** support. Please read before deploying:

| Limitation | Impact |
|-----------|--------|
| **Text messages only** | Images, voice notes, files, and stickers are silently ignored |
| **Telegram only** | WhatsApp, LINE, WeChat are not supported in this release |
| **Shared group context** | Everyone in a group shares one agent session — no per-user conversation isolation |
| **No @mention filter yet** | Bot responds to every message in a group, not just ones directed at it |
| **Single runtime instance** | One NanoBot per deployment; horizontal scaling is not built in |
| **Manual infrastructure** | You manage VPS provisioning, updates, and backups |

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
| Group @mention filter | 🔄 Planned |
| Rich media (images, files, voice) | 🔄 CBECOps Pro |
| Multi-store routing | 🔄 CBECOps Pro |
| WhatsApp / LINE / WeChat channels | 🔄 CBECOps Pro |
| SSO & team access management | 🔄 CBECOps Pro |
| Managed / hosted deployment | 🔄 CBECOps Pro |

---

## Repository Layout

```
im-agent-bridge/
├── gateway/                 # Rust Gateway — routing, sessions, runtime dispatch
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

The open-source skeleton is **free forever** and production-ready for one Telegram channel and one Shopify store. As your operation scales, **[CBECOps Pro](https://cbec.injoys.ai/)** adds team-grade capabilities on top:

| | Community (Open Source) | CBECOps Pro | Enterprise |
|--|------------------------|-------------|------------|
| Telegram text + Shopify MCP | ✅ | ✅ | ✅ |
| Self-hosted deployment | ✅ | ✅ | ✅ |
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
