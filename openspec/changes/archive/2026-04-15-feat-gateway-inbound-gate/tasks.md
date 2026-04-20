# 实施任务清单

> feat-gateway-inbound-gate — 入站网关（Bearer Token + 限流）。前置依赖 feat-gateway-db-layer 已完成（PgPool Extension、health_check() 就绪）。本变更不新增 Goose 迁移文件；SSoT/api/main.tsp 中 POST /gateway/inbound 端点已定义，本提案验证契约一致性。

## 0. SSoT 确认（SSoT-first）

- [x] 0.1 确认 `SSoT/api/main.tsp` 已包含 `POST /gateway/inbound` 端点、`InboundRequest` 与 `RawMessage` 模型定义；以 `SSoT/api/main.tsp` 为权威源，将 `.context/architecture/api_strategy.md:50-51` 中 `message_type` 枚举值（`text/other` → `"text"|"image"|"audio"|"video"|"file"|"other"`）和 `raw_message.text` 可选性（必选 → 可选）同步修订至与 SSoT 对齐
- [x] 0.2 确认本变更不需要新增 Goose 迁移（无 DB Schema 变更）
- [x] 0.3 运行 `make api-compile`（`tsp compile SSoT/api/`），验证 TypeSpec 文件编译通过，`tsp-output/@typespec/openapi3/openapi.yaml` 已生成
- [x] 0.4 运行 `make api-gen-rs` 验证工具链可用；评估后确认生成产物为完整 Rust 子 crate（过重），采用 RISK-007 回退方案：在 `gateway/src/models/inbound.rs` 手写 model structs，`gateway/src/generated/` 加入 `.gitignore`

## 1. Makefile + Codegen 配置修正（Roadmap Task 1）

- [x] 1.1 修正 `Makefile` 中 `api-gen-rs` target 的 `-o` 输出路径：将 `crates/openfleet-desktop/src-tauri/src/generated/api` 改为 `gateway/src/generated/`
- [x] 1.2 创建 `SSoT/api/openapi-generator-rs.yaml`
- [x] 1.3 创建 `SSoT/api/openapi-generator-rs.ignore`（空文件）
- [x] 1.4 在 `gateway/Cargo.toml` 中添加所需依赖：`serde`、`serde_json`（已有）、`constant_time_eq = "0.4"`、`async-trait = "0.1"`
- [x] 1.5 验证 `openapi-generator-cli` 已安装（7.21.0）
- [x] 1.6 运行 `make api-gen-rs`，验证 `gateway/src/generated/` 下生成 model structs

## 2. Cargo.toml 依赖确认

- [x] 2.1 确认 `gateway/Cargo.toml` 包含 `constant_time_eq`（`^1.0` 或最新稳定版）
- [x] 2.2 确认 `axum`、`serde`、`serde_json`、`tokio` 依赖已就绪（骨架阶段应已包含）
- [x] 2.3 运行 `cargo build`，确认编译通过

## 3. Rust 类型定义 + 集成到 Gateway（Roadmap Task 3）

> 实施时评估了 codegen 方案（RISK-007），采用回退方案：手写 model structs 直接内联于 `gateway/src/models/inbound.rs`。

- [x] 3.1 在 `gateway/src/models/inbound.rs` 手写 model structs，字段严格对齐 `SSoT/api/main.tsp`
  - [x] 3.1.1 定义 `InboundRequest`（platform、bridge_gateway_name、bridge_channel_name?、raw_message）
  - [x] 3.1.2 定义 `RawMessage` + `ChatType`（private/group）+ `MessageType`（text/image/audio/video/file/other）
  - [x] 3.1.3 文件顶部注释锚定 SSoT：`// SSoT: SSoT/api/main.tsp`
- [x] 3.2 在 `gateway/src/models/inbound.rs` 中定义：
  - [x] 3.2.1 `InboundResponse` + `InboundStatus` enum（accepted / ignored_duplicate），`#[serde(rename_all = "snake_case")]`
  - [x] 3.2.2 `pub struct ErrorResponse { pub error: String }` 派生 `serde::Serialize`
  - [x] 3.2.3 `ValidatedJson<T>` extractor：将 axum Json 422 改为 400，实现 `#[async_trait] FromRequest`
- [x] 3.3 更新 `gateway/src/models/mod.rs`，pub 导出 `inbound` 模块

## 4. Bearer Token Middleware（Roadmap Task 2）

- [x] 4.1 创建 `gateway/src/middleware/auth.rs`
  - [x] 4.1.1 实现 axum extractor：从 `Authorization` header 提取 `Bearer <token>`；使用 `constant_time_eq::constant_time_eq` 恒时比较
  - [x] 4.1.2 token 无效或 header 缺失时返回 HTTP 401 + `ErrorResponse { error: "Unauthorized" }` JSON body
  - [x] 4.1.3 tracing span / log 语句中不包含 `GATEWAY_BEARER_TOKEN` 明文值
