## ADDED Requirements

### Requirement: NanoBot 容器化构建
系统必须（MUST）提供 Dockerfile，基于 Python 3.10+ + Node.js LTS，安装 `nanobot-ai[api]`，以 `nanobot serve` 启动 HTTP API 服务（端口 :8900）。

#### Scenario: Dockerfile 构建成功
- **WHEN** 在 `deploy/internal-server/nanobot/` 执行 `docker build -t nanobot .`
- **THEN** 构建过程无报错，生成可运行的 nanobot 镜像
- **AND** 镜像同时包含 Python 3.10+、pip、Node.js LTS 及 npx 可执行文件

#### Scenario: NanoBot 服务正常启动
- **WHEN** 通过 `docker compose up nanobot` 启动容器（挂载有效 `config.json`）
- **THEN** 容器进入 healthy 状态（healthcheck 通过）
- **AND** 端口 :8900 可访问

---

### Requirement: Docker Compose 服务编排
系统必须（MUST）在 `deploy/internal-server/nanobot/docker-compose.yml` 中配置 nanobot 服务，包含：端口映射 `127.0.0.1:8900:8900`（仅宿主机 localhost 可达，禁止绑定 `0.0.0.0`）、健康检查（`test: curl -f http://localhost:8900/health`）、volume 挂载 `./nanobot-data:/home/nanobot/.nanobot`、`restart: unless-stopped` 和日志轮转（`logging`）。

#### Scenario: healthcheck 配置有效
- **WHEN** 容器启动后 healthcheck 执行 `curl -f http://localhost:8900/health`
- **THEN** 返回 HTTP 200 且 body 包含 `{"status":"ok"}`
- **AND** Docker 将容器状态标记为 `healthy`

#### Scenario: volume 挂载持久化数据
- **WHEN** 执行 `docker compose restart nanobot`（或容器因异常重启）
- **THEN** 宿主机 `./nanobot-data/` 目录内容（对话记忆、config.json 等）保持不变
- **AND** NanoBot 重启后可继续读取已有 config.json（缓解 RISK-002）

#### Scenario: 持久化目录路径验证（RISK-007 适配）
- **WHEN** 容器首次启动后发送一条测试消息（`POST /v1/chat/completions`，含 `session_id`）
- **THEN** 宿主机 `./nanobot-data/` 目录内出现 NanoBot 会话状态文件
- **AND** 若文件未出现，须检查 NanoBot 实际写入路径（RISK-007：TAD 记录 `~/.local/state/nano-bots/`，HKUDS/nanobot 实际路径为 `~/.nanobot/`），并据结果调整 volume 挂载点后重新验证

#### Scenario: 日志轮转配置有效
- **WHEN** 检查 `docker-compose.yml` 中 nanobot 服务的 `logging` 节点
- **THEN** 存在 `driver: json-file` 且配置 `max-size`（建议 `10m`）及 `max-file`（建议 `3`）
- **AND** 长期运行时宿主机日志不会无限增长（防止 AI agent 大量输出日志导致磁盘溢出）

#### Scenario: Runtime API 端口不公网暴露（M-4）
- **WHEN** 检查 `docker-compose.yml` 中 nanobot 服务的 `ports` 配置
- **THEN** 端口绑定为 `127.0.0.1:8900:8900`，而非 `0.0.0.0:8900:8900`
- **AND** NanoBot API 仅宿主机 localhost 可达，不对 Internal Server 以外的网络暴露（security_policy.md §3）

---

### Requirement: config.json.example 配置模板
系统必须（MUST）提供 `deploy/internal-server/nanobot/config.json.example`，包含 `providers` 节 LLM 配置骨架（secret 以 `${VAR}` 语法引用）和空 `tools.mcpServers`（`{}`）。

#### Scenario: secret 不含真实凭证
- **WHEN** 检查 `config.json.example` 的 `providers.*.apiKey` 及所有凭证字段
- **THEN** 所有凭证字段均使用 `${VAR_NAME}` 占位符语法，不含真实 API Key
- **AND** 文件可安全提交到 Git 仓库

#### Scenario: tools.mcpServers 为空占位
- **WHEN** 检查 `config.json.example` 中 `tools.mcpServers` 节
- **THEN** 该节为空对象 `{}`
- **AND** 备注说明 Shopify MCP 条目由 `feat-nanobot-shopify-mcp` 补充

---

### Requirement: .env.example 密钥样例
系统必须（MUST）提供 `deploy/internal-server/nanobot/.env.example`，声明 `config.json.example` 中 `${VAR}` 引用的全部 secret 变量（最少包含 `LLM_API_KEY`），值为占位符非真实凭证。

#### Scenario: .env.example 覆盖所有 VAR 引用
- **WHEN** 对比 `config.json.example` 中的 `${VAR_NAME}` 引用列表与 `.env.example` 声明的变量列表
- **THEN** 所有 `${VAR_NAME}` 在 `.env.example` 中均有对应条目
- **AND** 变量值为示例占位符（如 `sk-your-llm-api-key`），不含真实凭证

---

### Requirement: NanoBot health 端点可用
系统必须（MUST）在 NanoBot 容器启动后，`GET /health` 端点返回 `{"status":"ok"}`，供 healthcheck 及外部监控使用。

#### Scenario: health 端点正常响应
- **WHEN** 向运行中的 NanoBot 容器发送 `GET http://localhost:8900/health`
- **THEN** 返回 HTTP 200
- **AND** 响应 body 为 `{"status":"ok"}`

---

### Requirement: NanoBot chat completions 端点可用
系统必须（MUST）在 NanoBot 容器启动后，`POST /v1/chat/completions` 端点满足以下约束（api_strategy.md §3.2）：`session_id` 必传（缺省时 NanoBot 退为 `api:default`，导致所有会话串扰）；`messages` 数组严格限 1 条（传 0 条或多条返回 HTTP 400）；不支持 `stream: true`（传入返回 HTTP 400）。

#### Scenario: chat completions 正常响应（含 session_id，messages=1）
- **WHEN** 向运行中的 NanoBot 容器发送 `POST http://localhost:8900/v1/chat/completions`，body 为 `{"model":"nanobot","messages":[{"role":"user","content":"test"}],"session_id":"test-session-1"}`
- **THEN** 返回 HTTP 200
- **AND** 响应 body 符合 OpenAI Chat Completion 格式（包含 `choices[0].message.content`）

#### Scenario: stream=true 返回 HTTP 400
- **WHEN** 向运行中的 NanoBot 容器发送 `POST http://localhost:8900/v1/chat/completions`，body 含 `"stream": true`
- **THEN** 返回 HTTP 400（NanoBot 不支持 streaming，api_strategy.md §3.2）
