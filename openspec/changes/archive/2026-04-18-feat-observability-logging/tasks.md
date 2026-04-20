# 实施任务清单

> SSoT 未更改（无 DB schema 变更，无 API 契约变更）。不涉及新错误码。logging 为纯应用层横切关注点。

## 1. SSoT 先行确认
- [x] 1.1 确认无 DB schema 变更（`SSoT/schema/migrations/` 无需新增迁移）
- [x] 1.2 确认无 API 契约变更（`SSoT/api/main.tsp` 无需修改）

## 2. tracing-subscriber 依赖与初始化（0.25 天）
- [x] 2.1 在 `gateway/Cargo.toml` 添加依赖
  - `tracing-subscriber`（features: `env-filter`, `json`）
  - `tracing`（若尚未引入）
- [x] 2.2 创建 `gateway/src/observability/mod.rs`，实现 `init_subscriber()` 函数
  - 配置 `tracing_subscriber::fmt` 使用 `.json()` 格式化
  - 通过 `EnvFilter::from_default_env()` 读取 `RUST_LOG`，默认级别 `info`
- [x] 2.3 在 `gateway/src/main.rs` 替换现有日志初始化，调用 `observability::init_subscriber()`

## 3. 脱敏 Filter 层（0.25 天）
- [x] 3.1 在 `gateway/src/observability/sanitize.rs` 实现脱敏 layer
  - 定义 `SENSITIVE_FIELDS: &[&str]` 常量，覆盖所有高敏感凭证环境变量键名：
    `GATEWAY_BEARER_TOKEN`、`BRIDGE_BEARER_TOKEN`、`TELEGRAM_BOT_TOKEN`、
    `SHOPIFY_CLIENT_SECRET`（及各 MCP 实例等价变量）、`DATABASE_URL`（含密码部分）、`POSTGRES_PASSWORD`
  - 实现自定义 `tracing::Layer`，在事件序列化前将命中字段值替换为 `[REDACTED]`
  - `SENSITIVE_FIELDS` 必须集中定义，禁止在各模块分散硬编码
- [x] 3.2 将脱敏 layer 通过 `.with(SanitizeLayer)` 注册到 subscriber 链

## 4. 主链路 5 场景埋点（0.5 天）
- [x] 4.1 **入站消息到达**：在入站 handler 入口写 `INFO` event，字段：`event_id`、`platform`、`chat_id`、`chat_type`
- [x] 4.2 **Bearer Token 认证**：认证通过写 `INFO`，认证失败写 `WARN`，字段：`event_id`
- [x] 4.3 **Channel 解析（bot_id）**：命中写 `INFO`（含 `bot_id`），未命中写 `WARN`，字段：`event_id`
- [x] 4.4 **Runtime 调用**：调用发出写 `INFO`（含 `session_id`），返回后补充延迟（毫秒）；超时写 `ERROR`（含 `error_code: RUNTIME_TIMEOUT`），字段：`event_id`
- [x] 4.5 **回写结果**：成功写 `INFO`（含 `reply_id`），失败写 `WARN`/`ERROR`，字段：`event_id`
- [x] 4.6 **跨切面补充**（cross_cutting_concepts.md §11.1 剩余 4 类事件）：审查并改造现存相关代码，确保使用结构化 `tracing::warn!` / `tracing::error!`，字段含 `event_id`
  - **被限流请求**：现存限流逻辑改为 `tracing::warn!`，字段：`chat_id`、`event_id`、时间戳
  - **DB 不可用**：熔断触发改为 `tracing::error!`，字段：`event_id`
  - **标准化完成**：消息标准化后补写 `tracing::debug!`，字段：`event_id`、`session_id`
  - **session_id 生成/命中**：首次生成写 `INFO`，命中写 `INFO`，字段：`event_id`、`session_id`

## 5. 测试
- [x] 5.1 单元测试：脱敏 filter
  - [x] 5.1.1 含 Bearer Token 值的日志事件经 layer 处理后输出不含原始 token
  - [x] 5.1.2 非敏感字段不受脱敏影响
- [x] 5.2 单元测试：JSON 格式校验
  - [x] 5.2.1 `init_subscriber()` 初始化后写一条 INFO log，捕获输出后 `serde_json::from_str` 解析成功
- [x] 5.3 手动测试
  - [x] 5.3.1 本地启动 Gateway，发送测试消息，确认日志为 JSON 且含 `event_id`
  - [x] 5.3.2 检查输出中无 Bearer Token 明文

## 6. 验证与归档
- [x] 6.1 `node design/context-dev/tools/specflow/specflow.mjs validate feat-observability-logging --strict`
- [x] 6.2 代码审查：Gateway 侧 8 类 TAD §11.1 必选事件全覆盖（含 4.6 跨切面补充）/ `SENSITIVE_FIELDS` 包含全部高敏感键名 / 无敏感字段硬编码分散 / `event_id` 在所有场景一致透传
- [x] 6.3 `node design/context-dev/tools/specflow/specflow.mjs archive feat-observability-logging --yes`
