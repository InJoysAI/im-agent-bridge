# Change: 多 Bot @mention 过滤（Gateway 侧 + DB 配置）

## Why

联调过程中发现默认行为是"群聊全量响应"——群内每条闲聊都会穿过主链路进入 Runtime / NanoBot，消耗 LLM token 且扰乱业务上下文。评估了三条路线后，选定 **Gateway 侧 + DB 配置** 方案（与 BR-032 "所有 bot 相关配置走 DB" 对齐，零 LLM 成本、粒度到单 bot、多租户天然隔离）。

业务目标：

- **群聊降噪**：仅当用户显式 `@{bot_username}` 时才进入 Runtime；其余群聊消息在 Gateway 入口层直接以 HTTP 200 + `{"status":"ignored_no_mention"}` 丢弃，不写 `message_events`、不调 Runtime。
- **多租户安全**：每个 `bots` 记录独立配置 `telegram_username` 与 `require_mention`，与 `channel_bindings.bot_id` 解析路径自然贯通（BR-032）。
- **私聊不受限**：1:1 对话始终直响，不触发过滤。
- **零停机升级**：默认 `require_mention=false`，存量 bot 保持当前全量响应行为；开启需运营显式 `UPDATE`。

备选方案已否决：Matterbridge `MessageMatchRegex`（粒度粗、无 DB 多租户灵活性）、NanoBot 系统提示词 `<SKIP>`（依赖 LLM 推理、仍消耗 token）、env 变量 `TELEGRAM_MENTION_FILTER`（与 BR-032 冲突）。

## What Changes

### 新增功能

- **`bots` 表扩展**：新增列 `telegram_username TEXT NULL`、`require_mention BOOLEAN NOT NULL DEFAULT FALSE`（Expand-Contract：NULL + 默认 false 保证向后兼容，零停机）。
- **Gateway 入口过滤**：`handlers/inbound.rs` 在 Bearer 校验、字段校验通过且加载到 Bot 后新增一层判定——当 `platform=telegram && raw_message.chat_type=group && bot.require_mention=true` 时，按 `bot.telegram_username`（大小写不敏感、首次命中子串即视为 mention）校验 `raw_message.text`；未命中则返回 HTTP 200 + `{"status":"ignored_no_mention"}`，不写 `message_events`、不调 Runtime、不触发 Bridge 回写。
- **`InboundStatus` 枚举扩展**：`SSoT/api/main.tsp` 将 `status` 允许值从 `"accepted" | "ignored_duplicate"` 扩展为 `"accepted" | "ignored_duplicate" | "ignored_no_mention"`，codegen 同步。
- **单元测试**：7 场景覆盖（5 基础场景：私聊直通 / 群聊 @ 触发 / 群聊无 @ 忽略 / 大小写不敏感 / `require_mention=false` 退化；加 2 条顺序约束：Mention 在 BR-055 前 / 空文本先拦椒 BR-001）。

### 修改功能

- `gateway/src/db/bots.rs` 的 `Bot` struct 与 `bots::get_by_id` 查询暴露 `telegram_username`、`require_mention` 两个新字段（读取路径严格遵循 BR-032，禁止全表扫）。
- `gateway/src/models/inbound.rs` 的 `InboundResponse.status` 允许值扩展 `ignored_no_mention`。
- `.context/domain/domain_model.md` 的 Bot 实体定义同步新增两个字段。
- `.context/architecture/api_strategy.md` 补充 `ignored_no_mention` 的请求/响应示例（含群聊无 @ 场景）。

**非 BREAKING**：响应 JSON 保持 `{"status": "..."}` 结构；`ignored_no_mention` 是 200 正常响应的新增状态值，Bridge 侧消费者若仅观测 HTTP 2xx 不会受影响。

### 技术实现

- Goose 迁移 `SSoT/schema/migrations/00005_bots_mention_filter.sql`（`ADD COLUMN IF NOT EXISTS`，NULL / 默认 false）。
- TypeSpec 扩展 `InboundStatus` union → `tsp compile` 生成 OpenAPI → `openapi-generator-rs` 刷新客户端产物。
- Rust 侧过滤实现：`raw_message.text.to_ascii_lowercase().contains(&format!("@{}", bot.telegram_username.to_ascii_lowercase()))`，保持 O(n) 且无正则依赖。
- 日志：命中过滤时打 `inbound skipped: group_no_mention`（含 `bot_id` / `chat_id`，不记 `telegram_username` 明文）。

