# 实施任务清单

> **前置条件**：`feat-gateway-channel-session` 已完成并合并；`SSoT/schema/migrations/00001_init.sql` 已执行（`message_events` 表和 `uq_message_events_inbound_dedup` 索引均在该迁移中定义，无独立的 `00004`/`00005` 迁移）。若迁移未执行，ON CONFLICT 会返回 DB 错误→503，不会静默失效，无需额外起动检查。

## 1. StandardMessage struct

- [x] 1.1 新建 `gateway/src/models/standard_message.rs`
  - 定义 `StandardMessage` struct，字段：`event_id: String`、`platform: String`、`chat_id: String`、`chat_type: String`、`user_id: String`、`session_id: String`、`text: String`、`timestamp: String`、`bot_id: Uuid`
  - 实现 `StandardMessage::build(req: &InboundRequest, bot_id: Uuid, session_id: &str) -> Self`，`event_id` = `Uuid::new_v4().to_string()`
- [x] 1.2 在 `gateway/src/models/mod.rs` 中 `pub mod standard_message;`

## 2. message_events DB 层

- [x] 2.1 新建 `gateway/src/db/message_events.rs`
  - 实现 `insert_pending(pool, msg: &StandardMessage, input_text: &str, reply_id: &str) -> Result<Option<Uuid>, sqlx::Error>`
    - 使用 `INSERT ... ON CONFLICT (platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''), bridge_message_id) DO NOTHING RETURNING id`
    - 注意：`uq_message_events_inbound_dedup` 是表达式索引，不能用 `ON CONFLICT ON CONSTRAINT` 引用，必须写出完整 COALESCE 表达式
    - 返回 `Some(id)` = 新行插入，`None` = 冲突（重复）；sqlx 的 `fetch_optional` 自然映射
    - INSERT 字段包含 `bot_id`（BR-032）
  - 实现 `mark_processing(pool, event_id: &str, bot_id: Uuid) -> Result<(), sqlx::Error>`
    - `UPDATE message_events SET status='processing' WHERE event_id=$1 AND bot_id=$2`
  - 实现 `mark_done(pool, event_id: &str, bot_id: Uuid, output_text: Option<&str>) -> Result<(), sqlx::Error>`
    - `UPDATE message_events SET status='done', output_text=$3 WHERE event_id=$1 AND bot_id=$2`（为 Runtime 提案预留，本提案实现但不调用）
  - 实现 `mark_error(pool, event_id: &str, bot_id: Uuid, error_code: &str, error_message: &str) -> Result<(), sqlx::Error>`
    - 同上（为 Runtime 提案预留，本提案实现但不调用）
- [x] 2.2 在 `gateway/src/db/mod.rs` 中 `pub mod message_events;`

## 3. inbound handler 集成

- [x] 3.1 在 `gateway/src/handlers/inbound.rs` session upsert 成功后插入：
  1. 计算 `input_text_truncated`: `text.chars().take(512).collect::<String>()`
  2. 生成 `reply_id`: `Uuid::new_v4().to_string()`
  3. 调用 `StandardMessage::build(&req, bot_id, &session_id)` → `std_msg`
  4. 调用 `db::message_events::insert_pending(&pool, &std_msg, &input_text_truncated, &reply_id)`
     - 返回 `None`（醲等冲突）→ 返回 `HTTP 200 + {"status":"ignored_duplicate"}`，结束处理
     - DB 错误 → 503 熔断 + `db_unavailable_total` 递增
  5. 调用 `db::message_events::mark_processing(&pool, &std_msg.event_id, bot_id)`
     - DB 错误 → 503 熔断（同上）
- [x] 3.2 更新 `InboundResponse` / `InboundStatus`，新增 `IgnoredDuplicate` 变体，序列化为 `"ignored_duplicate"`

## 4. 单元测试

- [x] 4.1 `standard_message.rs` 单元测试
  - [x] 4.1.1 `build()` 生成的 `event_id` 符合 UUID v4 格式（regex 或 `Uuid::parse_str` + version 检查）
  - [x] 4.1.2 所有必填字段均非空字符串

