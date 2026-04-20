# IM-Agent-Bridge

**Query orders, trigger inventory alerts, and generate safe customer replies — all from Telegram, on your own server.**

Purpose-built for Shopify cross-border sellers who want an AI assistant that stays private, costs nothing to run, and connects to any AI runtime they choose.

[中文文档](README.cn.md) · [Website](https://cbec.injoys.ai/) · [Issues](../../issues) · [![License](https://img.shields.io/badge/license-Apache%202.0-blue)](LICENSE) [![Stage](https://img.shields.io/badge/stage-MVP%20v1.1-orange)](CHANGELOG.md) [![Self-hosted](https://img.shields.io/badge/hosting-self--hosted-purple)](#quick-start)

---

## What You Can Do Right Now

Once deployed, send natural-language messages to your Telegram bot — it calls real Shopify APIs and replies instantly:

```
You  →  Where is order #US-20456?

Bot  →  📦 Order #US-20456 — In Transit
        Carrier: DHL Express | Tracking: 1234567890
        Est. delivery: Apr 23 (2 business days)
        Last update: Departed Shanghai hub — Apr 19, 14:32 UTC
```

```
You  →  Is SKU WIDGET-BLK-XL still in stock?

Bot  →  ✅ WIDGET-BLK-XL — 47 units available
        Warehouse: US-West | Updated: 2 hours ago
```

```
You  →  Which SKUs have fewer than 10 units left?

Bot  →  ⚠️ Low Stock Alert — my-gadgets-shop
        CABLE-USB-C    →  3 units  🔴 Critical
        GADGET-RED-S   →  7 units  ⚠️ Low
        CASE-BLK-M     →  9 units  ⚠️ Low
        Recommend restocking CABLE-USB-C urgently.
```

```
You  →  Draft a refund reply for order #EU-8821, professional tone.

Bot  →  Here's a draft 👇

        "Hi [Name], thank you for reaching out.
        Your refund for order #EU-8821 ($42.00) has been processed
        and will appear in 5–7 business days. We apologize for any
        inconvenience and look forward to serving you again."

        Want me to adjust the tone?
```

> **Note:** These examples require a configured Shopify MCP connection. See [Quick Start](#quick-start).

---

## Why Self-Host?

| | IM-Agent-Bridge | Typical SaaS AI tools |
|--|----------------|----------------------|
| Customer PII stays on | ✅ Your server | ❌ Vendor's cloud |
| AI runtime choice | ✅ Swap freely | ❌ Locked to vendor |
| Monthly cost | ✅ Your infra cost | ❌ Per-seat/per-message fees |
| Shopify MCP calls | ✅ Real API calls | ⚠️ Often mocked or limited |
| Multi-store | 🔄 Roadmap | ✅ Usually included |

---

## Architecture

```
Telegram ──► Matterbridge (Edge) ──► Gateway (Rust) ──► Runtime (NanoBot)
                                           │                    │
                                      PostgreSQL           Shopify MCP
```

| Layer | Component | What it does |
|-------|-----------|-------------|
| **Channel** | Telegram + Matterbridge | Message in / out — edge only, no business logic |
| **Bridge** | Matterbridge poller | Pure relay between Telegram and Gateway |
| **Core** | Gateway (Rust) + Runtime + PostgreSQL | All routing, sessions, and tool dispatch |

**Key design choices:**
- The Bridge never calls the Runtime directly → swap either side independently
- MCP credentials live only in the Runtime `.env` — never written to the database
- The Runtime is pluggable: NanoBot ships by default, replace it via a one-file adapter

---

## Quick Start

### Minimum Requirements

- A Linux VPS (1 vCPU / 1 GB RAM is enough for a single store)
- Docker & Docker Compose
- A [Telegram Bot Token](https://t.me/BotFather)
- Shopify OAuth credentials (from your Partners dashboard)
- An OpenAI-compatible API key (GPT-4o, etc.)

> **Local development only:** also needs Rust (stable) and [Goose](https://pressly.github.io/goose/) for DB migrations.

---

### Step 1 — Start PostgreSQL

```bash
cd deploy/postgres
cp .env.example .env          # Set POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
docker compose up -d
```

Then apply the schema:

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

### Step 2 — Configure and Start NanoBot (AI Runtime)

```bash
cd deploy/internal-server/nanobot
cp .env.example .env            # Set LLM_API_KEY + Shopify credentials per store
cp config.json.example config.json   # Wire MCP servers (one entry per Shopify store)
cp memory/MEMORY.md.example memory/MEMORY.md   # Optionally customise bot persona
docker compose up -d
```

**Shopify credential pattern in `.env`:**
```dotenv
LLM_API_KEY=sk-your-key

# One group per store — slug uppercased, hyphens → underscores
SHOPIFY_STORE1_CLIENT_ID=your-client-id
SHOPIFY_STORE1_CLIENT_SECRET=your-client-secret
SHOPIFY_STORE1_DOMAIN=store1.myshopify.com
```

### Step 3 — Start the Gateway

```bash
cd gateway
cp .env.example .env
# Required: GATEWAY_BEARER_TOKEN, DATABASE_URL, BRIDGE_URL
cargo run
```

Available endpoints:
- `POST /gateway/inbound` — receives messages from Matterbridge
- `GET /health` — liveness check
- `GET /metrics` — Prometheus metrics

### Step 4 — Start Matterbridge (Telegram Edge)

```bash
cd deploy/edge-server
cp .env.example .env
# Required: TELEGRAM_BOT_TOKEN, GATEWAY_URL, GATEWAY_BEARER_TOKEN
docker compose up -d
```

**Done.** Send a message to your bot in Telegram.

```bash
curl http://localhost:8080/health
# → {"status":"ok"}
```

---

## Known Limitations (MVP v1.1)

Be aware of these before deploying in production:

| Limitation | Detail |
|-----------|--------|
| **Text messages only** | Images, voice, files, and stickers are ignored |
| **Telegram only** | WhatsApp, LINE, WeChat are not supported yet |
| **Shared group context** | All members of a group share one agent session — no per-user isolation |
| **No @mention filter** | In group chats the bot responds to every message (filter is on the roadmap) |
| **Single runtime instance** | One NanoBot per deployment; no load balancing |
| **No auto-scaling** | You manage infrastructure scaling manually |

---

## Feature Status

| Feature | Status |
|---------|--------|
| Telegram text messages | ✅ Done |
| Gateway inbound routing | ✅ Done |
| Session persistence (PostgreSQL) | ✅ Done |
| NanoBot runtime adapter | ✅ Done |
| Shopify MCP tool calls | ✅ Done |
| `/health` endpoint | ✅ Done |
| Prometheus `/metrics` | ✅ Done |
| Group chat @mention filter | 🔄 Planned |
| Rich media (images, files) | 🔄 CBECOps Pro |
| Multi-store routing | 🔄 CBECOps Pro |
| WhatsApp / LINE channels | 🔄 CBECOps Pro |
| SSO & team access control | 🔄 CBECOps Pro |
| Managed hosting | 🔄 CBECOps Pro |

---

## Repository Layout

```
im-agent-bridge/
├── gateway/                 # Rust Gateway — routing, sessions, runtime dispatch
├── deploy/
│   ├── edge-server/         # Matterbridge — Telegram ↔ Gateway relay
│   ├── internal-server/     # NanoBot runtime + Shopify MCP config
│   └── postgres/            # PostgreSQL + pg_cron retention setup
└── SSoT/
    ├── schema/migrations/   # Goose SQL migrations (authoritative schema)
    └── api/                 # TypeSpec API contracts (authoritative endpoints)
```

---

## Growing Beyond the Skeleton — CBECOps Pro

The open-source skeleton is production-ready for a single Telegram channel and single Shopify store. When your operation grows, **[CBECOps Pro](https://cbec.injoys.ai/)** adds the layers that help teams scale:

| | Community | CBECOps Pro | Enterprise |
|--|-----------|-------------|------------|
| Telegram text + Shopify MCP | ✅ | ✅ | ✅ |
| Self-hosted | ✅ | ✅ | ✅ |
| Rich media (images, files) | ❌ | ✅ | ✅ |
| Multi-store routing | ❌ | ✅ | ✅ |
| WhatsApp / LINE / WeChat | ❌ | ✅ | ✅ |
| SSO & role-based access | ❌ | ✅ | ✅ |
| Audit logs | ❌ | ✅ | ✅ |
| Managed hosting option | ❌ | ✅ | ✅ |
| SLA & priority support | Community | ✅ | ✅ Dedicated |
| Custom development | ❌ | ❌ | ✅ |

→ **[cbec.injoys.ai](https://cbec.injoys.ai/)** — contact us for pricing and a demo

---

## Contributing

Bug reports, runtime adapters, MCP templates, and docs improvements are all welcome. See [CONTRIBUTING.md](CONTRIBUTING.md).

Security issues: do **not** open a public issue — see [SECURITY.md](SECURITY.md) for responsible disclosure.

---

## License

Apache 2.0 — see [LICENSE](LICENSE).  
Copyright 2026 InJoys AI
