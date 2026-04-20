# Contributing to IM-Agent-Bridge

Thank you for your interest in contributing! This document outlines how to participate in the project.

## Code of Conduct

By participating, you agree to maintain a respectful and constructive environment for everyone.

## How to Contribute

### Reporting Bugs

1. Search [existing issues](../../issues) first to avoid duplicates.
2. Open a new issue using the **Bug Report** template.
3. Include: environment details, steps to reproduce, expected vs actual behavior.

### Requesting Features

1. Open a new issue using the **Feature Request** template.
2. Describe the use case and why it matters for cross-border e-commerce operators.

### Submitting Pull Requests

1. **Fork** the repository and create a branch from `main`.
2. Follow the [Architecture Constraints](#architecture-constraints) below.
3. Ensure `cargo test` passes for `gateway/` changes.
4. Update relevant documentation (README, `.context/`, SSoT if applicable).
5. Open a PR with a clear description of the change and its motivation.

## Architecture Constraints (MUST follow)

- **No cross-layer calls**: Bridge must not directly call Runtime; Runtime must not directly connect to Telegram.
- **API changes**: Modify `SSoT/api/main.tsp` first, compile, then implement.
- **Database changes**: Add a Goose migration in `SSoT/schema/migrations/` before any schema change.
- **No credentials in DB**: MCP credentials/instance configs must never be stored in the database.
- **Feature proposals**: Non-trivial changes should have an `openspec/` proposal before implementation.

## Development Setup

```bash
# 1. Start PostgreSQL
cp deploy/postgres/.env.example deploy/postgres/.env
docker compose -f deploy/postgres/docker-compose.yml up -d postgres

# 2. Run migrations
export GOOSE_DRIVER=postgres
export GOOSE_DBSTRING='postgres://<user>:<pass>@127.0.0.1:<port>/<db>?sslmode=disable'
make db-migrate-up

# 3. Start Gateway
cd gateway && cp .env.example .env
cargo run

# 4. Run tests
cd gateway && cargo test
```

## Contributor License Agreement (CLA)

By submitting a Pull Request, you agree that your contribution is licensed under the [Apache 2.0 License](LICENSE) and that InJoys AI retains the right to use your contribution in both the open-source and commercial versions of the product.

## Questions?

Open a [GitHub Discussion](../../discussions), email **support@injoys.ai**, or reach out via [cbec.injoys.ai](https://cbec.injoys.ai/).
