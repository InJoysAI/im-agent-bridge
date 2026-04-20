# 实施任务清单

> Phase 0 第一个交付物，无前置依赖。**SSoT 已对齐，无需变更**：`GET /health` 已在 `SSoT/api/main.tsp` L147 定义，实现需与契约保持一致；骨架阶段不建立 DB 连接，无新迁移文件。

## 0. SSoT 先行检查

- [x] 0.1 确认 `SSoT/api/main.tsp` L147 已定义 `@route("/health") op health(): { status: "ok" }`，实现返回结构与契约一致，**无需变更 TypeSpec**
- [x] 0.2 确认 `SSoT/schema/migrations/` 无需新增迁移文件（骨架阶段不建立 DB 连接，无表结构变更）

## 1. 初始化 Cargo.toml + 全部依赖（0.5 天）

- [x] 1.1 在项目根目录创建 `gateway/` 目录
- [x] 1.2 创建 `gateway/Cargo.toml`（package name: gateway）
- [x] 1.3 添加依赖（按 criterion.md §3.4 + tech_stack.md MUST/SHOULD）：
  - `axum`（HTTP 框架，SHOULD）
  - `tokio`（features = ["full"]，异步运行时，MUST）
  - `serde`（features = ["derive"]，JSON 序列化，MUST）
  - `serde_json`（MUST）
  - `sqlx`（features = ["postgres", "runtime-tokio-rustls", "uuid", "chrono"]，SHOULD；骨架阶段声明不连接）
  - `reqwest`（features = ["json"]，HTTP 客户端，SHOULD）
  - `tracing`（结构化日志，SHOULD）
  - `tracing-subscriber`（SHOULD）
  - `uuid`（features = ["v4"]）
  - `dotenvy`（开发环境 .env 加载）
- [x] 1.4 运行 `cargo build` 确认依赖解析成功（exit 0）

## 2. 建立目录结构，各模块占位（0.5 天）

- [x] 2.1 创建以下目录和占位 mod.rs 文件（内容为空模块声明）：
  - `gateway/src/handlers/mod.rs`
  - `gateway/src/adapters/mod.rs`
  - `gateway/src/db/mod.rs`
  - `gateway/src/models/mod.rs`
  - `gateway/src/errors/mod.rs`
- [x] 2.2 创建 `gateway/src/config.rs`（暂为空模块，Step 3 填充）
- [x] 2.3 在 `gateway/src/main.rs` 中声明各模块：
  ```rust
  mod handlers;
  mod adapters;
  mod db;
  mod models;
  mod errors;
  mod config;
  ```
- [x] 2.4 运行 `cargo build` 确认模块结构编译通过

## 3. 实现 config.rs 环境变量加载（0.25 天）

- [x] 3.1 在 `gateway/src/config.rs` 定义 `AppConfig` struct（字段：`gateway_bearer_token`、`database_url`、`bridge_url`、`bridge_bearer_token`）
- [x] 3.2 实现 `AppConfig::from_env()` 函数：使用 `std::env::var()` 读取各字段；缺失必要变量时 `panic!` 并在错误信息中明确字段名（如 `"missing env: DATABASE_URL"`）
- [x] 3.3 在 `main.rs` 入口处调用 `dotenvy::dotenv().ok();`（开发环境加载 `.env`，返回 `Err` 时忽略），随即调用 `AppConfig::from_env()`
- [x] 3.4 编写单元测试：模拟缺少 `DATABASE_URL` 时 `from_env()` 发生 panic（`#[should_panic]`）
- [x] 3.5 在 `.gitignore` 中确认 `.env` 已忽略（`dotenvy` 仅用于开发环境，生产环境通过容器环境变量注入）

## 4. 实现 GET /health 端点 + 编译验证（0.5 天）

- [x] 4.1 创建 `gateway/src/handlers/health.rs`，实现 handler：
  ```rust
  // 返回 HTTP 200 + {"status":"ok"}
  ```
- [x] 4.2 在 `gateway/src/handlers/mod.rs` 中暴露 health handler
- [x] 4.3 在 `main.rs` 中注册路由：`Router::new().route("/health", get(health_handler))`，绑定 `0.0.0.0:8080`
- [x] 4.4 运行 `cargo build` 确认编译成功（exit 0）

## 5. 测试

- [x] 5.1 单元测试
  - [x] 5.1.1 `AppConfig::from_env()` 正常加载（全部环境变量已设置）
  - [x] 5.1.2 `AppConfig::from_env()` 缺少 `DATABASE_URL` 时 panic，错误含字段名
- [x] 5.2 集成测试（可选基础）
  - [x] 5.2.1 使用 `axum::test` 或 `reqwest` 测试 `GET /health` 返回 HTTP 200 + `{"status":"ok"}`
- [x] 5.3 手动测试
  - [x] 5.3.1 `cargo build` exit 0
  - [x] 5.3.2 设置全部环境变量后启动，`curl http://localhost:8080/health` 返回 `{"status":"ok"}`
  - [x] 5.3.3 缺少 `DATABASE_URL` 启动时，终端输出含字段名的 panic 信息
  - [x] 5.3.4 开发环境创建 `.env` 文件含全部必要变量，应用通过 `dotenvy::dotenv().ok()` 加载该文件并正常启动

## 6. 验证与归档

- [x] 6.1 验证所有验收标准：
  - [x] 6.1.1 `cargo build` exit 0
  - [x] 6.1.2 `GET /health` → HTTP 200 + `{"status":"ok"}`
  - [x] 6.1.3 缺少 `DATABASE_URL` 时启动失败，错误信息含字段名
- [x] 6.2 代码审查
  - [x] 6.2.1 三层目录结构符合规范（handlers/adapters/db 无越界引用）
  - [x] 6.2.2 无硬编码凭证（BR-030），所有凭证从环境变量读取
- [x] 6.3 specflow validate：运行 `node design/context-dev/tools/specflow/specflow.mjs validate feat-infra-gateway-scaffold --strict`，确认所有检查通过
- [ ] 6.4 specflow archive：评审通过且实施完成后，运行 `node design/context-dev/tools/specflow/specflow.mjs archive feat-infra-gateway-scaffold --yes` 完成归档
