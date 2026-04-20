# 实施任务清单

> **Roadmap 对齐**: `feat-gateway-channel-session` | Phase 1 | 前置: `feat-gateway-inbound-gate`（已完成）| 预计 2 天

## 1. SSoT 先行检查与迁移
- [x] 1.1 确认 `SSoT/schema/migrations/00001_init.sql` 中 `channel_bindings` 与 `sessions` 表已就位
  - `idx_channel_bindings_lookup` 索引（`COALESCE(bridge_channel_name,'')` 降级）✅
  - `uq_channel_bindings_source` 联合唯一约束（`00002_channel_bindings_unique.sql`）✅
  - `sessions.session_id UNIQUE`（全局单列，多 Bot 场景有冲突风险）⚠️ → 需迁移升级
- [x] 1.1.1 确认 `SSoT/schema/migrations/00003_sessions_bot_session_unique.sql` 已创建，内容正确
  - **Up**：`DROP CONSTRAINT sessions_session_id_key` + `CREATE UNIQUE INDEX uq_sessions_bot_session ON sessions (bot_id, session_id)` ✅
  - **Down**：可逆 ✅
  - **结论：需运行 Goose 迁移 00003，更新本地/测试 DB 后再执行手动测试**
- [x] 1.2 确认 `SSoT/api/main.tsp` 的 `POST /gateway/inbound` 已含 404 响应定义 ✅
  - **结论：SSoT 未更改，无需修改 TypeSpec**

## 2. db/channel_bindings.rs 实现（0.5 天）
- [x] 2.1 创建 `gateway/src/db/channel_bindings.rs`
  - [x] 2.1.1 实现 `find_bot_id_by_channel(pool, platform, bridge_gateway_name, bridge_channel_name: Option<&str>) -> Result<Option<Uuid>>`
    - 若 `bridge_channel_name = Some(name)`：先精确匹配 `WHERE platform=$1 AND bridge_gateway_name=$2 AND COALESCE(bridge_channel_name,'')=$3 AND is_enabled=true`；若 None，降级查询 `WHERE platform=$1 AND bridge_gateway_name=$2 AND bridge_channel_name IS NULL AND is_enabled=true`
    - 若 `bridge_channel_name = None`：直接走降级查询，跳过精确查询 roundtrip
    - 注：channel_bindings 是 bot_id 的解析源头，查询谓词为来源三元组，**无需**也**无法**以 bot_id 过滤
  - [x] 2.1.2 函数签名使用 sqlx PgPool，返回类型为 `Result<Option<Uuid>, sqlx::Error>`
- [x] 2.2 在 `gateway/src/db/mod.rs` 中添加 `pub mod channel_bindings`

## 3. generate_session_id() 实现（0.25 天）
- [x] 3.1 在 `gateway/src/models/` 中新增 session_id 生成纯函数（`gateway/src/models/session.rs`）
  - [x] 3.1.1 `pub fn generate_session_id(platform: &str, chat_type: &str, chat_id: &str) -> String`
    - `"private"` → `"telegram:private:{chat_id}"`
    - `"group"` → `"telegram:group:{chat_id}"`
    - 其他（防御性）→ `"telegram:unknown:{chat_id}"` + WARN 日志

## 4. db/sessions.rs 实现（0.25 天）
- [x] 4.1 创建 `gateway/src/db/sessions.rs`
  - [x] 4.1.1 实现 `upsert_session(pool, session_id, bot_id, platform, chat_id, chat_type, last_user_id) -> Result<()>`
    - SQL：`INSERT INTO sessions (id, session_id, bot_id, platform, chat_id, chat_type, last_user_id, created_at, updated_at) VALUES (...) ON CONFLICT (bot_id, session_id) DO UPDATE SET updated_at = NOW(), last_user_id = EXCLUDED.last_user_id`（冲突键对应 `uq_sessions_bot_session`，由迁移 `00003` 建立）
    - `id` 使用 `uuid::Uuid::new_v4()`（应用层生成，BR 规范）
  - [x] 4.1.2 函数签名携带 `bot_id: Uuid` 参数（BR-032 隔离要求）
