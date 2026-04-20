# Change: 端到端集成验证 (feat-e2e-integration-test)

## Why

IM Agent Bridge MVP 所有核心功能模块（消息接入、Bot ID 解析、会话管理、Runtime 调用、回写、幂等、限流、DB 熔断）已通过各自的单元/集成提案交付，但尚未进行完整主链路的联合验证。在进入生产部署前，必须以"真实环境中的一键联调"来验证：

1. **主链路闭环**：Telegram 文本消息 → Matterbridge → Gateway → NanoBot → Shopify MCP → 回写到原会话（Telegram 收到 AI 回复）
2. **P0 BDD 场景全覆盖**：`testing_strategy.md` 所定义的模块 1–8 全部关键场景得到验证（文本接入、bot_id 解析、幂等去重、限流 429、DB 熔断 503、Runtime 超时回写等）
3. **异常注入验证**：在受控环境中停止 NanoBot / PostgreSQL，验证系统错误处理和用户提示路径

此提案是 Phase 4 的收口 Gate，所有前置提案（`feat-runtime-reply-bridge`, `feat-nanobot-shopify-mcp`, `feat-observability-logging`, `feat-infra-matterbridge-deploy`）必须归档完毕后方可执行本提案。

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | openspec/proposal-roadmap.md |
| roadmap_source_supplement | N/A |
| phase | Phase 4 |
| business_goal | 验证完整主链路闭环；覆盖 P0 BDD 场景（模块 1–8）；验证关键异常场景（Runtime 超时、DB 不可用、限流触发） |
| dependencies | 前置: feat-runtime-reply-bridge, feat-nanobot-shopify-mcp, feat-observability-logging, feat-infra-matterbridge-deploy |
| acceptance_criteria | 主链路文本消息闭环；幂等 ignored_duplicate；DB 熔断 503；限流 429；Runtime 超时错误提示；specflow 全归档门禁 |

## What Changes

### 新增功能
- Docker Compose 一键启动验证流程（全服务 health check 通过确认）
- 主链路 BDD P0 场景手动执行清单（覆盖 `testing_strategy.md` 模块 1–8 共 20+ 场景）
- 异常注入测试流程（停止 NanoBot → 15s 超时 + 错误提示；停止 PG → 503 熔断）
- P95 响应时间验证（20 条消息采样，端到端 ≤ 5s 目标）
- 可观测性验证（trace_id 全链路贯通 + 10 个 Counter 指标可查询）

### 修改功能
- 无破坏性变更（Out of scope）

### 技术实现
- MVP 采用手动联调 + 简单 shell 脚本（非自动化 E2E 框架；自动化 E2E 框架为 Out of scope）
- 全部验证步骤均在 Docker Compose 环境（Internal Server 拓扑）执行
- specflow gate：本提案执行前，确认所有前置 changes 已 archive

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/e2e-integration/spec.md` — 端到端集成验证行为规范（主链路闭环、P0 BDD 场景、异常注入场景、可观测性验证、性能门禁）

### 涉及的代码
- **新增**：
  - `openspec/changes/feat-e2e-integration-test/` — 提案目录
  - 可选：`scripts/e2e/` — 联调辅助脚本（停止容器、计时脚本等，按需创建）
- **修改**：无

### 依赖关系
- **依赖（前置，必须归档完毕）**：
  - `feat-runtime-reply-bridge`（主链路回写能力）
  - `feat-nanobot-shopify-mcp`（Runtime + MCP 部署）
  - `feat-observability-logging`（日志可观测，辅助联调定位）
  - `feat-infra-matterbridge-deploy`（Matterbridge Edge Server 部署）
- **被依赖**：无（Phase 4 收口，无下游提案）

### 风险与注意事项
- 前置提案未完成归档时，本提案联调无实际意义，`specflow validate` 会失败
- NanoBot 容器状态依赖 `MEMORY.md` / `.env` 配置正确（TAD §9.4.1）；联调前需确认 Runtime 部署产物就绪
- Shopify MCP 可用性依赖外部 SaaS（RISK-003）；建议使用测试店铺数据规避生产影响
- P95 ≤ 5s 目标在 NanoBot 调用 Shopify MCP 的实际网络延迟下存在裕量风险；若超标需记录测量数据并作风险接受决策
- 手动联调无法覆盖长期并发稳定性（Post-MVP 回归范围）

### 验证标准
- ✅ `docker compose up -d` 后全服务 health check 通过（Gateway / NanoBot / PostgreSQL / Matterbridge）
- ✅ 文本消息 → Telegram 收到 AI 回复（端到端主链路闭环）
- ✅ 重复消息 → Gateway 返回 HTTP 200 + `{"status":"ignored_duplicate"}`，且 `message_events` 无新增行（BR-042 MUST NOT 重复写入）
- ✅ DB 停止 → Gateway 返回 HTTP 503（熔断验证，BR-041）
- ✅ 同一 `chat_id` 第 6 条/秒 → HTTP 429（限流验证，BR-055）
- ✅ NanoBot 停止 → Gateway 15s 超时后回写错误提示（RUNTIME_TIMEOUT，BR-051）
- ✅ 端到端 P95 响应时间 ≤ 5s（`criterion.md §8`）
- ✅ 20 条采样：接入成功率 ≥ 95%、回写成功率 ≥ 95%、MCP 调用成功率 ≥ 90%（`testing_strategy.md §DoD`）
- ✅ 全部前置提案（4 项直接依赖）已完成 `specflow validate + archive` 归档；`openspec/changes/archive/` 覆盖路线图实施清单（criterion.md §6 门禁）

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §8 性能超时约束（P95 ≤ 5s）；§3 三层架构边界验证；§5 接口契约（幂等键、限流、熔断）；§4 安全约束（Bearer Token、日志脱敏） |
| domain | `.context/domain/testing_strategy.md` | P0 BDD 场景权威来源（模块 1–8）；测试金字塔（E2E 关键路径 100%）；DoD 定义 |
| domain | `.context/domain/business_rules.md` | BR-041（DB 熔断）、BR-055（限流）、BR-042（幂等）、BR-062（回写重试）、BR-051/052（超时） |
| domain | `.context/domain/edge_cases.md` | 异常注入场景依据（PG 不可用、Runtime 超时） |
| architecture | `.context/architecture/runtime_view.md` | 主链路数据流验证依据（Gateway → RuntimeAdapter → NanoBot → 回写链路） |
| architecture | `.context/architecture/cross_cutting_concepts.md` | 可观测性验证（trace_id 全链路、10 个 Counter 指标） |
| architecture | `.context/architecture/deployment_view.md` | Docker Compose 拓扑验证依据（Internal Server / Edge Server 拓扑） |
