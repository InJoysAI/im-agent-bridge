# 实施任务清单

> 严格按 SSoT-first 顺序推进：先改 schema / API 合约，再运行 codegen，再落代码与测试。Roadmap 条目（`openspec/proposal-roadmap.md:890-975`）中的 5 项 `key_tasks` 已在下方 Phase 1-4 可执行拆解，不得偏离。

## 1. SSoT：Schema 迁移 + API 契约扩展

- [x] 1.1 编写 Goose 迁移 `SSoT/schema/migrations/00005_bots_mention_filter.sql`
  - 使用 `ALTER TABLE bots ADD COLUMN IF NOT EXISTS telegram_username TEXT NULL`
  - 使用 `ALTER TABLE bots ADD COLUMN IF NOT EXISTS require_mention BOOLEAN NOT NULL DEFAULT FALSE`
  - 迁移脚本包含 `-- +goose Up` / `-- +goose Down` 两段；Down 段执行 `DROP COLUMN`
- [x] 1.2 扩展 `SSoT/api/main.tsp` 的 `InboundStatus` union：`"accepted" | "ignored_duplicate" | "ignored_no_mention"`
- [x] 1.3 同步 `.context/domain/domain_model.md` Bot 实体字段说明（新增 `telegram_username` / `require_mention` 两列的定义与默认值）
- [x] 1.4 同步 `.context/db/schema_design.md` `bots` 表列清单
- [x] 1.5 执行迁移（环境无 `goose`，通过 `psql` 等价执行 Up 语句并完成幂等复验）
  ```bash
  export GOOSE_DRIVER=postgres
  export DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')"
  export GOOSE_DBSTRING="$DATABASE_URL"
  make db-migrate-up
  ```

## 2. Codegen 与上下文示例

- [x] 2.1 在 `SSoT/api/` 下运行 `make api-compile` 刷新 OpenAPI（遵循仓库既有 Makefile / package.json 约定）
- [x] 2.2 运行 `openapi-generator-rs` 刷新 Rust 客户端与 server-side schema（如仓库已有 `make codegen` / `make generate` 则直接调用）
- [x] 2.3 在 `.context/architecture/api_strategy.md` 追加 `ignored_no_mention` 的请求 / 响应示例（含群聊无 @ 场景的 curl 样例）
- [x] 2.4 确认本提案不涉及新错误码（`ignored_no_mention` 为 200 正常响应状态值，跳过 `make errcode-gen` 流程；在本任务勾选项中留痕）

## 3. Gateway 代码落地

- [x] 3.1 `gateway/src/db/bots.rs`：Bot struct 新增 `telegram_username: Option<String>` / `require_mention: bool` 字段；`bots::get_by_id` 查询语句 SELECT 列表补齐这两列
- [x] 3.2 `gateway/src/models/inbound.rs`：`InboundResponse.status` 允许值注释更新为三值枚举（若使用 `serde` 字符串则保证反序列化兼容）
- [x] 3.3 `gateway/src/handlers/inbound.rs`：按 **固定顺序** 插入 mention 过滤判定（严格遵循 `design.md > Decision 6`）
  - **执行链（自上而下）**：Bearer → 字段/JSON 校验 → `message_type != text` 拦截（BR-001）→ 空文本/仅空白校验（BR-001 / `edge_cases.md:17-18`）→ 长度 > 4096 拦截（BR-002）→ `bots::get_by_id(bot_id)` 加载（BR-032）→ **@mention 过滤（本任务）** → Token Bucket 限流（BR-055）→ 写 `message_events` + Runtime
  - **Mention 过滤 MUST 位于 BR-055 限流之前**（回应评审 Q1）：若颠倒，群聊闲聊会耗尽 `chat_id` 令牌桶，真正 @ 消息会被 429 短路
  - **Mention 过滤 MUST 位于空文本校验（BR-001）之后**（回应评审 Q2）：空字符串在此步骤前已被 400 拒绝，避免 mention 分支处理退化输入
  - 分支条件：`platform == "telegram" && raw_message.chat_type == "group" && bot.require_mention == true`
  - 匹配实现：`text.to_ascii_lowercase().contains(&format!("@{}", username.to_ascii_lowercase()))`
  - 未命中 → 返回 `(StatusCode::OK, Json(InboundResponse { status: "ignored_no_mention" }))`；命中 / 分支条件不成立 → 继续主链路
  - 在代码处添加注释 `// BR-055 ordering: mention filter precedes rate limit; see design.md Decision 6`
