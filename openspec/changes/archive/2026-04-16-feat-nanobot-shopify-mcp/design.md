## Context

本提案涉及 NanoBot Runtime 中 Shopify MCP 子进程的配置集成。核心问题在于：
1. **MEMORY.md 加载机制**：HKUDS/nanobot 原生通过 `config.json → tools.mcpServers` 管理 MCP 子进程，并且**原生支持读取 `memory/MEMORY.md` 并在构建上下文时自动注入**。TAD §9.4.1 描述的格式可以直接作为 `memory/MEMORY.md` 放置在 NanoBot Workspace 目录下。
2. **多店铺命名约定**：每个 Shopify 店铺对应 `mcpServers` 中一个具名条目（格式：`shopify-{store-slug}`），如 `shopify-store1`、`shopify-cool-gadgets`。
3. **凭证隔离**：凭证通过 `${VAR}` 引用 `.env`，实际 `config.json` 和 `.env` 须在 `.gitignore` 中。

## Goals / Non-Goals

- Goals:
  - 提供可复制、可扩展的多店铺 shopify-mcp 配置模板（config.json.example）
  - 提供 MEMORY.md 格式样板（TAD §9.4.1）供运维按实际环境填写
  - 确保凭证隔离（${VAR} 注入，禁止入库）
  - 实施 MCP 超时（BR-052）与工具审计日志（BR-034）

- Non-Goals:
  - 在 PostgreSQL 中存储任何 MCP 配置（MUST NOT）
  - 实现 MCP 动态路由或 Gateway 侧 MCP 选择逻辑
  - 实现独立的 MEMORY.md 解析服务
  - 实现 Shopify MCP 本身的业务逻辑

## Decisions

- **Decision：MEMORY.md 注入方式**
  - 问题：TAD §9.4.1 定义的 MEMORY.md 为 Agent 上下文声明格式，需确保该上下文声明能被 NanoBot 正确加载。
  - 决策：根据实际代码核实，NanoBot `ContextBuilder` **原生支持直接读取 `memory/MEMORY.md`**，若其内容非空且不为默认模板，会通过 `# Memory` 块自动注入到系统提示词中。本提案提供 `MEMORY.md.example` 格式样板，指引运维将其直接放置于 NanoBot Workspace 目录的 `memory/MEMORY.md` 路径下，无需通过 `config.json` 手动注入（移除了之前的 RISK-007）。本提案不实现解析逻辑，仅依赖 NanoBot 原生官方能力加载。
  - Alternatives considered：在 Gateway 启动时读取 MEMORY.md 并传递给 Runtime → 违反 criterion.md §3.4（Gateway MUST NOT 做 MCP 选择）；通过 `config.json` 手动注入 → 虽然可行，但不符合 NanoBot 的原生上下文构建最佳实践。

- **Decision：多店铺 mcpServers 命名格式**
  - 格式：`shopify-{store-slug}`，与 criterion.md §3.6 MCP_Instances 命名规范（`shopify-{store-slug}`）一致
  - 每个店铺一个具名条目，凭证变量前缀为 `SHOPIFY_{STORE_SLUG_UPPER}_`

- **Decision：不涉及新错误码**
  - `MCP_TIMEOUT` / `MCP_UNAVAILABLE` 已在 criterion.md §5.4 定义，由 `feat-runtime-nanobot-adapter` 处理；本提案不新增错误码。
  - SSoT 未更改（本提案不涉及 `SSoT/schema/migrations/` 或 `SSoT/api/main.tsp` 变更）。

## Risks / Trade-offs

- ~~**RISK-007（工具链差距）**：已解决。已确认 NanoBot 支持 `memory/MEMORY.md` 文件加载并注入。~~
- **RISK-003（MCP 可用性）**：shopify-mcp 子进程崩溃时 NanoBot 的降级行为需联调验证；Docker `restart: unless-stopped` 保障容器级恢复，但子进程退出后 NanoBot 内部重试机制不明确
- **Trade-off**：选择将 `toolTimeout: 10` 写入 `config.json` 而非 `MEMORY.md`，因为 timeout 是系统级约束，不应由运维按环境自定义；MEMORY.md 专注于实例声明和工具允许列表

## Migration Plan

N/A — 全新配置文件，无数据迁移。运维操作：
1. 将 `config.json.example` 复制为 `./nanobot-data/config.json`，填入真实凭证
2. 将 `.env.example` 复制为 `nanobot/.env`，填入真实 secret
3. 将 `MEMORY.md.example` 复制为 `./nanobot-data/memory/MEMORY.md`，按实际店铺填写
4. 确认 `config.json`、`.env` 已在 `.gitignore` 中

## Open Questions

1. ~~**HKUDS/nanobot MEMORY.md 加载**：当前版本 NanoBot 读取 MEMORY.md 的实际机制是否为 `config.json → agents.defaultAgent → instructions`？或有其他官方支持方式？（影响：MEMORY.md.example 的注入说明）→ 实施时以官方文档验证~~（已在设计阶段解决：NanoBot 原生支持加载 Workspace 的 `memory/MEMORY.md` 文件）
2. **shopify-mcp 子进程崩溃恢复**：NanoBot 是否会自动重启崩溃的 MCP 子进程？若不自动重启，运维告警策略是什么？→ 联调时验证