### 范围外（Out of Scope）

- 非 Telegram 平台的 mention 语法（企业微信 / 飞书等后续按平台扩展）。
- 多个 username 别名匹配（单字段单值）。
- `channel_bindings` 级别的 `require_mention` 覆盖（MVP 粒度在 bot 级）。
- **其他 `chat_type`**：仅对 `chat_type=group` 进行 过滤；`chat_type=private` 及未来可能引入的其他类型均不适用本过滤逻辑。
- **回归边界**：空文本（BR-001）和超长消息（BR-002）的正常封帛行为不受影响，本提案 MUST NOT 改变其返回层次或情境。

## Impact

### 涉及的规范（Specs）

- **变更匹配目录 delta spec**：`openspec/changes/feat-gateway-mention-filter/specs/inbound-gate/spec.md` — ADDED 2 条 Requirement：`群聊 @mention 过滤`（含 9 个 Scenario，覆盖基础行为、大小写、私聊直通、退化兼容、BR-032、迁移向后兼容、顺序约束 x2）和 `InboundResponse 状态枚举扩展`（含 3 个 Scenario）；关联套用 `openspec/specs/inbound-gate/spec.md` 的履历规范作为基线。

### 涉及的代码

- **新增**：
  - `SSoT/schema/migrations/00005_bots_mention_filter.sql`
  - `gateway/tests/inbound_mention_filter.rs`（7 场景单元 / 集成测试）

- **修改**：
  - `SSoT/api/main.tsp`（`InboundStatus` union 扩展）
  - `SSoT/api/tsp-output/`（codegen 产物，随 `make` 刷新）
  - `gateway/src/db/bots.rs`（Bot struct + `get_by_id` 查询）
  - `gateway/src/models/inbound.rs`（`InboundResponse.status` 语义）
  - `gateway/src/handlers/inbound.rs`（mention 过滤判定）
  - `.context/domain/domain_model.md`（Bot 实体字段）
  - `.context/architecture/api_strategy.md`（`ignored_no_mention` 示例）

### 依赖关系

- **前置**：`feat-gateway-inbound-gate`（handler 架构已就位）、`feat-e2e-integration-test`（主链路已稳定）。
- **被依赖**：无。
- 与 `feat-runtime-reply-bridge` 的 `ignored_duplicate` 语义正交，不冲突。

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|------|
| criterion | `.context/criterion.md` | §3.2 非文本消息拦截 / §4 MUST 规则边界；本提案新增的“群聊无 @ 忽略”属 200 正常语义、非错误 |
| domain | `.context/domain/business_rules.md` | BR-032（bot_id 隔离读取）、BR-001（可忽略输入）、BR-010（私聊独立路由）、BR-012（私聊/群聊隔离）、BR-031（Bearer）、BR-055（限流顺序约束） |
| domain | `.context/domain/domain_model.md` | Bot 实体新增 `telegram_username` / `require_mention` 字段 |
| domain | `.context/domain/edge_cases.md` | 群聊无 @ 消息归类 `ignored_no_mention`（非 400，属预期过滤） |
| domain | `.context/domain/user_journeys.md` | 群聊用户旅程中启动 @ 触发 Bot 的正常流程 |
| domain | `.context/domain/testing_strategy.md` | 单元/集成测试分层要求；7 场景对应该文档定义的层次领域 |
| domain | `.context/domain/risks_and_debt.md` | RISK-B007（新增，过滤规则引发漏响应）待登记 |
| architecture | `.context/architecture/api_strategy.md` | InboundResponse 新增 `ignored_no_mention` 状态及示例 |
| architecture | `.context/architecture/security_policy.md` | 日志脱敏策略（§65-74, §99-109）：MUST NOT 记录 `telegram_username` 明文 |
| architecture | `.context/architecture/cross_cutting_concepts.md` | 可观测性、日志规范、统一错误响应格式 |
| architecture | `.context/architecture/runtime_view.md` | Gateway → Runtime 调用链路图；Mention 过滤在调用入口层不影响 Runtime 触发路径 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-006（Bearer Token 泄露）不进入本提案范围；RISK-B002（Runtime/Bridge 边界混淆）不被本提案的 Gateway 层设计引入 |
| db | `.context/db/schema_design.md` | `bots` 表新增两列 |
| db | `.context/db/migrations_and_ssot.md` | Expand-Contract 迁移策略（NULL + 默认 false → 部署代码 → 写入数据） |

