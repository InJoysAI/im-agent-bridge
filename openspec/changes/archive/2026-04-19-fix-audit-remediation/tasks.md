# 实施任务清单

> 所有修订均为文档/配置性修改，无核心业务逻辑变更。修订对象为 `openspec/proposal-roadmap.md` 中的提案条目、`openspec/config.yaml`、`.context/architecture/risks_and_debt.md` 和 `.context/domain/risks_and_debt.md`。

## 1. 路线图提案条目修订

- [x] 1.1 **[P-02]** `feat-gateway-db-layer` 提案修订
  - 关键任务节新增「message_events INSERT 前对 input_text/output_text 截断至 512 字符（BR-070）」
  - 验收表新增对应验收行
- [x] 1.2 **[P-03]** `feat-e2e-integration-test` 提案修订
  - `依赖` 字段 `前置` 追加 `feat-observability-metrics`, `feat-persist-runtime-logs`
- [x] 1.3 **[P-04]** `feat-gateway-mention-filter` 提案修订
  - 关键任务新增 Task 0（归档前在 `domain/risks_and_debt.md` 登记 RISK-B007）
  - `telegram_username` 确认为低敏感（与 bot_name 同级），无需额外脱敏断言行
- [x] 1.4 **[P-08 + P-09 + P-12]** `feat-runtime-nanobot-adapter` 提案修订
  - `依赖` 前置字段补充 `feat-gateway-channel-session`
  - 补充 RUNTIME_SESSION_NOT_FOUND Gherkin 场景
  - 验收注备注 Phase 2/3 时序约束
- [x] 1.5 **[P-13]** `feat-nanobot-shopify-mcp` 提案修订
  - 删除「可拆解提示」注释块（决策维持单提案）

## 2. 配置与 Context 资产修订

- [x] 2.1 **[P-10]** `openspec/config.yaml` 修订
  - `Architecture Patterns` 节 Counter 列表更新为 11 项（补充 `runtime_log_write_failures_total`）
- [x] 2.1b **[P-10 配套]** `.context/architecture/cross_cutting_concepts.md` 修订
  - AI 引用指南中 Counter 数量由「10 个」更正为「11 个」（含 `runtime_log_write_failures_total`）
- [x] 2.2 **[P-05 + P-06]** `.context/architecture/risks_and_debt.md` 修订
  - TD-005（数据保留清理依赖手动）：细化消减路径——`runtime_logs` 14 天清理由 `feat-runtime-log-retention` 承接（done）；`message_events` 30 天清理由 `feat-message-event-retention` 承接（done）；`sessions` 清理由独立后续提案 `feat-session-cleanup` 承接（pending）
  - TD-007（Bridge↔Gateway 无 TLS）：补充消减路径 → 独立后续提案 `feat-bridge-tls-upgrade`
- [x] 2.3 **[P-04 前置]** `.context/domain/risks_and_debt.md` 验证
  - 验证 RISK-B007 条目已存在（由 `feat-gateway-mention-filter` 归档前登记）
  - 若不存在则补充新增

## 3. E2E 验收补充

- [x] 3.1 **[BR 盲区]** `feat-e2e-integration-test` 验收节修订
  - 新增「BR 全量覆盖矩阵」验收项
  - 以显式 BR-xxx → 提案映射表嵌入验收节（自包含，不依赖外部文档）

## 4. 验证

- [x] 4.1 逐项核查验收标准
  - [x] 4.1.1 P-02 BR-070 显式任务存在
  - [x] 4.1.2 P-03 依赖链完整
  - [x] 4.1.3 P-04 RISK-B007 已登记 + Task 0 存在
  - [x] 4.1.4 P-08 依赖 + P-09 时序 + P-12 Gherkin 补全
  - [x] 4.1.5 P-10 Counter = 11（config.yaml + cross_cutting_concepts.md 口径一致）
  - [x] 4.1.6 P-11 `telegram_username` 低敏感分类确认（无需脱敏断言行）
  - [x] 4.1.7 P-05/P-06 TD-005 消减路径已按表细化 + TD-007 消减路径已标注
  - [x] 4.1.8 P-13 拆解提示已删除
  - [x] 4.1.9 索引行数 = 18
  - [x] 4.1.10 BR 盲区验收项存在

- [x] 4.2 SSoT 检查
  - SSoT 未更改（本提案不涉及 schema/API 变更）

- [x] 4.3 不涉及新错误码

- [x] 4.4 风险与注意事项闭环核查（proposal.md §风险与注意事项）
  - [x] 4.4.1 仅文档/配置修改，无业务逻辑代码变更（以 `git status --short` 文件范围核查）
  - [x] 4.4.2 TD-007 与 TD-005 sessions 子项仅登记消减路径，不触发实施（以 `risks_and_debt.md` 文案核查）
  - [x] 4.4.3 P-07 仅记录为设计指导，不拆分已 done 提案（以 `proposal-roadmap.md` 文案核查）

## 5. Specflow 门禁

- [x] 5.1 specflow validate: `node design/context-dev/tools/specflow/specflow.mjs validate fix-audit-remediation --strict`
- [ ] 5.2 specflow archive: `node design/context-dev/tools/specflow/specflow.mjs archive fix-audit-remediation --yes`

## 6. 执行证据归档

- [x] 6.1 写入 specflow validate 执行证据（以 `openspec/changes/fix-audit-remediation/evidence.md` §1 原始输出摘录为准）
- [x] 6.2 写入测试执行证据（以 `openspec/changes/fix-audit-remediation/evidence.md` §1 原始输出摘录为准）
- [x] 6.3 生成证据索引：`openspec/changes/fix-audit-remediation/evidence.md`（验收项 -> 命令 -> 结果）