- [x] 4.2 在 `gateway/src/db/mod.rs` 中添加 `pub mod sessions`

## 5. 集成到 inbound handler（0.25 天）
- [x] 5.1 修改 `gateway/src/handlers/inbound.rs`，在限流检查通过后追加：
  - [x] 5.1.1 调用 `channel_bindings::find_bot_id_by_channel()`
    - None → 返回 HTTP 404 + `{"error": "channel binding not found"}` + WARN 日志（含 platform/gateway/channel 字段）
  - [x] 5.1.2 调用 `generate_session_id()` 生成 `session_id`
  - [x] 5.1.3 调用 `sessions::upsert_session()`（DB 不可用 → 503 熔断，复用 `feat-gateway-db-layer` 熔断逻辑）

## 6. 单元测试（0.5 天）
- [x] 6.1 `channel_bindings` 模块测试（`#[ignore]` — 需 DATABASE_URL + seed 数据）
  - [x] 6.1.1 精确匹配命中 → 返回正确 bot_id
  - [x] 6.1.2 精确无结果 + 降级命中 → 返回正确 bot_id
  - [x] 6.1.3 精确 + 降级均无结果 → 返回 None
- [x] 6.2 `generate_session_id()` 纯函数测试（无 DB 依赖，全部通过）
  - [x] 6.2.1 `chat_type="private"` → `"telegram:private:{chat_id}"`
  - [x] 6.2.2 `chat_type="group"` → `"telegram:group:{chat_id}"`
  - [x] 6.2.3 私聊/群聊相同 chat_id → 两个不同 session_id（隔离验证）
- [x] 6.3 `sessions::upsert_session()` 测试（`#[ignore]` — 需 DATABASE_URL + goose up 00003）
  - [x] 6.3.1 首次插入 → 记录存在，created_at 已设置
  - [x] 6.3.2 重复插入同 session_id → 不报错，updated_at 更新
- [x] 6.4 运行 `cargo test` 确认全部通过（26 passed / 0 failed / 6 ignored）
  - [x] 运行 `cargo test -- --ignored`（需 DATABASE_URL）确认 DB 集成测试全部通过（6 passed / 0 failed）
    - `db::channel_bindings::tests::exact_match_returns_bot_id` ✅
    - `db::channel_bindings::tests::fallback_match_returns_bot_id` ✅
    - `db::channel_bindings::tests::no_match_returns_none` ✅
    - `db::pool::tests::health_check_returns_ok_when_pg_reachable` ✅
    - `db::sessions::tests::first_insert_creates_record` ✅
    - `db::sessions::tests::duplicate_upsert_is_idempotent` ✅

## 7. 不涉及新错误码
> 本提案涉及的失败场景（channel 未找到 → 404，DB 不可用 → 503）均映射到已有 HTTP 状态码与标准错误响应格式，无需自定义 MMTXXX 错误码（项目当前也未启用 errcode SSoT）。

## 8. 手动测试（在步骤 2–5 完成后执行）

> **前提**：Gateway 正在运行，DB 已迁移，环境变量已设置。

### 8.0 环境准备

**环境变量**（参考 `gateway/.env.example`）：
```sh
export GATEWAY_BEARER_TOKEN=<your-gateway-bearer-token>
export DATABASE_URL=postgres://user:password@localhost:5432/im_agent_bridge
export GATEWAY_URL=http://localhost:8080   # 按实际端口调整
```

**Seed 数据**（确保已执行）：
```sh
bash scripts/seed_db.sh "$DATABASE_URL"
# 写入：default-bot（id: 11111111-...）+ 降级绑定（bridge_channel_name=NULL）
```

**追加精确绑定**（MT-1 需要）：
```sql
-- psql $DATABASE_URL
INSERT INTO channel_bindings
  (id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled)
VALUES (
  '33333333-3333-4333-8333-333333333333',
  '11111111-1111-4111-8111-111111111111',
  'telegram', 'default', 'general', TRUE
) ON CONFLICT DO NOTHING;
```

---

### 8.1 MT-1：精确匹配命中 → 200 + session 写入

