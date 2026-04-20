# 执行证据索引（fix-audit-remediation）

执行日期：2026-04-19

## 1) 门禁与测试证据

- Specflow 严格校验
  - 命令：`node design/context-dev/tools/specflow/specflow.mjs validate fix-audit-remediation --strict`
  - 结果：`✅ OK`
  - 原始输出摘录：
    ```text
    === Specflow Validate: fix-audit-remediation (strict) ===
    ✅ OK
    ```
  - 证据文件：`openspec/changes/fix-audit-remediation/evidence/specflow_validate.log`

- Gateway 测试
  - 命令：`cd gateway && cargo test`
  - 结果：`test result: ok. 78 passed; 0 failed; 14 ignored`
  - 原始输出摘录：
    ```text
    test result: ok. 78 passed; 0 failed; 14 ignored; 0 measured; 0 filtered out; finished in 30.03s

    running 7 tests
    ... all ignored (requires DATABASE_URL and integration environment) ...
    test result: ok. 0 passed; 0 failed; 7 ignored; 0 measured; 0 filtered out
    ```
  - 说明：集成测试 `tests/inbound_mention_filter.rs` 7 项因缺少 `DATABASE_URL` 环境被 `ignored`，不影响本提案的文档/配置修订闭环。
  - 证据文件：`openspec/changes/fix-audit-remediation/evidence/cargo_test.log`

## 2) 验收项可复核证据（命令 -> 命中）

- P-02（BR-070 显式任务 + 验收项）
  - 命令：`rg -n "message_events.*input_text|BR-070 数据最小化" openspec/proposal-roadmap.md`
  - 结果：命中关键任务行与验收表行。

- P-03（E2E 前置依赖补全）
  - 命令：`rg -n "feat-observability-metrics|feat-persist-runtime-logs" openspec/proposal-roadmap.md`
  - 结果：命中提案 14 前置依赖。

- P-04（Task 0 + 低敏感分类）
  - 命令：`rg -n "Task 0（归档前门禁）|低敏感分类" openspec/proposal-roadmap.md`
  - 结果：命中提案 16 关键任务与验收表。

- P-08/P-09/P-12（依赖 + Gherkin + 时序）
  - 命令：`rg -n "feat-gateway-channel-session|Runtime 会话失效自动重建|时序约束说明（Phase 2/3）" openspec/proposal-roadmap.md`
  - 结果：命中提案 7 依赖、Gherkin 新场景、时序说明。

- P-10（Counter=11 口径一致）
  - 命令：`rg -n "全部 11 个 Counter|runtime_log_write_failures_total" openspec/config.yaml .context/architecture/cross_cutting_concepts.md`
  - 结果：`config.yaml` 与 `cross_cutting_concepts.md` 均命中 11 项口径。

- P-05/P-06（TD-005/TD-007 消减路径）
  - 命令：`rg -n "feat-session-cleanup|feat-bridge-tls-upgrade" .context/architecture/risks_and_debt.md`
  - 结果：命中 TD-005 的 sessions 承接路径与 TD-007 的独立提案路径。

- BR 盲区（显式 BR 矩阵）
  - 命令：`rg -n "BR 全量覆盖矩阵（显式可复核）" openspec/proposal-roadmap.md`
  - 结果：命中提案 14 的 BR 全量覆盖矩阵章节。

## 3) 风险与注意事项闭环证据

- 风险 1：仅文档/配置修改，无业务逻辑变更
  - 命令：`git status --short`
  - 结果：仅出现本次提案文档与证据目录变更。

- 风险 2：TD-007 与 TD-005 sessions 子项仅登记消减路径，不触发实施
  - 命令：`rg -n "feat-session-cleanup|feat-bridge-tls-upgrade" .context/architecture/risks_and_debt.md`
  - 结果：均为“后续提案承接”表述，无实施项。

- 风险 3：P-07 仅记录设计指导，不拆分已 done 提案
  - 命令：`rg -n "P-07|原子性问题记录为设计指导" openspec/changes/fix-audit-remediation/proposal.md openspec/proposal-roadmap.md`
  - 结果：命中“记录为设计指导，不强制拆分”描述。

## 4) 归档状态

- `specflow archive` 当前保持未执行（待用户核查通过后执行）：
  - 目标命令：`node design/context-dev/tools/specflow/specflow.mjs archive fix-audit-remediation --yes`