- [x] 4.2 `db/message_events.rs` 单元测试（需 PostgreSQL，可用 `#[sqlx::test]`）
  - [x] 4.2.1 `insert_pending()` 首次插入返回 `Some(id)`（`id` 为 UUID），`message_events` 有对应行，`status = 'pending'`
  - [x] 4.2.2 `insert_pending()` 重复相同幂等键返回 `None`，表中仅一行
  - [x] 4.2.3 `mark_processing()` 将 `status` 更新为 `'processing'`
  - [x] 4.2.4 `input_text` = 传入的截断字符串（验证字段正确写入）

- [x] 4.3 `handlers/inbound.rs` 集成测试（4.3.2 dead pool；4.3.1 需 PostgreSQL）
  - [x] 4.3.1 重复消息 → HTTP 200 + body `{"status":"ignored_duplicate"}`（已通过 `DATABASE_URL=... cargo test ... -- --ignored` 验证）
  - [x] 4.3.2 DB 不可达时 `insert_pending` 路径 → HTTP 503 + `db_unavailable_total` 递增（dead pool）
  - [x] 4.3.3 text = 600 字符时 `input_text` 截断为前 512 字符（字符截断函数单测已覆盖）

## 5. Cargo 依赖检查

- [x] 5.1 确认 `gateway/Cargo.toml` 中 `uuid = { version = "1", features = ["v4"] }` 已存在或添加

## 6. 验证与归档

- [x] 6.1 运行 `specflow validate feat-gateway-message-pipeline --strict`（即 `node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-message-pipeline --strict`），确认无报错
- [x] 6.2 运行 `cargo test -p gateway`（30 passed / 11 ignored）+ 关键 DB ignored 用例已用 `DATABASE_URL=... cargo test <case> -- --ignored` 单独执行通过
- [x] 6.3 代码审查：BR-032 覆盖（bot_id 在所有 INSERT/UPDATE 中）、幂等去重正确、截断逻辑使用 `chars()` 而非字节
- [x] 6.4 归档：`specflow archive feat-gateway-message-pipeline --yes`（即 `node design/context-dev/tools/specflow/specflow.mjs archive feat-gateway-message-pipeline --yes`）

### 6.x 验证证据（命令 + 输出摘要）

> 说明：`DATABASE_URL` 可能包含 `&`，复现时请从 `gateway/.env` 提取并用**双引号**包裹，避免 shell 误解析。

- `DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')" cargo test db::message_events::tests::insert_pending_first_time_returns_some_and_pending -- --ignored`
  - 输出摘要：`running 1 test` → `...insert_pending_first_time_returns_some_and_pending ... ok` → `test result: ok. 1 passed; 0 failed`
- `DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')" cargo test db::message_events::tests::insert_pending_duplicate_returns_none_and_only_one_row -- --ignored`
  - 输出摘要：`running 1 test` → `...insert_pending_duplicate_returns_none_and_only_one_row ... ok` → `test result: ok. 1 passed; 0 failed`
- `DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')" cargo test db::message_events::tests::mark_processing_updates_status -- --ignored`
  - 输出摘要：`running 1 test` → `...mark_processing_updates_status ... ok` → `test result: ok. 1 passed; 0 failed`
- `DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')" cargo test db::message_events::tests::insert_pending_persists_input_text -- --ignored`
  - 输出摘要：`running 1 test` → `...insert_pending_persists_input_text ... ok` → `test result: ok. 1 passed; 0 failed`
- `DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')" cargo test handlers::inbound::tests::duplicate_message_returns_ignored_duplicate -- --ignored`
  - 输出摘要：`running 1 test` → `...duplicate_message_returns_ignored_duplicate ... ok` → `test result: ok. 1 passed; 0 failed`
- `cargo test`（在 `gateway/`）
  - 输出摘要：`test result: ok. 30 passed; 0 failed; 11 ignored`
- `node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-message-pipeline --strict`
  - 输出摘要：`=== Specflow Validate: feat-gateway-message-pipeline (strict) ===` + `✅ OK`