- [x] 8.1.1 发送请求（`bridge_channel_name='general'`，精确绑定存在）

```sh
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "$GATEWAY_URL/gateway/inbound" \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "telegram",
    "bridge_gateway_name": "default",
    "bridge_channel_name": "general",
    "raw_message": {
      "chat_id": "111000001",
      "chat_type": "private",
      "user_id": "user-111",
      "message_type": "text",
      "text": "MT-1 精确匹配",
      "timestamp": "2026-04-16T00:00:00Z",
      "message_id": "mt1-msg-001"
    }
  }'
```

**预期响应**：`{"status":"accepted"}` / HTTP 200

- [x] 8.1.2 验证 sessions 写入

```sql
SELECT session_id, bot_id, platform, chat_id, chat_type, created_at
FROM sessions
WHERE session_id = 'telegram:private:111000001';
```

**预期**：1 行；`session_id = 'telegram:private:111000001'`；`bot_id = '11111111-1111-4111-8111-111111111111'` ✅

---

### 8.2 MT-2：COALESCE 降级匹配 → 200 + session 写入

- [x] 8.2.1 发送请求（`bridge_channel_name='some-unknown-channel'`，无精确绑定 → 降级到 NULL 绑定）

```sh
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "$GATEWAY_URL/gateway/inbound" \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "telegram",
    "bridge_gateway_name": "default",
    "bridge_channel_name": "some-unknown-channel",
    "raw_message": {
      "chat_id": "222000002",
      "chat_type": "private",
      "user_id": "user-222",
      "message_type": "text",
      "text": "MT-2 降级匹配",
      "timestamp": "2026-04-16T00:00:01Z",
      "message_id": "mt2-msg-001"
    }
  }'
```

**预期响应**：HTTP 200

- [x] 8.2.2 验证 session 通过降级路径正确写入

```sql
SELECT session_id, bot_id
FROM sessions
WHERE session_id = 'telegram:private:222000002';
```

**预期**：1 行；`bot_id = '11111111-1111-4111-8111-111111111111'`（来自 NULL 降级绑定）✅

---

### 8.3 MT-3：channel_bindings 完全缺失 → 404

- [x] 8.3.1 发送请求（`platform='slack'`，无任何绑定）

```sh
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "$GATEWAY_URL/gateway/inbound" \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "slack",
    "bridge_gateway_name": "default",
    "raw_message": {
      "chat_id": "333000003",
      "chat_type": "private",
      "user_id": "user-333",
      "message_type": "text",
      "text": "MT-3 无绑定",
      "timestamp": "2026-04-16T00:00:02Z",
      "message_id": "mt3-msg-001"
    }
  }'
```

**预期响应**：HTTP 404

- [x] 8.3.2 验证 sessions 表未写入任何记录

```sql
SELECT COUNT(*) FROM sessions WHERE chat_id = '333000003';
```

**预期**：`count = 0` ✅

---

### 8.4 MT-4：群聊 session_id 格式验证

- [x] 8.4.1 发送群聊消息

```sh
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "$GATEWAY_URL/gateway/inbound" \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "telegram",
    "bridge_gateway_name": "default",
    "raw_message": {
      "chat_id": "444000004",
      "chat_type": "group",
      "user_id": "user-444",
      "message_type": "text",
      "text": "MT-4 群聊 session_id",
      "timestamp": "2026-04-16T00:00:03Z",
      "message_id": "mt4-msg-001"
    }
  }'
```

- [x] 8.4.2 验证 session_id 格式

```sql
SELECT session_id, chat_type
FROM sessions
WHERE chat_id = '444000004';
```

**预期**：`session_id = 'telegram:group:444000004'` ✅

---

### 8.5 MT-5：sessions upsert 幂等

- [x] 8.5.1 记录 MT-1 session 的当前 `updated_at` 时间戳 T1

```sql
SELECT updated_at AS t1
FROM sessions
WHERE session_id = 'telegram:private:111000001';
```

- [x] 8.5.2 再次发送相同 chat_id 的请求（换新 `message_id` 避免幂等去重拦截）

