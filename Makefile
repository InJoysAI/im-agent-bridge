# Makefile — OpenFleet Build Targets
#
# ================================
# API Codegen (TypeSpec → OpenAPI → Go/Rust)
# ================================

.PHONY: api-compile api-gen-rs

OPENAPI_YAML ?= SSoT/api/tsp-output/@typespec/openapi3/openapi.yaml

$(OPENAPI_YAML): SSoT/api/main.tsp SSoT/api/tspconfig.yaml
	@echo "Compiling TypeSpec..."
	cd SSoT/api && tsp compile main.tsp --config tspconfig.yaml
	@if grep -q '^paths: {}' $(OPENAPI_YAML); then \
		echo "❌ OpenAPI spec has no paths (paths: {}). Check TypeSpec @service/@route usage."; \
		exit 1; \
	fi
	@echo "✅ OpenAPI spec generated at $(OPENAPI_YAML)"

api-compile:
	@$(MAKE) $(OPENAPI_YAML) --no-print-directory	

api-gen-rs: api-compile
	@echo "Generating Rust API model types..."
	mkdir -p gateway/src/generated
	openapi-generator-cli generate \
		-i $(OPENAPI_YAML) \
		-g rust \
		-o gateway/src/generated \
		-c SSoT/api/openapi-generator-rs.yaml \
		--ignore-file-override SSoT/api/openapi-generator-rs.ignore
	@echo "✅ Rust API types generated at gateway/src/generated/"


# ================================
# Database Migration (Goose)
# ================================

GOOSE_DIR := SSoT/schema/migrations

# Shared env-guard: fails with a friendly message if GOOSE_DRIVER / GOOSE_DBSTRING are unset.
define DB_ENV_GUARD
	if [ -z "$(GOOSE_DRIVER)" ] || [ -z "$(GOOSE_DBSTRING)" ]; then \
		echo "❌ 需要设置 GOOSE_DRIVER 和 GOOSE_DBSTRING"; \
		echo "   PostgreSQL 示例: GOOSE_DRIVER=postgres GOOSE_DBSTRING=postgres://user:pass@host:5432/dbname make $@"; \
		exit 1; \
	fi
endef

.PHONY: db-migrate-up db-migrate-down db-migrate-status

## db-migrate-up: Apply all pending Up migrations.
db-migrate-up:
	@$(DB_ENV_GUARD)
	goose -dir $(GOOSE_DIR) up

## db-migrate-down: Roll back the last applied migration (one step).
db-migrate-down:
	@$(DB_ENV_GUARD)
	goose -dir $(GOOSE_DIR) down

## db-migrate-status: Show current migration version and pending migrations.
db-migrate-status:
	@$(DB_ENV_GUARD)
	goose -dir $(GOOSE_DIR) status

