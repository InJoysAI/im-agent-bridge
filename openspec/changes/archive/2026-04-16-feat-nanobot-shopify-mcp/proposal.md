# Change: Shopify MCP 实例配置

## Why

본提案在 NanoBot Runtime 容器中配置 Shopify MCP 子进程，使 AI Agent 具备直接查询多店铺 Shopify 数据的能力。

**业务驱动（Roadmap 对齐：Phase 3，business_goal）**:
- 用户在 Telegram 通过自然语言查询 Shopify 店铺数据（订单状态、商品库存、商品信息等）
- 每个店铺对应一个独立的 `shopify-mcp` 子进程（`geli2001/shopify-mcp`），以 `npx shopify-mcp` 方式运行在 nanobot 容器内
- NanoBot 通过 `MEMORY.md` 声明可用 MCP 实例，通过 `config.json → tools.mcpServers` 管理 MCP 子进程生命周期
- Shopify 凭证必须通过环境变量注入（`${VAR}` 语法引用 `.env`），**严禁入库或写入 PostgreSQL**（criterion.md §3.7 MUST NOT）

**上游依赖验证**:
- `feat-nanobot-deploy`（#9）已完成：NanoBot 容器（Python + Node.js）正常启动，健康检查通过，config.json.example 骨架已存在（`providers` LLM 节已配置，`tools.mcpServers` 为空占位）
- `feat-runtime-nanobot-adapter`（#7）已完成：Gateway 可正常调用 NanoBot Runtime POST /v1/chat/completions，session_id 必传，messages 严格 1 条

## What Changes

### 新增功能

- **多店铺 MCP 配置模板**：在 `deploy/internal-server/nanobot/config.json.example` 的 `tools.mcpServers` 节新增多店铺 shopify-mcp 子进程条目（每店铺一个具名条目，如 `shopify-store1`），凭证 (`--clientId`/`--clientSecret`/`--domain`) 作为 `args` 内联，值通过 `${VAR}` 语法引用 `.env`；新增 `toolTimeout: 10`（对应 BR-052）
- **`nanobot/.env.example` 追加 Shopify 凭证变量**：`SHOPIFY_STORE1_CLIENT_ID`、`SHOPIFY_STORE1_CLIENT_SECRET`、`SHOPIFY_STORE1_DOMAIN` 等；多店铺按格式追加
- **MEMORY.md 格式样板**：编写 NanoBot Agent Runtime 的 MEMORY.md 格式样板（TAD §9.4.1），声明可用 MCP Server 列表（含 url/name/工具允许列表），供运维在 `./nanobot-data/memory/` 中按实际环境填写；MCP 工具允许列表封锁超出预期的工具暴露（BR-034，RISK-B003）

### 修改功能

- **`config.json.example`**：在已有的 `providers`（LLM 配置）骨架基础上，补全 `tools.mcpServers` 多店铺子进程配置

### 技术实现

- Shopify MCP 子进程启动方式：`"command": "npx", "args": ["shopify-mcp", "--clientId", "${VAR}", ...]`（stdio MCP，Node.js），由 NanoBot 容器内的 Node.js 运行时执行
- `toolTimeout: 10`：对应 BR-052（MCP 工具调用 ≤ 10s 超时）；MCP 不可达时 NanoBot 自动返回降级提示，不阻塞 Gateway
- 凭证绝对隔离：config.json 实际文件（含真实凭证）须加入 `.gitignore`；PostgreSQL 无 MCP 配置字段（验证：查询 information_schema 确认无 `mcp_*` 表）
- MCP 工具调用审计日志：由 NanoBot stdout/文件输出，禁止入库（BR-034）

## Impact

### 涉及的规范（Specs）

- **新增**：`specs/nanobot-shopify-mcp/spec.md` — Shopify MCP 实例配置行为规范（多店铺配置、凭证、超时、工具限制、审计日志）

### 涉及的代码

- **新增/修改**：
  - `deploy/internal-server/nanobot/config.json.example`（补全 `tools.mcpServers` 多店铺模板）
  - `deploy/internal-server/nanobot/.env.example`（追加 Shopify 凭证变量）
  - `deploy/internal-server/nanobot/memory/MEMORY.md.example`（MEMORY.md 格式样板，供运维参考）

- **运行时文件（不入库，由运维管理）**：
  - `./nanobot-data/config.json`（真实凭证，volume 挂载）
  - `./nanobot-data/memory/MEMORY.md`（实际 MCP 实例声明，NanoBot 原生支持加载供 AI Agent 上下文参考）
  - `nanobot/.env`（真实 secret，Docker Compose 加载）

