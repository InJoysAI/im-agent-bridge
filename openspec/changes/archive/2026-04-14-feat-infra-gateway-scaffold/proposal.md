# Change: Gateway Rust 项目初始化（feat-infra-gateway-scaffold）

## Why

IM Agent Bridge 的 Core Layer 以 Gateway（Rust）为唯一入口，承担消息标准化、Session 管理、Runtime 调用与回写控制等全部业务逻辑。在任何业务提案实施之前，必须先建立 `gateway/` Rust 项目骨架并固定强制技术栈（axum / tokio / sqlx / serde / tracing），确保：

1. 项目可编译，后续所有提案均在同一骨架上叠加，避免后期重构。
2. 三层目录结构（`handlers/` / `adapters/` / `db/` / `models/` / `errors/` / `config.rs`）在约定层面降低越界引用概率（RISK-B005 缓解），配合 PR code review gate 执行边界检查。
3. 环境变量配置在启动时统一校验，缺失必要变量时快速失败，防止带错误配置运行。
4. `GET /health` 端点提供统一健康检查入口，供 Docker Compose 和后续监控使用。

此提案是 Phase 0 基础设施的第一个交付物，是 `feat-gateway-db-layer`、`feat-infra-deploy-compose` 等后续所有提案的前置条件。

### 路线图对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | `openspec/proposal-roadmap.md` |
| phase | Phase 0 |
| business_goal | 建立 gateway/ Rust 骨架，固定强制技术栈；项目可编译；GET /health → {"status":"ok"}；env var 启动校验 |
| dependencies | 无前置依赖 |
| acceptance_criteria | `cargo build` exit 0；`GET /health` → 200 + `{"status":"ok"}`；缺 DATABASE_URL 时启动失败并提示字段名 |

## What Changes

### 新增功能

- `gateway/Cargo.toml`：初始化 Rust 项目，锁定依赖：axum、sqlx（features: postgres, runtime-tokio-rustls, uuid, chrono）、reqwest（features: json）、tokio（features: full）、tracing、tracing-subscriber、serde、serde_json、uuid（features: v4）、dotenvy
- `gateway/src/main.rs`：axum 路由注册 + 启动逻辑，绑定 `0.0.0.0:8080`
- `gateway/src/config.rs`：从环境变量加载配置（GATEWAY_BEARER_TOKEN、DATABASE_URL、BRIDGE_URL、BRIDGE_BEARER_TOKEN），缺失必要变量时 `panic!` 并输出字段名
- `gateway/src/handlers/health.rs`：`GET /health` → `{"status":"ok"}`，HTTP 200，无需 Bearer Token
- 目录结构占位文件：`src/handlers/mod.rs`、`src/adapters/mod.rs`、`src/db/mod.rs`、`src/models/mod.rs`、`src/errors/mod.rs`

### 技术实现

- 使用 **axum**（SHOULD，HTTP 框架）+ **tokio**（MUST，异步运行时）
- 使用 **serde / serde_json**（MUST，JSON 序列化）
- 使用 **sqlx**（SHOULD，PostgreSQL 驱动）：骨架阶段声明依赖，不建立连接（连接池由 `feat-gateway-db-layer` 实现）
- 使用 **tracing**（SHOULD，结构化日志）：骨架阶段声明占位，具体配置由后续可观测性提案实现
- 三层目录边界约定：`handlers/`（入站路由）、`adapters/`（Runtime/Bridge 适配）、`db/`（数据库访问）；跨层引用通过 code review gate 阻断，未来规模扩展时拆分独立 crate 以获得编译期边界检查

## Impact

### 涉及的规范（Specs）

- **新增**：`specs/gateway-scaffold/spec.md` — Gateway Rust 项目骨架、`GET /health` 端点、环境变量配置加载

### 涉及的代码

- **新增**：
  - `gateway/Cargo.toml`
  - `gateway/src/main.rs`
  - `gateway/src/config.rs`
  - `gateway/src/handlers/mod.rs`
  - `gateway/src/handlers/health.rs`
  - `gateway/src/adapters/mod.rs`
  - `gateway/src/db/mod.rs`
  - `gateway/src/models/mod.rs`
  - `gateway/src/errors/mod.rs`

### 业务规则对齐

| 业务规则 | 规范来源 | 本提案如何体现 | 后续提案补齐 |
|---------|---------|--------------|-------------|
| **BR-020** 三层架构边界 | `criterion.md §3.1` | 三层目录结构约定（handlers/adapters/db），不同职责物理分离，code review gate 阻断跨层 `use` | 规模扩展时拆 crate 获得编译期检查 |
| **BR-021** Gateway 唯一入口 | `criterion.md §3.4` | 骨架仅暴露 HTTP 端口（`GET /health`），无旁路直连 Runtime 或 DB 的代码路径 | `feat-gateway-inbound-gate` 实现 Bearer Token 校验与 `POST /gateway/inbound` |
| **BR-030** 凭证保护 | `criterion.md §4` | `AppConfig::from_env()` 所有凭证从环境变量读取，无硬编码，缺失时快速失败 | — |

### SSoT 对齐

| SSoT 层 | 文件 | 状态 | 说明 |
|--------|------|------|------|
| API 层 | `SSoT/api/main.tsp` | ✅ 已对齐，无需变更 | `GET /health` 已在 L147 定义（`@route("/health") op health(): { status: "ok" }`），本提案实现须与契约一致 |
| 数据层 | `SSoT/schema/migrations/` | ✅ 无需变更 | 骨架阶段不建立 DB 连接，无新表或迁移 |

### 依赖关系

- **依赖**：无前置依赖
- **被依赖**：`feat-gateway-db-layer`（DB 连接层），`feat-infra-deploy-compose`（Docker Compose 编排）

### 风险与注意事项

- **RISK-B005**（后续需求扩展破坏三层骨架）：三层目录结构（handlers/adapters/db）在约定层面降低越界引用概率；单 crate 内目录无法技术强制，需配合 code review gate（PR 阶段检查跨层 `use`）；长期计划在规模扩展时拆分独立 crate 以实现编译期边界检查。
- **RISK-007**（TAD 设计与工具实际能力差距）：骨架阶段仅锁定依赖版本，具体适配层（如 Matterbridge push 桥接、MEMORY.md 配置映射）由后续提案实现，减少骨架变更成本。
- `DATABASE_URL` 在骨架阶段声明但不建立连接，避免骨架提案引入 DB 可用性依赖（RISK-004）。

### 验证标准

- ✅ `cargo build` exit 0（无编译错误）
- ✅ `GET /health` → HTTP 200 + `{"status":"ok"}`
- ✅ 缺少 `DATABASE_URL` 环境变量时，启动失败并输出包含字段名的错误信息

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.4 Core Layer Gateway MUST/MUST NOT 约束；§3.1 三层架构边界 |
| architecture | `.context/architecture/tech_stack.md` | Rust MUST、tokio MUST、serde MUST、axum SHOULD、sqlx SHOULD |
| architecture | `.context/architecture/system_design.md` | 三层架构组件说明，Gateway 作为 Core 唯一入口 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-007 工具能力差距；TD-002 无管理后台 |
| domain | `.context/domain/business_rules.md` | BR-020 三层架构边界；BR-021 Gateway 唯一入口；BR-030 凭证保护 |
