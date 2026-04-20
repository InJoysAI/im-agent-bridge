# audit-compliance Specification

## Purpose
TBD

## Requirements
### Requirement: 路线图提案依赖声明完整性
路线图中每个提案的「依赖」（前置/被依赖）字段必须（MUST）准确反映实际的构建顺序与数据流依赖关系。

#### Scenario: E2E 集成测试依赖链完整
- **WHEN** 审查 `feat-e2e-integration-test` 提案的前置依赖字段
- **THEN** 前置列表包含 `feat-observability-metrics` 和 `feat-persist-runtime-logs`
- **AND** 依赖关系与实际构建顺序一致

#### Scenario: NanoBotAdapter 前置依赖补全
- **WHEN** 审查 `feat-runtime-nanobot-adapter` 提案的前置依赖字段
- **THEN** 前置列表包含 `feat-gateway-channel-session`
- **AND** 反映 session_id / bot_id 数据流依赖

---

### Requirement: 提案验收标准与业务规则对齐
每个提案的验收标准必须（MUST）覆盖其范围内涉及的所有 MUST 级别业务规则（BR-xxx），不得遗漏。

#### Scenario: DB 层提案含 BR-070 验收项
- **WHEN** 审查 `feat-gateway-db-layer` 提案的关键任务和验收表
- **THEN** 存在「message_events INSERT 前对 input_text/output_text 截断至 512 字符（BR-070）」的显式任务
- **AND** 验收表包含对应验收行

#### Scenario: E2E 提案含 BR 全量覆盖矩阵
- **WHEN** 审查 `feat-e2e-integration-test` 提案的验收节
- **THEN** 存在 BR 全量覆盖矩阵验收项
- **AND** 矩阵以显式 BR-xxx → 提案映射表嵌入验收节（自包含，不依赖外部文档）

#### Scenario: Mention Filter 提案含 RISK-B007 登记任务
- **WHEN** 审查 `feat-gateway-mention-filter` 提案的验收表
- **THEN** 关键任务含 Task 0（归档前登记 RISK-B007）
- **AND** `telegram_username` 确认为低敏感（与 bot_name 同级），无需额外脱敏断言

---

### Requirement: 技术债务消减路径追踪
`.context/architecture/risks_and_debt.md` 中登记的技术债务必须（MUST）标注消减路径（指向后续提案或明确的解决方案）。

#### Scenario: TD-005 数据保留清理消减路径按表细化
- **WHEN** 查阅 `.context/architecture/risks_and_debt.md` 中 TD-005 条目
- **THEN** 消减路径按表细化：`runtime_logs` 14 天清理由 `feat-runtime-log-retention` 承接（done）；`message_events` 30 天清理由 `feat-message-event-retention` 承接（done）；`sessions` 清理由独立后续提案 `feat-session-cleanup` 承接（pending）

#### Scenario: TD-007 Bridge TLS 升级消减路径
- **WHEN** 查阅 `.context/architecture/risks_and_debt.md` 中 TD-007 条目
- **THEN** 存在消减路径说明，指向后续独立提案 `feat-bridge-tls-upgrade`

---

### Requirement: 业务风险登记完整性
新增功能引入的业务风险必须（MUST）在 `.context/domain/risks_and_debt.md` 中登记。

#### Scenario: RISK-B007 群聊 mention 过滤风险已登记
- **WHEN** 查阅 `.context/domain/risks_and_debt.md`
- **THEN** 存在 RISK-B007 条目（群聊 mention 过滤导致漏响应）
- **AND** 包含缓解措施和应急预案

---

### Requirement: 项目配置与架构文档一致性
`openspec/config.yaml` 中的可观测性指标列表必须（MUST）与架构文档定义的指标清单一致。

#### Scenario: Counter 列表同步为 11 项
- **WHEN** 查阅 `openspec/config.yaml` 的 `Architecture Patterns` 节 Counter 列表
- **THEN** 列表包含 11 个指标（含 `runtime_log_write_failures_total`）
- **AND** 与 `.context/architecture/cross_cutting_concepts.md` TAD §11.2 定义一致
- **AND** `cross_cutting_concepts.md` AI 引用指南中 Counter 数量描述为「11 个」

---

### Requirement: 提案 Gate 场景覆盖完整性
涉及 Runtime 错误处理的提案必须（MUST）包含关键错误场景的 Gherkin 验收用例。

#### Scenario: NanoBotAdapter 含 SESSION_NOT_FOUND Gherkin
- **WHEN** 审查 `feat-runtime-nanobot-adapter` 提案的 Gate 场景
- **THEN** 存在 RUNTIME_SESSION_NOT_FOUND 处置场景
- **AND** 场景描述清空 runtime_session_key 并重建的行为

#### Scenario: NanoBotAdapter 含 Phase 2/3 时序约束说明
- **WHEN** 审查 `feat-runtime-nanobot-adapter` 提案的验收节
- **THEN** 存在 Phase 2/3 端到端验收时序约束备注

---

### Requirement: 路线图索引一致性
路线图索引表行数必须（MUST）与实际提案数量一致，不得遗漏。

#### Scenario: 索引表包含全部 18 个提案
- **WHEN** 查阅 `openspec/proposal-roadmap.md` 的提案索引表
- **THEN** 行数 = 18
- **AND** 包含 `feat-runtime-log-retention`（#16）
