# Change: 路线图审查缺口修复

## Why

2026-04-18 全量路线图审查（审查缺口编号 P-02 至 P-13，详见本提案 `openspec/changes/fix-audit-remediation/proposal.md` 各修订条目的 `[P-xx]` 编号标注）发现 12 项规范合规缺口，涉及提案依赖声明不一致、验收标准缺失、技术债未追踪等问题。这些缺口虽不影响已归档提案的运行时行为（均已 done），但会导致后续提案引用和审计时产生混淆。本提案系统性修复这些文档/配置层面的合规空白，确保路线图审查通过。

## What Changes

### 修改功能
- **[P-02]** `feat-gateway-db-layer` 路线图条目：补充 BR-070 显式任务（`message_events` 512 字截断）+ 验收项
- **[P-03]** `feat-e2e-integration-test` 路线图条目：`前置` 字段补充 `feat-observability-metrics`, `feat-persist-runtime-logs`
- **[P-04]** `feat-gateway-mention-filter` 路线图条目：关键任务新增 Task 0（归档前登记 RISK-B007）
- **[P-08 + P-09 + P-12]** `feat-runtime-nanobot-adapter` 路线图条目：`依赖` 前置字段补充 `feat-gateway-channel-session`；补充 RUNTIME_SESSION_NOT_FOUND Gherkin 场景；验收注备注 Phase 2/3 时序约束
- **[P-10]** `openspec/config.yaml`：`Architecture Patterns` 节 Counter 列表从 10 项更新为 11 项（补充 `runtime_log_write_failures_total`）
- **[P-13]** `feat-nanobot-shopify-mcp` 路线图条目：删除「可拆解提示」注释块
- **[P-05 + P-06]** `.context/architecture/risks_and_debt.md`：更新 TD-005（数据保留清理依赖手动）消减路径——按表细化为 `runtime_logs`（done, feat-runtime-log-retention）、`message_events`（done, feat-message-event-retention）、`sessions`（pending, feat-session-cleanup）；更新 TD-007（Bridge↔Gateway TLS 升级）消减路径
- **[P-04 前置]** `.context/domain/risks_and_debt.md`：新增 RISK-B007 条目（已在前序提案 `feat-gateway-mention-filter` 中完成，本提案验证其存在）
- **[BR 盲区]** `feat-e2e-integration-test` 验收节新增显式 BR 覆盖矩阵（逐条列出 BR-xxx 与提案映射关系，替代不可复核的外部报告引用）
- **[P-10 配套]** `.context/architecture/cross_cutting_concepts.md`：AI 引用指南第 2 条 Counter 数量从 10 修正为 11（与 TAD §11.2 指标表对齐）

### 技术实现
- 所有修订均为文档/配置性修改，无核心业务逻辑变更
- 不涉及 SSoT schema/API 变更
- 不涉及新错误码
- 不影响已归档提案的 archive 状态

## Impact

### 涉及的规范（Specs）
- **修改**：`specs/audit-compliance/spec.md` - 路线图文档合规性验证

### 涉及的代码
- **修改**：
  - `openspec/proposal-roadmap.md`（提案 #3, #7, #10, #14, #15 条目修订）
  - `openspec/config.yaml`（Counter 列表更新）
  - `.context/architecture/risks_and_debt.md`（TD-005/TD-007 消减路径）

### 依赖关系
- **依赖**：`feat-message-event-retention`（#17, done）、`feat-gateway-mention-filter`（#15, done）
- **被依赖**：无

### 风险与注意事项
- 所有修订为文档/配置修改，无代码逻辑变更，变更风险极低
- TD-007（TLS 升级）和 TD-005 sessions 子项仅登记消减路径，不触发实施
- `feat-gateway-inbound-gate`（P-07）原子性问题记录为设计指导，不强制拆分已 done 的提案

### 交付风险

