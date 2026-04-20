## ADDED Requirements

### Requirement: 多店铺 Shopify MCP 子进程配置模板
系统必须（MUST）在 `deploy/internal-server/nanobot/config.json.example` 的 `tools.mcpServers` 节声明多店铺 shopify-mcp 子进程条目，每个店铺使用独立具名条目（如 `shopify-store1`），凭证以 `${VAR}` 语法内联于 `args`，并设置 `toolTimeout: 10`（BR-052）。

#### Scenario: 多店铺 mcpServers 条目格式正确
- **WHEN** 运维将 config.json.example 复制为实际 config.json 并填入真实凭证，NanoBot 容器启动
- **THEN** NanoBot 按 mcpServers 中的每个条目各启动一个 `shopify-mcp` 子进程（`npx shopify-mcp --clientId ... --clientSecret ... --domain ...`）
- **AND** 已启动的子进程在 NanoBot 工具列表中可被发现和调用

#### Scenario: toolTimeout 10s 配置生效（BR-052）
- **WHEN** NanoBot 调用 shopify-mcp 工具操作超过 10 秒
- **THEN** NanoBot 终止该工具调用并返回用户可理解的降级提示（"工具暂不可用" 语义）
- **AND** 不挂起 Gateway，Gateway 可在 15s hard timeout 内收到响应

---

### Requirement: Shopify 凭证通过环境变量注入，禁止入库
系统必须（MUST）确保 Shopify MCP 凭证（clientId、clientSecret、domain）通过 `.env` + `${VAR}` 语法注入 config.json，不得硬编码于代码仓库，不得在 PostgreSQL 中创建任何 MCP 配置/凭证相关字段。（criterion.md §3.7 MUST NOT；BR-030；BR-033；security_policy.md）

#### Scenario: 凭证通过 ${VAR} 语法从 .env 注入
- **WHEN** 运维将实际凭证写入 `nanobot/.env`，Docker Compose 加载 `.env` 并启动 nanobot 容器
- **THEN** NanoBot 容器内 `config.json` 的 `args` 字段中 `${SHOPIFY_STORE1_CLIENT_ID}` 等变量被正确替换为实际值
- **AND** 代码仓库中不存在任何包含真实 Shopify 凭证的文件

#### Scenario: PostgreSQL 无 MCP 配置字段（MUST NOT）
- **WHEN** 执行 `SELECT table_name FROM information_schema.tables WHERE table_name LIKE 'mcp_%'`
- **THEN** 返回空结果集
- **AND** 确认不存在任何存储 MCP clientId、clientSecret、domain 的数据库表

#### Scenario: 凭证未完全提供时安全降级（edge_cases.md §4）
- **WHEN** 运维遗漏配置 `.env` 中的 Shopify 凭证并启动 NanoBot 容器
- **THEN** NanoBot 记录子进程启动失败告警，但不随之崩溃（不影响主服务 Gateway 与 LLM 通信）
- **AND** 当请求涉及使用 shopify-mcp 相关工具时，直接返回工具不可用提示，不会出现进程式挂起

---

### Requirement: MEMORY.md 格式样板声明可用 MCP 实例与工具允许列表
系统必须（MUST）提供 MEMORY.md 格式样板（TAD §9.4.1），声明当前环境可用的 MCP Server 实例（含 MCP Server 名称、显示名称、品类、地区、币种、时区等元数据），并支持通过工具允许列表限制 MCP 工具访问范围（BR-034，RISK-B003）。

#### Scenario: MEMORY.md 声明多店铺 MCP Server，NanoBot 成功加载
- **WHEN** 运维将填写完毕的 MEMORY.md 放置于 NanoBot 运行时上下文目录（`./nanobot-data/memory/`）
- **THEN** NanoBot 启动后可感知当前环境可用的 Shopify 店铺 MCP 实例列表
- **AND** AI Agent 在处理用户查询时可根据 MEMORY.md 声明的店铺信息选择正确的 MCP 实例

#### Scenario: 默认策略（全工具启用）与工具允许列表限制（BR-034，RISK-B003）
- **WHEN** MEMORY.md 中未配置任何工具允许列表（默认状态）
- **THEN** 该 Shopify MCP 暴露的所有工具均默认开启，可被 AI Agent 调用
- **WHEN** 发现不安全工具并在 MEMORY.md 中显式声明了工具允许列表，且用户请求调用列表之外的工具
- **THEN** 该被排除的工具调用不被执行
- **AND** NanoBot 审计日志记录该拦截尝试（可见于 NanoBot stdout/文件输出，不入库）

---

### Requirement: MCP 工具调用审计日志（BR-034）
系统必须（MUST）在 NanoBot stdout 或文件输出中记录 MCP 工具调用的审计日志，日志内容包含调用的工具名称、调用时间、结果状态；审计日志禁止入库（criterion.md §3.7），禁止在日志中记录 Shopify 凭证明文（security_policy.md §高敏感数据）。

#### Scenario: MCP 工具调用成功时审计日志可见
- **WHEN** NanoBot 成功调用 shopify-mcp 工具（如 `get_products`）
- **THEN** NanoBot stdout/日志文件中出现该工具调用的审计记录（含工具名、时间、状态）
- **AND** PostgreSQL 中无新增 MCP 调用日志行

#### Scenario: 审计日志不包含敏感凭证
- **WHEN** NanoBot 调用 shopify-mcp 工具
- **THEN** NanoBot 日志输出中不包含 `clientId`、`clientSecret` 等 Shopify 凭证明文
