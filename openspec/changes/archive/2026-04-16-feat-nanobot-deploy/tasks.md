# 实施任务清单

> feat-nanobot-deploy: NanoBot 服务部署（Phase 3, P0）。预计 1 天。SSoT 无更改（无 DB 迁移、无 TypeSpec 变更）。不涉及新错误码（纯部署配置提案，不引入新 Gateway API 端点或错误场景）。

## 1. SSoT 先行检查
- [x] 1.1 确认无 DB schema 变更 — 无 Goose 迁移需求（本提案不新增数据库表或字段）
- [x] 1.2 确认无 Gateway API 合约变更 — 无 `SSoT/api/main.tsp` 修改需求（NanoBot API 为 Runtime 自身端点，不属于 Gateway TypeSpec 管辖范围）
- [x] 1.3 记录结论：SSoT 未更改（纯部署配置，不引入新数据模型或 Gateway API 接口）

## 2. Dockerfile 构建（0.5 天）
- [x] 2.1 创建 `deploy/internal-server/nanobot/Dockerfile`
  - 基础镜像：`python:3.10-slim`
  - 安装 Node.js LTS（通过 nodesource 脚本或等效方式）
  - 运行 `pip install "nanobot-ai[api]"`
  - EXPOSE 8900
  - CMD：`["nanobot", "serve"]`
- [x] 2.2 本地执行 `docker build -t nanobot .` 验证构建无报错
- [x] 2.3 验证镜像同时包含 `python`、`node`、`npx` 可执行文件

## 3. Docker Compose 编排（0.25 天）
- [x] 3.1 创建 `deploy/internal-server/nanobot/docker-compose.yml`
  - service name: `nanobot`
  - ports: `"127.0.0.1:8900:8900"`（仅宿主机 localhost 可达，禁止绑定 `0.0.0.0`，M-4）
  - healthcheck: `test: ["CMD", "curl", "-f", "http://localhost:8900/health"]`，配置 interval / timeout / retries
  - volumes: `./nanobot-data:/home/nanobot/.nanobot`
  - env_file: `.env`
  - restart: `unless-stopped`
  - logging: `driver: json-file`，`max-size: "10m"`，`max-file: "3"`（防止 NanoBot 大量输出日志导致磁盘溢出，M-1）
- [x] 3.2 确认宿主机 `nanobot-data/` 目录在首次 `docker compose up` 时自动创建（Docker 默认行为）

## 4. 配置模板与密钥样例（0.25 天）
- [x] 4.1 创建 `deploy/internal-server/nanobot/config.json.example`
  - `providers.openai.apiKey`: `"${LLM_API_KEY}"`
  - `agents.defaults.model`: 填入推荐 LLM 模型（如 `"openai/gpt-4o"`）
  - `tools.mcpServers`: `{}` （留空，Shopify MCP 由 feat-nanobot-shopify-mcp 补充）
  - 所有 secret 字段使用 `${VAR_NAME}` 语法，禁止硬编码真实凭证
- [x] 4.2 创建 `deploy/internal-server/nanobot/.env.example`
  - `LLM_API_KEY=sk-your-llm-api-key`
  - 其他被 `config.json.example` 引用的 `${VAR_NAME}` 变量（占位值）
- [x] 4.3 确认 `deploy/internal-server/nanobot/config.json`（实际值）已加入 `.gitignore`

## 5. 验证（0.25 天）
- [x] 5.1 复制 `config.json.example` → `config.json`，填入真实 LLM API Key
- [x] 5.2 `docker compose up nanobot`，等待容器 health = healthy
- [x] 5.3 验收：`GET http://localhost:8900/health` → `{"status":"ok"}`
- [x] 5.4 验收：POST /v1/chat/completions（session_id 必传、messages 严格 1 条）→ HTTP 200
  - curl 样例：`curl -s -X POST http://localhost:8900/v1/chat/completions -H "Content-Type: application/json" -d '{"model":"nanobot","messages":[{"role":"user","content":"test"}],"session_id":"test-session-1"}'`
- [x] 5.4a 验收：POST /v1/chat/completions（含 `"stream":true`）→ HTTP 400（api_strategy.md §3.2，M-2）
- [x] 5.5 重启验证：`docker compose restart nanobot`，确认 `./nanobot-data/` volume 数据持久化不丢失
- [x] 5.6 路径验证（RISK-007，M-3）：发送测试消息后确认 `./nanobot-data/` 内出现会话状态文件；若无，检查 NanoBot 实际写入路径（RISK-007：`~/.local/state/nano-bots/` vs `~/.nanobot/`）并据结果调整 volume 挂载点重新验证

## 6. 文档
- [x] 6.1 确认 `deploy/internal-server/nanobot/` 目录下文件齐全：`Dockerfile` / `docker-compose.yml` / `config.json.example` / `.env.example`

## 7. 验证与归档
- [x] 7.1 运行 specflow validate feat-nanobot-deploy --strict（即 `node design/context-dev/tools/specflow/specflow.mjs validate feat-nanobot-deploy --strict`）
- [x] 7.2 运行 specflow archive feat-nanobot-deploy --yes（即 `node design/context-dev/tools/specflow/specflow.mjs archive feat-nanobot-deploy --yes`）