| 风险ID | 触发条件 | 影响 | 缓解方案 | 责任归属 |
|--------|---------|------|---------|---------|
| D-R1 | BR 覆盖矩阵引用不可复核的外部文档 | 审计/评审无法逐条追溯 | 改为显式 BR 矩阵嵌入 E2E 验收节，逐条列出 BR-xxx 与提案映射 | 提案作者 |
| D-R2 | .context 内 Counter 数量口径冲突（cross_cutting_concepts.md AI 引用指南 "10" vs TAD §11.2 表 "11"） | P-10 验收争议 | 本提案同步修正 AI 引用指南为 11 | 提案作者 |
| D-R3 | P-11 原要求 telegram_username 日志脱敏，但用户决策为低敏感（与 bot_name 同级） | 原验收项与安全口径冲突 | 移除 P-11 脱敏断言行，telegram_username 按低敏感处理 | 提案作者 |

### 验证标准
- ✅ P-02：`feat-gateway-db-layer` 提案含「512 字截断」显式任务 + 验收项
- ✅ P-03：`feat-e2e-integration-test` 前置列表含 `feat-observability-metrics` 和 `feat-persist-runtime-logs`
- ✅ P-04：`domain/risks_and_debt.md` 存在 RISK-B007 条目；`feat-gateway-mention-filter` 关键任务含 Task 0
- ✅ P-08：`feat-runtime-nanobot-adapter` 前置含 `feat-gateway-channel-session`
- ✅ P-09：`feat-runtime-nanobot-adapter` 验收节含 Phase 2/3 时序约束说明
- ✅ P-10：`config.yaml` Counter 列表为 11 项（含 `runtime_log_write_failures_total`）；`.context/architecture/cross_cutting_concepts.md` AI 引用指南 Counter 数量 = 11
- ✅ P-12：`feat-runtime-nanobot-adapter` 含 RUNTIME_SESSION_NOT_FOUND Gherkin 场景
- ✅ P-05/P-06：`architecture/risks_and_debt.md` 中 TD-005 消减路径已按表细化（runtime_logs/message_events/sessions）、TD-007 已标注消减路径
- ✅ 索引一致：路线图索引行数 = 18（含 `feat-runtime-log-retention` #16）
- ✅ BR 盲区：`feat-e2e-integration-test` 验收节含显式 BR 覆盖矩阵（逐条可复核）
- ✅ `node design/context-dev/tools/specflow/specflow.mjs validate fix-audit-remediation --strict` 通过

### 关联 Context 资产
| Scope | 资产路径 | 关联说明 |
|-------|---------|----------|
| criterion | `.context/criterion.md` | §6 变更工作流门禁（specflow validate/archive） |
| domain | `.context/domain/business_rules.md` | BR-070 消息数据最小化（P-02 修订依据） |
| architecture | `.context/architecture/risks_and_debt.md` | TD-005（数据保留清理）消减路径细化 + TD-007 消减路径登记目标 |
| architecture | `.context/architecture/cross_cutting_concepts.md` | TAD §11.2 指标清单权威表（P-10 Counter 数量对齐依据） |
| architecture | `.context/architecture/security_policy.md` | 数据分类/保留期/敏感数据处理（BR-070/512 截断权威来源） |
| domain | `.context/domain/risks_and_debt.md` | RISK-B007 登记验证目标 |
| domain | `.context/domain/edge_cases.md` | 群聊 mention 过滤边界口径（ignored_no_mention 处理规则） |
| openspec | `openspec/config.yaml` | Counter 列表同步修正目标 |
| openspec | `openspec/proposal-roadmap.md` | 提案条目修订主文件 |

### 提案大纲对齐（Roadmap Alignment）
| 字段 | 内容 |
|------|------|
| roadmap_source_primary | openspec/proposal-roadmap.md |
| roadmap_source_supplement | N/A |
| phase | Phase 6 |
| business_goal | 系统性修复审查报告缺口，修正依赖声明不一致、验收缺口、技术债未追踪 |
| dependencies | feat-message-event-retention (#17, done), feat-gateway-mention-filter (#15, done) |
| acceptance_criteria | P-02 至 P-13 全部验收项通过 + specflow validate --strict 通过 |