- [x] 3.4 日志：命中过滤时 `tracing::info!(bot_id=..., chat_id=..., "inbound skipped: group_no_mention")`；禁止记录 `telegram_username` 明文（避免侧信道推测）
- [x] 3.5 `bots::get_by_id` 的调用位置保持现有位置不变（BR-032 — 禁止新增基于 `telegram_username` 的反查）
- [x] 3.6 **登记新风险**：在 `.context/domain/risks_and_debt.md` 新增 `RISK-B007: 过滤规则引发漏响应` 条目（严重度、触发条件、缓解措施、负责人）— 本步骤 MUST 在归档前完成，避免风险编号在 roadmap/提案与 .context 间孤儿化

## 4. 测试

- [x] 4.1 单元测试（`gateway/src/handlers/inbound.rs` + `gateway/tests/inbound_mention_filter.rs`，7 场景：5 基础 + 2 顺序约束）
  - [x] 4.1.1 群聊 `@CBECOpsBot hi` → 200 `{"status":"accepted"}` → 主链路触发
  - [x] 4.1.2 群聊 `"今天天气不错"`（无 @）→ 200 `{"status":"ignored_no_mention"}`，`message_events` 计数不增
  - [x] 4.1.3 群聊 `@cbecopsbot hi`（全小写）→ `{"status":"accepted"}`
  - [x] 4.1.4 私聊 `"你好"`（`chat_type=private`）→ `{"status":"accepted"}`
  - [x] 4.1.5 `require_mention=false` 的 Bot 群聊无 @ → `{"status":"accepted"}`
  - [x] 4.1.6 **顺序约束（BR-055）**：同 `chat_id` 1 秒内发 10 条无 @ → 均 `ignored_no_mention` 且令牌桶未被消耗；随后 1 条 `@CBECOpsBot` → `accepted`（验证 Mention 在 Token Bucket 之前）
  - [x] 4.1.7 **顺序约束（BR-001）**：`text=""` 空字符串 → HTTP 400（由空文本校验拦截），不进入 mention 匹配分支
- [x] 4.2 集成测试：对真实 Postgres 执行迁移 → 种 Bot 数据 → 端到端 POST `/gateway/inbound`
  - [x] 4.2.1 `UPDATE bots SET telegram_username='CBECOpsBot', require_mention=true WHERE id=...`
  - [x] 4.2.2 验证 4.1.1 与 4.1.2 两条关键路径在真实 DB 上的行为一致
- [x] 4.3 手动联调（Telegram 群聊）
  - [x] 4.3.1 群聊发 `"@CBECOpsBot 你好"` → Bot 正常回复
  - [x] 4.3.2 群聊发 `"早上好"`（无 @）→ 无回复；Gateway 日志出现 `inbound skipped: group_no_mention`

## 5. 文档

- [x] 5.1 更新 README / runbook 中 `bots` 表字段说明（若 README 列出了 Bot 表结构）（当前仓库未维护该结构说明，记为 N/A）
- [x] 5.2 `.context/domain/edge_cases.md` 补充"群聊无 @ 消息 → 200 ignored_no_mention（非 400）"的边缘场景条目

## 6. 验证与归档

- [x] 6.1 运行 `cargo test -p gateway --test inbound_mention_filter`，全部 7 场景通过（5 基础 + 2 顺序约束）（本地单仓结构通过 `cargo test` + `cargo test mention_filter_` 验证）
- [x] 6.2 运行迁移：`make db-migrate-up`，迁移成功且幂等（环境无 `goose`，通过 `psql` 等价执行并复跑验证幂等）
- [x] 6.3 验证所有验收标准
  - [x] 6.3.1 迁移后既有 Bot 行 `telegram_username IS NULL` / `require_mention = false`，行数不减
  - [x] 6.3.2 BR-032 读取路径回归：`bots::get_by_id` 仍是唯一加载入口（grep 确认无新增 `SELECT * FROM bots`）
- [x] 6.4 运行 specflow validate feat-gateway-mention-filter --strict（完整命令：`node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-mention-filter --strict`），所有检查通过
- [x] 6.5 代码审查：安全性（日志脱敏）、性能（单次子串匹配 O(n)）、向后兼容（默认 false 行为不变）
- [x] 6.6 合并后运行 specflow archive feat-gateway-mention-filter --yes（完整命令：`node design/context-dev/tools/specflow/specflow.mjs archive feat-gateway-mention-filter --yes`）归档提案到 `openspec/changes/archive/`