### 依赖关系

- **依赖**：`feat-nanobot-deploy`（#9，已完成）— NanoBot 容器 + config.json 骨架
- **依赖**：`feat-runtime-nanobot-adapter`（#7，已完成）— Gateway ↔ NanoBot 调用链
- **被依赖**：`feat-e2e-integration-test`（#14）— E2E 联调须 Shopify MCP 正常运行

### 风险与注意事项

- **RISK-003（Shopify MCP 可用性依赖）**：BR-052 10s 超时 + NanoBot 降级提示已封控；MCP 子进程崩溃时 NanoBot 自动重启子进程（Docker `restart: unless-stopped` 提供容器级保障，但子进程本身异常退出由 NanoBot 框架处理，风险残留需联调验证）
- **RISK-B003（MCP 工具超出预期暴露范围）**：MEMORY.md 中显式配置工具允许列表（BR-034），超出列表的工具不可调用；凭证通过 `${VAR}` 语法保护，禁止硬编码
- **RISK-006（凭证泄露）**：`config.json`（实际文件）和 `.env` 须加入 `.gitignore`；审计日志禁止记录凭证明文（security_policy.md §高敏感数据）
- ~~**RISK-007（TAD 与工具能力差距）**~~：已解决。已确认 NanoBot 支持 `memory/MEMORY.md` 文件加载并主动注入。

### 验证标准

- ✅ shopify-mcp 子进程启动：NanoBot 启动时，`config.json` 中每个 `mcpServers` 条目对应的 shopify-mcp 子进程正常运行
- ✅ MEMORY.md 加载验证：通过检查 NanoBot 启动日志（例如 grep "Loaded memory/MEMORY.md"）或调用调试端点确认实例上下文注入成功
- ✅ 凭证隔离：PostgreSQL 无 MCP 配置/凭证字段（`SELECT table_name FROM information_schema.tables WHERE table_name LIKE 'mcp_%'` 返回空）
- ✅ 凭证缺失安全降级与告警：故意移除 `.env` 凭证，确认 NanoBot 打印相应的启动错误日志告警但不崩溃；后续涉及该 MCP 的请求返回类似“工具暂不可用”预设文案
- ✅ BR-052 超时：MCP 调用超过 10s → NanoBot 返回降级提示，不挂起 Gateway
- ✅ BR-034 工具限制：MEMORY.md 工具允许列表已声明，超出列表的工具不可调用
- ✅ BR-034 审计日志：MCP 工具调用可见于 NanoBot 日志，无数据入库，且确认日志不含敏感凭证（client_secret）
- ✅ `make errcode-gen` — 本提案不涉及新错误码（MCP 超时/不可达 error_code 已在 criterion.md §5.4 定义：`MCP_TIMEOUT`、`MCP_UNAVAILABLE`，由 feat-runtime-nanobot-adapter 处理）

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| `roadmap_source_primary` | `openspec/proposal-roadmap.md` |
| `roadmap_source_supplement` | N/A |
| `phase` | Phase 3 |
| `business_goal` | 多店铺 shopify-mcp 子进程配置 + MEMORY.md 格式样板 + 凭证 ${VAR} 注入；PostgreSQL MUST NOT 存储 MCP 配置 |
| `dependencies` | feat-nanobot-deploy (#9), feat-runtime-nanobot-adapter (#7) |
| `acceptance_criteria` | shopify-mcp 子进程启动；MEMORY.md 加载；凭证隔离；BR-052 超时；BR-034 工具限制；BR-034 审计日志 |

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.6 Runtime 约束；§3.7 MCP 禁止入库（MUST NOT）；§4 凭证管理；§5.4 MCP_TIMEOUT/MCP_UNAVAILABLE 错误码 |
| architecture | `.context/architecture/tech_stack.md` | Shopify MCP（geli2001/shopify-mcp，Node.js，npx）；NanoBot config.json/MEMORY.md；禁止 MCP 配置持久化 |
| architecture | `.context/architecture/deployment_view.md` | config.json 多店铺模板规范；volume 挂载；.env.example |
| architecture | `.context/architecture/security_policy.md` | 凭证分类（高敏感）；禁止 MCP 凭证入库；日志脱敏 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-003（Shopify MCP 可用性）；RISK-006（凭证泄露）；RISK-007（工具链差距） |
| domain | `.context/domain/business_rules.md` | BR-033（MCP 凭证管理）；BR-034（MCP 工具发现与审计）；BR-052（MCP 超时 10s）；BR-061（MCP 调用失败处理） |
