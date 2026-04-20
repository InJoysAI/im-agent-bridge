# 实施任务清单

> SSoT-first 顺序（criterion.md §6）：Goose 迁移 → API 契约确认 → Rust 实现 → 测试 → 验证归档。所有 DB Schema 变更必须先执行 Goose 迁移文件，禁止裸 DDL。

## 1. SSoT 先行

- [x] 1.1 确认 `SSoT/schema/migrations/00004_bots_runtime_model.sql` 文件内容正确
  - Up：`ALTER TABLE bots ADD COLUMN runtime_model TEXT NOT NULL DEFAULT 'nanobot';`
  - Down：`ALTER TABLE bots DROP COLUMN runtime_model;`
- [x] 1.2 在开发环境执行 Up 迁移：
  ```bash
  export GOOSE_DRIVER=postgres
  export DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')"
  export GOOSE_DBSTRING="$DATABASE_URL"
  make db-migrate-up
  ```
- [x] 1.3 确认 `SSoT/api/main.tsp` 中 `RuntimeProcessRequest` 结构与 NanoBotAdapter 调用契约一致（本次无需修改，仅确认 `session_id`、`bot_id` 字段存在）

## 2. RuntimeAdapter trait

- [x] 2.1 新建 `gateway/src/adapters/runtime.rs`
  - 定义 `RuntimeAdapter` trait：`async fn process(&self, msg: &StandardMessage, bot: &BotConfig) -> Result<StandardReply, RuntimeError>`
  - 定义 `RuntimeError` enum：`Timeout | Unavailable | BadResponse | SessionNotFound`
  - 实现 error_code 字符串映射方法（对应 `RUNTIME_TIMEOUT` / `RUNTIME_UNAVAILABLE` / `RUNTIME_BAD_RESPONSE` / `RUNTIME_SESSION_NOT_FOUND`）

## 3. NanoBotAdapter 实现

- [x] 3.1 新建 `gateway/src/adapters/nanobot.rs`
  - 实现 `NanoBotAdapter { client: reqwest::Client }`
  - 构建请求体：`{model: bot.runtime_model, messages:[{role:"user",content:msg.text}], session_id: msg.session_id}`（不含 `stream` 字段）
  - HTTP 超时设置 15s
  - 响应解析：`choices[0].message.content`
- [x] 3.2 前置探针：确定 NanoBot session-not-found 错误结构（**编写映射前必须先完成**）
    ```bash
    # 向本地 NanoBot 发送一个伪造的不存在 session_id，记录精确的 HTTP 状态码与响应体
    curl -v -s http://localhost:8900/v1/chat/completions \
      -H "Content-Type: application/json" \
      -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"probe"}],"session_id":"probe-nonexistent-session-99999"}' \
      2>&1 | grep -E "< HTTP|^\{|error"
    ```
    探针结果：NanoBot 不存在 session-not-found 错误 — 对任意 session_id 均自动创建新 session 并返回 HTTP 200。
    `is_session_not_found()` 返回 `false` 为正确行为，暂无触发条件，保留为占位符供未来 NanoBot 版本变更后更新。
    附加发现：`runtime_model` 必须设为 LLM 实际模型名（如 `deepseek-chat`），migration DEFAULT `'nanobot'` 仅占位，
    上线前需通过 seed 或 UPDATE 配置为正确值。
- [x] 3.3 实现错误码映射
  - `reqwest::Error::is_timeout()` → `RuntimeError::Timeout`
  - `reqwest::Error::is_connect()` → `RuntimeError::Unavailable`
  - HTTP 2xx 但 `choices[0]` 缺失 → `RuntimeError::BadResponse`
  - Session 相关错误响应 → `RuntimeError::SessionNotFound`（触发条件待探针测试 3.2 确认）
- [x] 3.4 实现 4096 字符截断辅助函数（截断并追加 "…（内容已截断）"）

## 4. Bot 配置读取更新

- [x] 4.1 更新 `gateway/src/db/bots.rs`（或 Bot 配置查询模块）
  - 查询 struct 加入 `runtime_model: String` 字段
  - 确认 SQL 查询语句包含 `runtime_model` 列

## 5. Gateway 分发入口接入

- [x] 5.1 在 Runtime 分发入口（`inbound.rs` 或调度模块）按 `runtime_type` 实例化 Adapter：`"nanobot"` → `NanoBotAdapter`
- [x] 5.2 处理 `RuntimeError::SessionNotFound`：清空 `sessions.runtime_session_key` 并重建 session

## 6. 测试

- [x] 6.1 单元测试（mock NanoBot server，使用 `wiremock` 或 `mockito`）
  - [x] 6.1.1 正常请求构建验证（`model`、`session_id`、`messages` 1 条、无 `stream`）
  - [x] 6.1.2 正常响应解析（`choices[0].message.content`）
  - [x] 6.1.3 超时场景 → `RuntimeError::Timeout`，error_code = `RUNTIME_TIMEOUT`
  - [x] 6.1.4 连接不可达 → `RuntimeError::Unavailable`，error_code = `RUNTIME_UNAVAILABLE`
  - [x] 6.1.5 响应格式异常（`choices` 缺失）→ `RuntimeError::BadResponse`，error_code = `RUNTIME_BAD_RESPONSE`
  - [x] 6.1.6 4096 字符截断函数（边界值：4096、4097、5000 字符）

- [x] 6.2 集成测试
  - [x] 6.2.1 Gateway 调用 NanoBotAdapter 返回正常 `StandardReply { status: "success" }` （覆盖：adapters::nanobot::tests::normal_response_parsed_correctly）
  - [x] 6.2.2 `session_id` 字段在请求体中与 `StandardMessage.session_id` 一致 （覆盖：adapters::nanobot::tests::request_contains_correct_session_id）

