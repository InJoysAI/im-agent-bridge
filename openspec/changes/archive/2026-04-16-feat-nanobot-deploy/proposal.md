# Change: NanoBot 服务部署

## Why

NanoBot 是 IM Agent Bridge 的 AI Agent Runtime（HKUDS/nanobot），必须部署为可被 Gateway RuntimeAdapter 调用的 HTTP 服务（端口 :8900）。当前仓库缺少 NanoBot 的 Docker 容器配置及 `deploy/internal-server/nanobot/` 部署文件，导致 Gateway RuntimeAdapter 无法在 Internal Server 上建立完整调用链路。

本提案交付 NanoBot 的 Dockerfile、docker-compose.yml、配置模板（`config.json.example`）及密钥样例（`.env.example`），使 NanoBot 以 OpenAI-compatible API 模式（`nanobot serve`，`:8900`）运行，并通过 volume 挂载缓解 RISK-002（容器重启导致对话记忆丢失）。

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | openspec/proposal-roadmap.md |
| roadmap_source_supplement | N/A |
| phase | Phase 3（Runtime 部署 + 持久化日志） |
| change_id | feat-nanobot-deploy |
| title | NanoBot 服务部署 |
| business_goal | NanoBot Docker 容器化（Python 3.10+ + Node.js）；`GET /health` 健康检查端点；`config.json` volume 挂载作为单一配置源（`providers` LLM 节 + 空 `tools.mcpServers`），secret 用 `${VAR}` 引用 `.env` |
| in_scope | Dockerfile（Python 3.10+ + Node.js，`pip install nanobot-ai[api]`，CMD `nanobot serve`）；`deploy/internal-server/nanobot/docker-compose.yml`（健康检查、volume 挂载 `./nanobot-data:/home/nanobot/.nanobot`）；`config.json.example`（providers LLM 骨架 + 空 `tools.mcpServers`，secret `${VAR}` 语法）；`.env.example`（`LLM_API_KEY` 等 secret 样例） |
| out_of_scope | `tools.mcpServers` Shopify MCP 配置（由 feat-nanobot-shopify-mcp 处理）；MEMORY.md 内容（由 feat-nanobot-shopify-mcp 处理） |
| dependencies | 前置: `feat-infra-gateway-scaffold`（已完成）；被依赖: `feat-nanobot-shopify-mcp` |
| acceptance_criteria | 容器启动后 health = healthy；`GET /health` → `{"status":"ok"}`；`POST /v1/chat/completions` → 200 |
| key_tasks | 1. Dockerfile（0.5 天）；2. docker-compose.yml（0.25 天）；3. config.json.example + .env.example（0.25 天）；4. 验证 NanoBot 启动（0.25 天） |
| risks | RISK-002（NanoBot 本地状态丢失）— volume 挂载 `./nanobot-data:/home/nanobot/.nanobot` 持久化对话记忆 |
| related_context_assets | `.context/architecture/deployment_view.md`；`.context/architecture/tech_stack.md`；`.context/criterion.md`（§3.6, §3.7）；`.context/architecture/security_policy.md` |
| milestones | N/A（1 天内交付，无子里程碑） |
| coverage_scope | N/A |
| gate_vs_non_gate | N/A |
| change_management | N/A |
| ops_support | N/A |
| kpi | N/A |
| risk_acceptance_policy | RISK-002 通过 volume 挂载缓解，可接受 |

## What Changes

### 新增功能
- `deploy/internal-server/nanobot/Dockerfile`：Python 3.10+ + Node.js LTS，安装 `nanobot-ai[api]`，CMD `nanobot serve`，EXPOSE 8900
- `deploy/internal-server/nanobot/docker-compose.yml`：nanobot 服务编排（端口 :8900、healthcheck `GET /health`、volume 挂载 `./nanobot-data:/home/nanobot/.nanobot`、restart: unless-stopped、logging 日志轮转）
- `deploy/internal-server/nanobot/config.json.example`：NanoBot `providers` LLM 节骨架 + 空 `tools.mcpServers`，secret 使用 `${VAR}` 语法
- `deploy/internal-server/nanobot/.env.example`：`LLM_API_KEY` 及其他 config.json 引用 secret 的占位样例

### 修改功能
- 无 Breaking Change

### 技术实现
- Dockerfile 同时安装 Python 3.10+ 和 Node.js LTS，为后续 `npx shopify-mcp` 子进程预置运行时（criterion.md §3.6）
- `nanobot-ai[api]` 额外依赖激活 `nanobot serve` HTTP API 模式（端口 :8900，OpenAI-compatible）
- `config.json` 放入 volume 挂载目录（`./nanobot-data/`），禁止入库；`config.json.example` 仅含 `${VAR}` 引用，可安全提交
- `tools.mcpServers` 留空（`{}`），Shopify MCP 配置由 `feat-nanobot-shopify-mcp` 补充

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/nanobot-deployment/spec.md` — NanoBot 容器部署规范（Dockerfile / compose / config.json 行为与验收）

### 涉及的代码
- **新增**：
  - `deploy/internal-server/nanobot/Dockerfile`
  - `deploy/internal-server/nanobot/docker-compose.yml`
  - `deploy/internal-server/nanobot/config.json.example`
  - `deploy/internal-server/nanobot/.env.example`

- **修改**：
  - 无

### 依赖关系
- **依赖**：`feat-infra-gateway-scaffold`（已完成）
- **被依赖**：`feat-nanobot-shopify-mcp`

### 风险与注意事项
- **RISK-002**（NanoBot 本地会话状态丢失）：volume 挂载 `./nanobot-data:/home/nanobot/.nanobot` 持久化对话记忆，缓解容器重启导致上下文丢失（criterion.md §3.6）
- Dockerfile 中 Node.js 运行时为后续 `npx shopify-mcp` 子进程预置（`feat-nanobot-shopify-mcp`），本提案 `tools.mcpServers` 留空
- 实际 `config.json` 须加入 `.gitignore`，防止 LLM API Key 泄露（criterion.md §4）

### 验证标准
- ✅ `docker compose up nanobot` 后，容器 health = healthy
- ✅ `GET http://localhost:8900/health` → `{"status":"ok"}`
- ✅ `POST http://localhost:8900/v1/chat/completions` → HTTP 200

### 关联 Context 资产
| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.6 Runtime 约束（MUST/MUST NOT）；§3.7 MCP 配置禁止入库；§4 凭证管理 |
| architecture | `.context/architecture/tech_stack.md` | NanoBot Python 3.10+；`nanobot-ai[api]`；Node.js 运行时要求；volume 挂载规范 |
| architecture | `.context/architecture/deployment_view.md` | Internal Server 部署拓扑；nanobot 容器划分；volume 路径；config.json 配置说明 |
| architecture | `.context/architecture/security_policy.md` | 凭证禁止硬编码；环境变量注入；`.gitignore` 要求 |
| domain | `.context/domain/business_rules.md` | BR-022（Runtime 可替换）；RISK-002（状态丢失缓解） |
