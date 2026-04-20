# IM-Agent-Bridge

A lightweight, self-hostable skeleton for connecting IM channels to AI Agent runtimes — built for cross-border e-commerce operators.

[中文文档](README.cn.md) | [Website](https://cbec.injoys.ai/) | [Issues](../../issues)

---

## What It Does

IM-Agent-Bridge solves the "last mile" problem of plugging any Agent runtime (NanoBot, custom builds, Accio-like) into Telegram and giving it access to real Shopify MCP tools — without locking you into a specific vendor or cloud.

**MVP scope:**
- Telegram as the message channel (text messages)
- Gateway (Rust) handles inbound routing, session management, and runtime dispatch
- NanoBot as the default runtime (swappable via Runtime Adapter)
- Shopify MCP tool calls executed inside the runtime
- PostgreSQL for persistence

---

## Architecture

```
Telegram ──► Matterbridge (Edge) ──► Gateway (Rust) ──► Runtime (NanoBot)
                                          │                    │
                                     PostgreSQL           Shopify MCP
```

Three-layer design:

| Layer | Component | Role |
|-------|-----------|------|
| **Channel** | Telegram + Matterbridge | Message in/out |
| **Bridge** | Matterbridge poller | Relay only, no business logic |
| **Core** | Gateway (Rust) + Runtime + PostgreSQL | All business logic lives here |

---

## Quick Start

### Prerequisites

- Docker & Docker Compose
- Rust (stable) — for local Gateway development
- [Goose](https://pressly.github.io/goose/) — for DB migrations
- A Telegram Bot Token

### 1. Start PostgreSQL

```bash
cp deploy/postgres/.env.example deploy/postgres/.env
# Edit .env with your credentials
docker compose -f deploy/postgres/docker-compose.yml up -d postgres
```

### 2. Run Migrations

```bash
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://<user>:<password>@127.0.0.1:<port>/<db>?sslmode=disable'
make db-migrate-up
```

### 3. Start NanoBot Runtime (optional but recommended)

```bash
cd deploy/internal-server/nanobot
cp .env.example .env
cp config.json.example config.json
cp memory/MEMORY.md.example memory/MEMORY.md
docker compose up -d
```

### 4. Start the Gateway

```bash
cd gateway
cp .env.example .env
# Set: GATEWAY_BEARER_TOKEN / DATABASE_URL / BRIDGE_URL
cargo run
```

### 5. Start Matterbridge (Edge)

```bash
cd deploy/edge-server
# Prepare .env and matterbridge.toml, then:
docker compose up -d
```

Gateway endpoints:
- `POST /gateway/inbound` — receive messages from Matterbridge
- `GET /health` — health check
- `GET /metrics` — basic metrics

---

## Repository Structure

```
im-agent-bridge/
├── gateway/                 # Rust Gateway (Core Layer)
├── deploy/
│   ├── edge-server/         # Matterbridge (Channel Layer)
│   ├── internal-server/     # NanoBot runtime
│   └── postgres/            # PostgreSQL setup
├── SSoT/
│   ├── schema/migrations/   # Goose SQL migrations (source of truth)
│   └── api/                 # TypeSpec API contracts (source of truth)
├── openspec/                # Feature proposals
└── docs/                    # Business and product documentation
```

---

## Development Constraints

- API changes → modify `SSoT/api/main.tsp` first, compile, then implement
- DB changes → add a Goose migration in `SSoT/schema/migrations/` first
- No cross-layer calls: Bridge never calls Runtime directly
- MCP credentials must never be stored in the database

---

## Commercial Version (CBECOps Pro)

The open-source skeleton covers the core bridge. For production-grade features — rich media, multi-store routing, SSO, audit logs, managed hosting — see **[CBECOps Pro](https://cbec.injoys.ai/)**.

| Tier | Price | Highlights |
|------|-------|------------|
| Starter | $29/store/mo | Core skeleton + monitoring + email support |
| Pro | $79/store/mo | Rich media + multi-store + priority support |
| Enterprise | $199+/store/mo | Multi-IM + SSO + audit logs + custom dev |

---

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). All contributions are welcome — bug reports, docs, runtime adapters, and new MCP templates.

## License

Apache 2.0 — see [LICENSE](LICENSE).

Copyright 2026 InJoys AI
