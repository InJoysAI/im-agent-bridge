#!/usr/bin/env bash
# quickstart.sh — IM-Agent-Bridge first-time setup helper
#
# Usage:
#   chmod +x quickstart.sh
#   ./quickstart.sh
#
# What it does:
#   1. Copies all .env.example files to .env (skips if .env already exists)
#   2. Opens each .env in your editor so you can fill in credentials
#   3. Copies config.json.example and MEMORY.md.example (NanoBot)
#   4. Runs docker compose up -d (all services, in dependency order)
#
# Requirements: Docker Compose v2, bash, a terminal editor (or $EDITOR set)

set -euo pipefail

EDITOR="${EDITOR:-nano}"
BOLD="\033[1m"
GREEN="\033[0;32m"
YELLOW="\033[1;33m"
CYAN="\033[0;36m"
RESET="\033[0m"

step() { echo -e "\n${BOLD}${CYAN}▶ $1${RESET}"; }
ok()   { echo -e "  ${GREEN}✔ $1${RESET}"; }
warn() { echo -e "  ${YELLOW}⚠ $1${RESET}"; }

echo -e "${BOLD}"
echo "   IM-Agent-Bridge — Quick Setup"
echo "   --------------------------------"
echo -e "${RESET}"

# ── Step 1: Copy .env files ────────────────────────────────────────────────
step "Copying .env.example files..."

copy_env() {
  local src="$1" dst="$2"
  if [ -f "$dst" ]; then
    warn "$dst already exists — skipping (delete it first to reset)"
  else
    cp "$src" "$dst"
    ok "Created $dst"
  fi
}

copy_env "deploy/postgres/.env.example"                              "deploy/postgres/.env"
copy_env "deploy/internal-server/nanobot/.env.example"               "deploy/internal-server/nanobot/.env"
copy_env "deploy/edge-server/.env.example"                           "deploy/edge-server/.env"
copy_env "gateway/.env.example"                                      "gateway/.env"

# ── NanoBot config files ───────────────────────────────────────────────────
if [ ! -f "deploy/internal-server/nanobot/config.json" ]; then
  cp "deploy/internal-server/nanobot/config.json.example" \
     "deploy/internal-server/nanobot/config.json"
  ok "Created deploy/internal-server/nanobot/config.json"
else
  warn "deploy/internal-server/nanobot/config.json already exists — skipping"
fi

if [ ! -f "deploy/internal-server/nanobot/memory/MEMORY.md" ]; then
  cp "deploy/internal-server/nanobot/memory/MEMORY.md.example" \
     "deploy/internal-server/nanobot/memory/MEMORY.md"
  ok "Created deploy/internal-server/nanobot/memory/MEMORY.md"
else
  warn "MEMORY.md already exists — skipping"
fi

# ── Step 2: Edit credentials ───────────────────────────────────────────────
step "Opening .env files for editing..."
echo -e "  ${YELLOW}Fill in your credentials in each file, then save and close.${RESET}\n"

ENV_FILES=(
  "deploy/postgres/.env"
  "deploy/internal-server/nanobot/.env"
  "deploy/edge-server/.env"
  "gateway/.env"
)

for f in "${ENV_FILES[@]}"; do
  echo -e "  Press Enter to open ${BOLD}$f${RESET}..."
  read -r
  $EDITOR "$f"
done

# ── Step 3: Launch all services ────────────────────────────────────────────
step "Starting all services with Docker Compose..."
docker compose up -d --build

echo ""
step "Waiting for Gateway health check..."
sleep 5

if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
  echo -e "\n${BOLD}${GREEN}✅ All services are up!${RESET}"
  echo -e "   Gateway:     http://localhost:8080/health"
  echo -e "   Metrics:     http://localhost:8080/metrics"
  echo ""
  echo -e "   ${BOLD}Open Telegram and message your bot to get started.${RESET}"
else
  echo -e "\n${YELLOW}⚠ Gateway not yet reachable — services may still be starting.${RESET}"
  echo -e "  Check status with:  ${BOLD}docker compose ps${RESET}"
  echo -e "  View logs with:     ${BOLD}docker compose logs -f gateway${RESET}"
fi