```sh
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "$GATEWAY_URL/gateway/inbound" \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "telegram",
    "bridge_gateway_name": "default",
    "bridge_channel_name": "general",
    "raw_message": {
      "chat_id": "111000001",
      "chat_type": "private",
      "user_id": "user-111",
      "message_type": "text",
      "text": "MT-5 upsert 幂等",
      "timestamp": "2026-04-16T00:01:00Z",
      "message_id": "mt5-msg-002"
    }
  }'
```

**预期响应**：HTTP 200

- [x] 8.5.3 验证 sessions 仅 1 行，`updated_at` 已更新

```sql
SELECT COUNT(*) AS row_count, MAX(updated_at) AS last_updated
FROM sessions
WHERE session_id = 'telegram:private:111000001';
```

**预期**：`row_count = 1`；`last_updated > T1` ✅

---

### 8.6 MT-6：私聊/群聊上下文隔离（同一 chat_id 两条独立 session）

- [x] 8.6.1 发送与 MT-1 相同 chat_id 的群聊消息

```sh
curl -s -w "\nHTTP %{http_code}\n" \
  -X POST "$GATEWAY_URL/gateway/inbound" \
  -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "platform": "telegram",
    "bridge_gateway_name": "default",
    "bridge_channel_name": "general",
    "raw_message": {
      "chat_id": "111000001",
      "chat_type": "group",
      "user_id": "user-111",
      "message_type": "text",
      "text": "MT-6 群聊隔离",
      "timestamp": "2026-04-16T00:02:00Z",
      "message_id": "mt6-msg-001"
    }
  }'
```

- [x] 8.6.2 验证同一 chat_id 存在两条独立 session 记录

```sql
SELECT session_id, chat_type
FROM sessions
WHERE chat_id = '111000001'
ORDER BY chat_type;
```

**预期**（2 行）：
- `telegram:group:111000001`   / `group` ✅
- `telegram:private:111000001` / `private` ✅

---

### 8.7 MT-7：BR-032 隔离验证（代码审查 + DB 状态检查）

- [x] 8.7.1 确认迁移 `00003` 已应用：`sessions.session_id_key` 约束已被 `uq_sessions_bot_session` 替换

```sql
-- psql $DATABASE_URL
\d sessions
-- 预期：无 sessions_session_id_key 约束，存在 uq_sessions_bot_session 唯一索引（bot_id, session_id）
```

- [x] 8.7.2 确认 `channel_bindings.rs` 查询谓词为来源三元组（**无** bot_id 过滤，符合预期）

```sh
grep -n "WHERE\|platform\|bridge_gateway" gateway/src/db/channel_bindings.rs
# 预期：查询条件含 platform / bridge_gateway_name / bridge_channel_name，不含 bot_id 过滤
# 注：channel_bindings 用来源三元组解析 bot_id，不以 bot_id 过滤——这是正确的
```

- [x] 8.7.3 确认 `sessions.rs` 所有读写均携带 `bot_id` 且冲突键为 `(bot_id, session_id)`

```sh
grep -n "bot_id\|ON CONFLICT" gateway/src/db/sessions.rs
# 预期：ON CONFLICT (bot_id, session_id)；SELECT/WHERE 均含 bot_id 条件
```

**预期**：channel_bindings 以来源三元组解析 bot_id ✅；sessions 操作全部携带 bot_id ✅；无全表访问 ✅

---

### 8.8 清理测试数据（可选）

```sql
-- psql $DATABASE_URL
DELETE FROM sessions
  WHERE chat_id IN ('111000001','222000002','333000003','444000004');
DELETE FROM channel_bindings
  WHERE id = '33333333-3333-4333-8333-333333333333';
```

---

## 9. 验证
- [x] 9.1 运行 `specflow validate feat-gateway-channel-session --strict`
  ```sh
  node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-channel-session --strict
  ```

## 10. 归档
- [x] 10.1 运行 `specflow archive feat-gateway-channel-session --yes`
  ```sh
  node design/context-dev/tools/specflow/specflow.mjs archive feat-gateway-channel-session --yes
  ```
