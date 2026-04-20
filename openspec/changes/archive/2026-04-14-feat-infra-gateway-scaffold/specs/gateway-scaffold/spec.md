## ADDED Requirements

### Requirement: Gateway Rust 项目骨架初始化

> **业务规则对齐**: BR-020（三层架构边界）、BR-021（Gateway 唯一入口）

系统必须（MUST）在 `gateway/` 目录下创建 Rust 项目，使用 axum + tokio + sqlx + serde + tracing 技术栈（criterion.md §3.4），并建立三层目录结构（handlers/adapters/db/models/errors/config），以约定层面降低各层之间越界引用概率（RISK-B005 缓解，配合 PR code review gate）。

#### Scenario: 项目编译成功
- **WHEN** 在 `gateway/` 目录运行 `cargo build`
- **THEN** 编译进程以 exit code 0 退出，无编译错误或 panic

#### Scenario: 三层目录结构完整
- **WHEN** 检查 `gateway/src/` 目录结构
- **THEN** 存在以下子目录：`handlers/`、`adapters/`、`db/`、`models/`、`errors/`，以及顶层文件 `config.rs`
- **AND** 每个子目录包含 `mod.rs` 占位文件

---

### Requirement: GET /health 健康检查端点

> **业务规则对齐**: BR-021（Gateway 是 Core 层对外唯一入口，健康检查端点均由 Gateway 提供）、**SSoT 已对齐**（`SSoT/api/main.tsp` L147）

系统必须（MUST）在 Gateway 服务上暴露 `GET /health` 端点，返回 HTTP 200 和 JSON 响应 `{"status":"ok"}`，供 Docker Compose 健康检查和运维监控使用，且该端点无需 Bearer Token 认证。

#### Scenario: 健康检查正常响应
- **WHEN** 向已启动的 Gateway 服务发送 `GET /health` 请求
- **THEN** 返回 HTTP 200
- **AND** 响应体为 `{"status":"ok"}`

#### Scenario: 健康检查无需认证
- **WHEN** 向 Gateway 发送不携带 `Authorization` 头的 `GET /health` 请求
- **THEN** 返回 HTTP 200（health 端点不受 Bearer Token 中间件拦截）

---

### Requirement: 环境变量配置加载与启动校验

> **业务规则对齐**: BR-030（凭证保护：禁止硬编码，通过环境变量注入）
> **待完善**: Bearer Token 校验逻辑（`GATEWAY_BEARER_TOKEN`）由 `feat-gateway-inbound-gate` 实现；骨架阶段仅校验变量存在性

系统必须（MUST）在启动时通过 `config.rs` 中的 `AppConfig::from_env()` 统一加载必要环境变量（GATEWAY_BEARER_TOKEN、DATABASE_URL、BRIDGE_URL、BRIDGE_BEARER_TOKEN），缺失任一必要变量时进程快速失败并输出包含缺失字段名的错误信息（BR-030 凭证保护）。

#### Scenario: 全部环境变量已配置时正常启动
- **WHEN** 已设置全部必要环境变量后启动 Gateway 服务
- **THEN** 服务正常启动，`GET /health` 响应 HTTP 200

#### Scenario: 缺少必要环境变量时启动失败
- **WHEN** 未设置 `DATABASE_URL` 环境变量时启动 Gateway 服务
- **THEN** 进程以非零 exit code 退出
- **AND** 错误输出中包含缺失字段名（如 `DATABASE_URL`）