### 风险与注意事项

- **RISK-B007（新增，过滤规则引发漏响应）** — 默认 `require_mention=false` 保证现有群不受影响；开启必须运营显式 `UPDATE` 并经灰度单群先验证。**提案归档前 MUST 在 `.context/domain/risks_and_debt.md` 新增该条目**（现有清单止于 `RISK-B006`；禁止复用 `RISK-B002`—Runtime/Bridge 边界混淆 的编号）。
- **数据治理（`telegram_username` 字段）** — username 本身非敏感凭证（不同于 `RISK-006` Bearer Token），因此不复用该编号；但日志与错误响应 MUST NOT 记录 `telegram_username` 明文，避免被用作侧信道推测 bot 映射。
- **误判风险（子串命中）** — 若 `telegram_username = "Ops"`，普通文本 "Operations" 中子串 `@Ops` 不会命中（匹配模式要求 `@` 前缀），降低误判面；测试用例覆盖该场景。
- **与限流的顺序耦合（回应评审 Q1）** — 若 Token Bucket（BR-055）早于 Mention 过滤，群聊闲聊会耗尽 `chat_id` 令牌桶，为真正 @ 消息的 429 短路埋下隐患。本提案强制要求 Mention 过滤在限流之前、空文本校验（BR-001）之后执行（详见 `design.md > Decision 6` 与 `tasks.md > 3.3`）。

### 验证标准

- ✅ `bots` 表迁移后包含 `telegram_username` / `require_mention` 列，既有行不被破坏（NULL / false）。
- ✅ 私聊（`chat_type=private`）即使 `require_mention=true` 也全量放行。
- ✅ 群聊 `@CBECOpsBot hi` → 200 `{"status":"accepted"}` → Runtime 处理。
- ✅ 群聊 `今天天气不错` → 200 `{"status":"ignored_no_mention"}`；`message_events` 无新增；NanoBot 未被调用。
- ✅ 大小写不敏感：`@cbecopsbot` / `@CBECOpsBot` 均触发。
- ✅ `require_mention=false` 的 bot 保持当前“全量响应”行为。
- ✅ **顺序约束**：群聊闲聊（1 秒内 10 条、均无 @）不耗尽 `chat_id` 令牌桶；随后 1 条 `@CBECOpsBot` 消息应正常 `accepted`（Mention 在 BR-055 前）。
- ✅ **SSoT / codegen 一致性**：`SSoT/api/main.tsp` `InboundStatus` union 为三値；`tsp compile` + `openapi-generator-rs` 产物同步。
- ✅ **日志脱敏**：Gateway 日志 MUST NOT 记录 `telegram_username` 明文（对照 `.context/architecture/security_policy.md` §65-74）。
- ✅ `node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-mention-filter --strict` 通过。

### 提案大纲对齐（Roadmap Alignment）

- **roadmap_source_primary**：`openspec/proposal-roadmap.md`（§提案 15，行 890-975）
- **roadmap_source_supplement**：N/A（无 `proposal-roadmap-Phase-5.md`）
- **phase**：Phase 5（运营增强，Post-MVP）
- **change_id**：`feat-gateway-mention-filter`
- **business_goal**：群聊降噪 / 多租户安全 / 私聊直响（已覆盖于 `## Why`）
- **dependencies**：前置 `feat-gateway-inbound-gate` + `feat-e2e-integration-test`；无被依赖（已覆盖于 `## Impact > 依赖关系`）
- **acceptance_criteria**：roadmap 原生 7 条验收条目已在 `## Impact > 验证标准` 展开（并补充 3 条新增）；其中「迁移不破坏既有行」和「BR-032 读取路径」已在 spec delta `群聊 @mention 过滤` Requirement 下新增追溯 Scenario；「私聊直通」 / 「群聊 @ 触发」 / 「群聊无 @ 忽略」 / 「大小写」 / 「退化兼容」5 条均有对应 Scenario；共 12 个 Scenario（第一 Requirement 9 条 + 第二 Requirement 3 条）全部可通过 spec delta 追溯
- **key_tasks**：Goose 迁移 / SSoT 同步 / Bot struct / Handler 过滤 / 单元测试（5 项已映射至 `tasks.md` 的 Phase 1-4）
- **扩展字段**（milestones/coverage_scope/gate_vs_non_gate/change_management/ops_support/kpi/risk_acceptance_policy）：roadmap 条目未声明，标注 N/A
