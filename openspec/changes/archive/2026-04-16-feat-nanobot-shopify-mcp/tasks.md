# 实施任务清单：feat-nanobot-shopify-mcp

> Roadmap 对齐：Phase 3，提案 #10，预计 1 天（0.25d/步）
> 前置条件：feat-nanobot-deploy (#9) + feat-runtime-nanobot-adapter (#7) 已完成，NanoBot 容器正常运行

## 1. SSoT 检查（SSoT 未更改）

- [x] 1.1 确认 `SSoT/schema/migrations/` 无需变更（本提案不涉及 DB Schema 变更，PostgreSQL MUST NOT 存储 MCP 配置）
- [x] 1.2 确认 `SSoT/api/main.tsp` 无需变更（本提案无新增 API 端点；Gateway ↔ NanoBot 接口由 feat-runtime-nanobot-adapter 定义）
- [x] 1.3 记录"SSoT 未更改"结论（在代码 Review 中注明）

## 2. config.json.example — 多店铺 mcpServers 模板

- [x] 2.1 编写 `deploy/internal-server/nanobot/config.json.example`（`tools.mcpServers` 多店铺模板）
  - 具名条目格式：`shopify-{store-slug}`（如 `shopify-store1`、`shopify-store2`）
  - 每个条目 `command: "npx"`，`args: ["shopify-mcp", "--clientId", "${SHOPIFY_STORE1_CLIENT_ID}", "--clientSecret", "${SHOPIFY_STORE1_CLIENT_SECRET}", "--domain", "store1.myshopify.com"]`
  - 每个条目 `toolTimeout: 10`（BR-052；MCP 调用 ≤ 10s）
  - 保留 `providers`（LLM 配置）节（来自 feat-nanobot-deploy）
- [x] 2.2 编写 `deploy/internal-server/nanobot/.env.example`（追加 Shopify 凭证变量）
  - 变量命名：`SHOPIFY_{STORE_SLUG_UPPER}_CLIENT_ID`、`SHOPIFY_{STORE_SLUG_UPPER}_CLIENT_SECRET`、`SHOPIFY_{STORE_SLUG_UPPER}_DOMAIN`
  - 多店铺按格式追加（含示例注释：如何增加第 N 个店铺）

## 3. MEMORY.md 格式样板

- [x] 3.1 编写 `deploy/internal-server/nanobot/memory/MEMORY.md.example`（TAD §9.4.1）
  - 按格式声明多店铺 MCP Server 列表（MCP Server 名称/显示名称/品类/地区/币种/时区/备注）
  - 说明工具允许列表配置方式（BR-034，RISK-B003）：若发现不安全工具，在 MEMORY.md 中限制允许工具列表
  - 注释说明 MEMORY.md 的放置方式（放入 `memory/MEMORY.md` 即可由 NanoBot 原生支持加载，参考 design.md Decision 1）
- [x] 3.2 确认 `config.json`、`.env` 已在 `deploy/internal-server/nanobot/.gitignore`（或根目录 `.gitignore`）中

## 4. 验证

- [x] 4.1 验证凭证注入：将实际 config.json（含真实凭证）写入 `./nanobot-data/`，重启 NanoBot 容器，确认 shopify-mcp 子进程正常启动（`docker compose logs nanobot` 中无子进程报错）
- [x] 4.2 验证 MCP 超时（BR-052）：模拟 MCP 子进程响应延迟 > 10s，确认 NanoBot 返回降级提示，Gateway 在 15s hard timeout 内收到响应（不阻塞）
- [x] 4.3 验证凭证隔离（MUST NOT）：运行 `SELECT table_name FROM information_schema.tables WHERE table_name LIKE 'mcp_%'`，确认返回空结果
- [x] 4.4 验证 MCP 工具调用审计日志（BR-034）：调用 shopify-mcp 工具，检查 NanoBot stdout/日志文件中有可见的工具调用记录，PostgreSQL 无新增记录
- [x] 4.5 验证 BR-034 工具限制：确认 MEMORY.md 工具允许列表声明正确，超出列表的工具不可被调用（如有可通过 NanoBot 调试模式验证）
- [x] 4.6 验证审计日志不含敏感凭证：检查 NanoBot 日志输出，确认无 `clientId`、`clientSecret` 等明文
- [x] 4.7 验证配置缺失场景：故意移除 `.env` 中的 Shopify 凭证并启动 NanoBot，确认系统会有明确的告警且不影响主服务拉起，但调用相关工具时返回错误。
- [x] 4.8 验证 MEMORY.md 加载证据：检索 NanoBot 启动日志（或调用调试接口），确认有类似于 `Loaded memory/MEMORY.md` 注入成功的明确证据
- [x] 4.9 验证子进程崩溃恢复（RISK-001/003）：手动 `kill` 掉容器内的 `shopify-mcp` 子进程，观察 NanoBot 是否输出告警日志并自动重启该子进程。若无法自动重启，须确认对应的健康检查和外置告警机制被正确触发
## 5. 文档

- [x] 5.1 在 `deploy/internal-server/nanobot/README.md`（如存在）中补充"如何增加新店铺"操作说明
- [x] 5.2 确认"不涉及接口设计"：本提案无新增 API，`.context/architecture/api_strategy.md` 无需更新
- [x] 5.3 确认"不涉及新错误码"：MCP_TIMEOUT/MCP_UNAVAILABLE 已在 criterion.md §5.4 定义，不触发 errcode SSoT 流程

## 6. 验证与归档

- [x] 6.1 运行 specflow validate：`node design/context-dev/tools/specflow/specflow.mjs validate feat-nanobot-shopify-mcp --strict`
- [x] 6.2 修复所有验证错误（若有），重新验证直至通过
- [x] 6.3 运行 specflow archive：`node design/context-dev/tools/specflow/specflow.mjs archive feat-nanobot-shopify-mcp --yes`