- [x] 6.3 手动验证
  - [x] 6.3.1 验证 Goose 迁移 Up
    ```bash
    export GOOSE_DRIVER=postgres
    export DATABASE_URL="$(rg '^DATABASE_URL=' gateway/.env | head -n1 | sed 's/^DATABASE_URL=//')"
    export GOOSE_DBSTRING="$DATABASE_URL"
    make db-migrate-status   # 确认 00004 显示为 Pending
    make db-migrate-up
    make db-migrate-status   # 确认 00004 显示为 Applied
    ```
    验收：`\d bots` 可见 `runtime_model text not null default 'nanobot'`（现有行已通过 DEFAULT 自动填充 `'nanobot'`）
    > 💡 可选联调步骤（非上线必需）：若本地测试需使用其他 model 值，可手动执行 `UPDATE bots SET runtime_model = 'deepseek-chat' WHERE name = '<your-bot>';`
  - [x] 6.3.2 验证 Goose 迁移 Down
    ```bash
    make db-migrate-down
    make db-migrate-status   # 确认回滚至 00003
    make db-migrate-up       # 重新应用，确认幂等
    ```
    验收：Down 后 `runtime_model` 列消失，Up 后重新出现，无报错
  - [x] 6.3.3 验证 NanoBotAdapter 请求构建（使用 `cargo test` + wiremock mock server）
    - 确认请求体包含 `"model": "deepseek-chat"`（取自 `bots.runtime_model`）
    - 确认请求体包含 `"session_id": "telegram:private:{chat_id}"` 或 `"telegram:group:{chat_id}"`
    - 确认请求体 `"messages"` 数组严格 1 条，无 `"stream"` 字段
  - [x] 6.3.4 验证 DB 中存在已设置 `runtime_model` 的测试 bot 记录
    ```sql
    -- 检查 bots 表是否有 runtime_type='nanobot' 的记录
    SELECT id, name, runtime_type, runtime_model, runtime_endpoint
    FROM bots
    WHERE runtime_type = 'nanobot';
    -- 若无记录，插入测试用 Bot（按本地实际配置调整）：
    -- INSERT INTO bots (name, runtime_type, runtime_model, runtime_endpoint)
    -- VALUES ('test-bot', 'nanobot', 'deepseek-chat', 'http://localhost:8900');

    -- 检查 channel_bindings 是否有对应 tg-gateway 的绑定
    SELECT bot_id, bridge_gateway_name, bridge_channel_name
    FROM channel_bindings
    WHERE bridge_gateway_name = 'tg-gateway';

    -- 若无绑定，插入（bot_id 替换为上方查询到的实际 id）：
    -- INSERT INTO channel_bindings (bot_id, bridge_gateway_name, bridge_channel_name)
    -- VALUES (<bot_id>, 'tg-gateway', 'telegram:private:123456');
    ```
    验收：`bots` 查询至少 1 条 `runtime_type='nanobot'` 记录，`channel_bindings` 有与 `tg-gateway` 对应的绑定行，否则按上方模板插入后再继续 6.3.4
  - [x] 6.3.5 验证 Gateway → NanoBot 调用链路（需本地 NanoBot 进程运行，端口 8900）
    ```bash
    # 1. 确认 NanoBot 可正常响应
    curl -s http://localhost:8900/v1/chat/completions \
      -H "Content-Type: application/json" \
      -d '{"model":"deepseek-chat","messages":[{"role":"user","content":"ping"}],"session_id":"test-manual-1"}' \
      | jq .choices[0].message.content
    # 2. 启动 Gateway（cargo run）后，模拟 Bridge 调用 /gateway/inbound
    curl -s -X POST http://localhost:8080/gateway/inbound \
      -H "Content-Type: application/json" \
      -H "Authorization: Bearer $GATEWAY_BEARER_TOKEN" \
      -d '{
        "platform": "telegram",
        "bridge_gateway_name": "tg-gateway",
        "raw_message": {
          "chat_id": "123456",
          "chat_type": "private",
          "user_id": "user-001",
          "message_type": "text",
          "text": "ping",
          "timestamp": "2026-04-16T10:00:00Z",
          "message_id": "msg-manual-001"
        }
      }' | jq .
    # 3. 验证 Gateway 日志：可见 runtime_model 字段被读取，NanoBotAdapter 请求包含正确的 model/session_id，
    #    NanoBot 返回 choices[0].message.content，Gateway 日志输出 StandardReply { status: "success" }
    ```
    验收：Gateway 成功拿到 `StandardReply { status: "success" }`，Gateway 日志无 `RUNTIME_*` error_code
    > ⚠️ "Telegram 侧收到回复"属于 `feat-runtime-reply-bridge` 范围，不在本提案验收边界内

## 7. 验证与归档

- [x] 7.1 执行 specflow 验证：
  ```bash
  node design/context-dev/tools/specflow/specflow.mjs validate feat-runtime-nanobot-adapter --strict
  ```
- [x] 7.2 代码审查 Checklist
  - [x] 7.2.1 `model` 字段来自 `bots.runtime_model`，无硬编码 `"nanobot"` 字符串
  - [x] 7.2.2 生产路径无 `.unwrap()` / `.expect()`（使用 `?` + typed error）
  - [x] 7.2.3 日志无敏感凭证输出（`tracing` 结构化日志）
  - [x] 7.2.4 Goose 迁移文件包含 `-- +goose Up` 和 `-- +goose Down`
- [x] 7.3 合并后归档：
  ```bash
  node design/context-dev/tools/specflow/specflow.mjs archive feat-runtime-nanobot-adapter --yes
  ```
