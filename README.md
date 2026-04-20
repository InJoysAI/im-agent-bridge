# IM-Agent-Bridge

> **Give your Shopify store a 24/7 AI assistant on Telegram — self-hosted, private, and runtime-swappable.**

[中文文档](README.cn.md) · [Website](https://cbec.injoys.ai/) · [Issues](../../issues) · [License: Apache 2.0](LICENSE)

![Build](https://img.shields.io/badge/build-passing-brightgreen) ![License](https://img.shields.io/badge/license-Apache%202.0-blue) ![MVP](https://img.shields.io/badge/stage-MVP%20v1.1-orange) ![Self-hosted](https://img.shields.io/badge/hosting-self--hosted-purple)

---

## Why IM-Agent-Bridge?

Cross-border Shopify sellers spend hours answering the same questions: *"Where's my order?", "Is this SKU still in stock?", "Can you handle this refund?"*

IM-Agent-Bridge solves the **last-mile problem** of plugging any AI Agent runtime into Telegram and giving it access to **real Shopify MCP tools** — without cloud lock-in, without sharing your customer data with a third party, and without being forced into a specific AI vendor.

- 🔌 **Runtime-swappable** — NanoBot ships by default; replace it with any OpenAI-compatible runtime via a single adapter swap
- 🔒 **Privacy-first** — all traffic stays on your own infrastructure; customer PII never leaves your server
- 🛒 **Real Shopify MCP calls** — order lookup, inventory checks, customer data — executed inside the runtime, not mocked
- ⚡ **Lightweight skeleton** — Rust Gateway handles routing and session management with minimal resource footprint

---

## Architecture

```
Telegram ──► Matterbridge (Edge) ──► Gateway (Rust) ──► Runtime (NanoBot)
                                           │                    │
                                      PostgreSQL           Shopify MCP
```

| Layer | Component | Responsibility |
|-------|-----------|---------------|
| **Channel** | Telegram + Matterbridge | Message in / out — edge only |
| **Bridge** | Matterbridge poller | Relay only — zero business logic |
| **Core** | Gateway (Rust) + Runtime + PostgreSQL | All routing, session, tool dispatch |

**Key design principles:**
- Bridge never calls Runtime directly → clean separation, easy to swap either side
- MCP credentials live only in the Runtime `.env` — never stored in the database
- Gateway is the single authority for session state and runtime dispatch

---

## Quick Start

### Prerequisites

| Tool | Purpose |
|------|---------|
| Docker & Docker Compose | Run all services |
| [Goose](https://pressly.github.io/goose/) | DB migrations |
| Telegram Bot Token | From [@BotFather](https://t.me/BotFather) |
| Shopify OAuth credentials | Per store, from Partners dashboard |
| An LLM API key | OpenAI-compatible (GPT-4o, etc.) |

> **Tip:** Rust is only required for local Gateway development. Docker Compose covers everything else.

---

### Step 1 — Bootstrap PostgreSQL

```bash
cd deploy/postgres
cp .env.example .env
# Edit .env: set POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
docker compose up -d
```

### Step 2 — Run DB Migrations

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://user:password@127.0.0.1:5432/im_agent_bridge?sslmode=disable'
make db-migrate-up
```

### Step 3 — Configure NanoBot Runtime

```bash
cd deploy/internal-server/nanobot
cp .env.example .env          # Set LLM_API_KEY + Shopify store credentials
cp config.json.example config.json  # Wire MCP server entries per store
cp memory/MEMORY.md.example memory/MEMORY.md  # Customize agent persona
docker compose up -d
```

### Step 4 — Start the Gateway

```bash
cd gateway
cp .env.example .env
# Set: GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run
# Gateway endpoints: POST /gateway/inbound  |  GET /health  |  GET /metrics
```

### Step 5 — Start Matterbridge (Edge)

```bash
cd deploy/edge-server
cp .env.example .env
# Set: TELEGRAM_BOT_TOKEN, GATEWAY_URL, GATEWAY_BEARER_TOKEN
docker compose up -d
```

**Health check:**
```bash
curl http://localhost:8080/health
# → {"status":"ok"}
```

---

## Real-World Usage Examples

Once running, your Telegram bot understands natural language requests backed by live Shopify data:

| User Message (Telegram) | What Happens Under the Hood |
|------------------------|----------------------------|
| `Where is order #12345?` | Runtime calls Shopify MCP → fetches fulfillment status → replies with tracking info |
| `Is SKU WIDGET-BLK-XL still in stock?` | Shopify MCP inventory query → returns current stock level |
| `Summarize open support tickets from today` | Agent composes from order + customer data |
| `Draft a refund reply for order #98765` | Agent generates PII-safe, brand-appropriate reply text |

---

## Feature Status (MVP v1.1)

| Feature | Status | Notes |
|---------|--------|-------|
| Telegram text messages | ✅ Done | Via Matterbridge edge |
| Gateway inbound routing | ✅ Done | `POST /gateway/inbound` |
| Session management (PostgreSQL) | ✅ Done | Per-chat session isolation |
| NanoBot runtime adapter | ✅ Done | Default runtime |
| Shopify MCP tool calls | ✅ Done | Order, inventory, customer |
| Health endpoint | ✅ Done | `GET /health` |
| Prometheus metrics | ✅ Done | `GET /metrics` |
| Mention filter (group chats) | 🔄 Planned | Only respond when @mentioned |
| Rich media (images, files) | 🔄 Planned | CBECOps Pro roadmap |
| Multi-store routing | 🔄 Planned | CBECOps Pro roadmap |
| WhatsApp / LINE channels | 🔄 Planned | CBECOps Pro roadmap |
| SSO / team access | 🔄 Planned | CBECOps Pro roadmap |
| Managed hosting | 🔄 Planned | CBECOps Pro roadmap |

---

## Known Limitations

Being honest about the current MVP scope:

- **Text only** — Images, voice, files, and stickers are not processed
- **Telegram only** — WhatsApp, LINE, WeChat, and other IM channels are not supported yet
- **Group chat context is shared** — All members in a group share one agent session (no per-user isolation)
- **No mention filter yet** — In group chats, the bot responds to every message (mention filter is on the roadmap)
- **Single runtime** — Only one NanoBot instance per deployment; multi-runtime load balancing is not implemented
- **Manual scaling** — Infrastructure scaling is not automated in this skeleton

---

## Repository Structure

```
im-agent-bridge/
├── gateway/                 # Rust Gateway (Core Layer) — routing, sessions, dispatch
├── deploy/
│   ├── edge-server/         # Matterbridge — Telegram ↔ Gateway bridge
│   ├── internal-server/     # NanoBot runtime + Shopify MCP config
│   └── postgres/            # PostgreSQL + pg_cron setup
├── SSoT/
│   ├── schema/migrations/   # Goose SQL migrations (source of truth for schema)
│   └── api/                 # TypeSpec API contracts (source of truth for endpoints)
├── openspec/                # Feature proposals and change specs
└── .context/                # AI context assets (authoritative constraints)
```

---

## Development Constraints

> These constraints are enforced across all contributions:

- **API changes** → modify `SSoT/api/main.tsp` first, compile, then implement
- **DB changes** → add a Goose migration in `SSoT/schema/migrations/` first
- **No cross-layer calls** — Bridge never calls Runtime directly; Runtime never connects to Telegram directly
- **MCP credentials** — must never be stored in the database; live only in Runtime `.env`

```bash
make api-compile       # TypeSpec → OpenAPI
make api-gen-rs        # OpenAPI → Rust types
make db-migrate-up     # Apply pending migrations
make db-migrate-status # Check migration state
cd gateway && cargo test  # Run Gateway unit + integration tests
```

---

## Commercial Version — CBECOps Pro

The open-source skeleton covers the core bridge and is free forever. For production-grade features, see **[CBECOps Pro](https://cbec.injoys.ai/)**.

| | Community (Open Source) | CBECOps Pro | Enterprise |
|--|------------------------|-------------|------------|
| **Price** | Free | Contact us | Contact us |
| **Telegram text** | ✅ | ✅ | ✅ |
| **Shopify MCP** | ✅ | ✅ | ✅ |
| **Self-hosted** | ✅ | ✅ | ✅ |
| **Rich media (images, files)** | ❌ | ✅ | ✅ |
| **Multi-store routing** | ❌ | ✅ | ✅ |
| **WhatsApp / LINE channels** | ❌ | ✅ | ✅ |
| **SSO & team access** | ❌ | ✅ | ✅ |
| **Audit logs** | ❌ | ✅ | ✅ |
| **Managed hosting option** | ❌ | ✅ | ✅ |
| **Custom development** | ❌ | ❌ | ✅ |
| **SLA & priority support** | ❌ | ✅ | ✅ |

→ **[Learn more at cbec.injoys.ai](https://cbec.injoys.ai/)**

---

## Contributing

Contributions are welcome — bug reports, docs improvements, new runtime adapters, and MCP tool templates are especially valued.

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Security vulnerabilities: please **do not** open a public issue — see [SECURITY.md](SECURITY.md) for responsible disclosure.

---

## License

Apache 2.0 — see [LICENSE](LICENSE).

Copyright 2026 InJoys AI