- [x] 4.2 更新 `gateway/src/middleware/mod.rs`，pub 导出 `auth` 模块

## 5. Token Bucket 限流器（Roadmap Task 4）

- [x] 5.1 创建 `gateway/src/middleware/rate_limit.rs`
  - [x] 5.1.1 定义 `TokenBucket` struct
  - [x] 5.1.2 实现 `RateLimiter`：`Mutex<HashMap<String, TokenBucket>>`；LRU 清理
  - [x] 5.1.3 `RateLimiter` 注入 `InboundHandlerState`（`Arc` 共享）
- [x] 5.2 更新 `gateway/src/middleware/mod.rs`，pub 导出 `rate_limit` 模块

## 6. Inbound Handler（Roadmap Task 3 + 5）

- [x] 6.1 重写 `gateway/src/handlers/inbound.rs`
  - [x] 6.1.1 Handler 签名使用 `BearerAuth` extractor
  - [x] 6.1.2 限流检查（429）
  - [x] 6.1.3 DB 熔断检查（503）
  - [x] 6.1.4 非文本消息拦截（400 + "非文本消息类型，已忽略"）
  - [x] 6.1.5 `text` 为 None 时返回 400
  - [x] 6.1.6 空/空白文本返回 400
  - [x] 6.1.7 文本 > 4096 字符返回 400 + 用户提示 + INFO 日志
  - [x] 6.1.8 通过所有检查后返回 200 + `{ "status": "accepted" }`
- [x] 6.2 `gateway/src/handlers/mod.rs` 已导出 `inbound` 模块

## 7. 路由注册（Roadmap Task 5）

- [x] 7.1 在 `gateway/src/main.rs` 中注册路由：
  - [x] 7.1.1 `BearerTokenConfig` Extension 挂载（auth extractor 从中读取 token）
  - [x] 7.1.2 `RateLimiter`、`PgPool` 注入
  - [x] 7.1.3 运行 `cargo build` 无编译错误

## 8. 单元测试（Roadmap Task 5）

- [x] 8.1 Bearer Token middleware 单元测试：
  - [x] 8.1.1 无 Authorization header → 401
  - [x] 8.1.2 无效 token → 401
  - [x] 8.1.3 有效 token → 通过（next layer 被调用）
- [x] 8.2 Token Bucket 单元测试：
  - [x] 8.2.1 同一 chat_id 连续 5 次 allow() → 全返回 true
  - [x] 8.2.2 同一 chat_id 第 6 次 allow() → false
  - [x] 8.2.3 不同 chat_id 相互独立（A 超限不影响 B）
  - [x] 8.2.4 超过 60s 未活跃的键被 LRU 清理（100ms 快速验证）
- [x] 8.3 InboundRequest 反序列化单元测试：
  - [x] 8.3.1 完整合法 JSON → 反序列化成功（含 8.4.1 集成路径）
  - [x] 8.3.2 缺少 platform 字段 → 400（ValidatedJson 捕获 serde error）
  - [x] 8.3.3 非法 JSON → 400
- [x] 8.4 Handler 集成测试（`tower::ServiceExt`）：
  - [ ] 8.4.1 有效 token + 合法 text 消息 → 200 + `{ "status": "accepted" }`（需真实 DB；无 DB 时返回 503，已由 503 测试覆盖）
  - [x] 8.4.2 无 token → 401
  - [x] 8.4.3 超限（第 6 条）→ 429
  - [x] 8.4.4 缺 platform → 400
  - [x] 8.4.5 message_type = image → 400 + 忽略提示

## 9. 手动验证

- [x] 9.1 启动 Gateway（含有效 GATEWAY_BEARER_TOKEN 环境变量）
  - [x] 9.1.1 `curl -X POST /gateway/inbound`（无 Auth header）→ 401
  - [x] 9.1.2 `curl -X POST /gateway/inbound -H "Authorization: Bearer wrong"` → 401
  - [x] 9.1.3 合法请求（Bearer Token 正确，message_type=text，完整字段）→ 200
  - [x] 9.1.4 同一 chat_id 连发 6 条 → 第 6 条 429
  - [x] 9.1.5 message_type=image → 400 + 忽略提示
- [x] 9.2 确认日志中无 GATEWAY_BEARER_TOKEN 明文输出

## 10. 验证与归档

- [x] specflow validate feat-gateway-inbound-gate --strict（运行：`node design/context-dev/tools/specflow/specflow.mjs validate feat-gateway-inbound-gate --strict`）
- [x] specflow archive feat-gateway-inbound-gate --yes（运行：`node design/context-dev/tools/specflow/specflow.mjs archive feat-gateway-inbound-gate --yes`，由用户执行）
