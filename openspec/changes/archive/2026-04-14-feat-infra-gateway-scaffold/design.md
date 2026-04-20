## Context

`feat-infra-gateway-scaffold` 需要确定 Gateway Rust 项目的初始骨架设计：目录结构约定、HTTP 框架选型、环境变量管理方式。这些决策一旦固定，后续所有提案均在其基础上叠加，改动成本高，需在骨架阶段明确。

## Goals / Non-Goals

- Goals:
  - 固定三层目录结构（handlers/adapters/db/models/errors/config）
  - 固定 HTTP 框架（axum）与异步运行时（tokio）
  - 确定环境变量加载方式（启动时 panic-on-missing）
  - 明确 DATABASE_URL 骨架阶段"声明但不连接"策略

- Non-Goals:
  - DB 连接实现（由 `feat-gateway-db-layer` 负责）
  - Bearer Token 校验中间件（由 `feat-gateway-inbound-gate` 负责）
  - 结构化日志完整配置（由后续可观测性提案负责）
  - Docker Compose 集成（由 `feat-infra-deploy-compose` 负责）

## Decisions

- **Decision**: 采用 **axum** 作为 HTTP 框架（SHOULD，criterion.md §3.4），而非 actix-web
  - Alternatives considered: actix-web（更成熟的生产使用量）
  - 结论：axum 与 tokio 和 tower 中间件生态深度集成，后续添加 Bearer Token 中间件、限流中间件时更自然；actix-web 的 Actor 模型增加心智负担

- **Decision**: 环境变量采用 **panic-on-missing** 策略（`AppConfig::from_env()` 启动即失败）
  - Alternatives considered: 返回 `Result<AppConfig, ConfigError>` + 优雅关闭
  - 结论：骨架阶段快速失败是更安全的选择，防止带错误配置运行导致不一致状态；Docker Compose 容器快速退出后 `restart: on-failure` 配合 `.env` 文件可正常修复

- **Decision**: `DATABASE_URL` 在骨架阶段**声明但不连接**（连接池由 `feat-gateway-db-layer` 建立）
  - Alternatives considered: 骨架阶段尝试建立 DB 连接验证可达性
  - 结论：骨架提案聚焦"可编译 + /health"，引入 DB 连接会将 RISK-004（PostgreSQL 不可用熔断）带入骨架验证流程，提升骨架提案复杂度

- **Decision**: **SSoT 已对齐，无需变更**
  - `GET /health` 已在 `SSoT/api/main.tsp`（L147）中定义（`@route("/health") op health(): { status: "ok" }`）；本提案实现的返回结构须与 TypeSpec 契约保持一致，无需新增或修改 TypeSpec
  - 骨架阶段不建立 DB 连接，`SSoT/schema/migrations/` 无需新增迁移文件

## Risks / Trade-offs

- 选择 axum 意味着深度绑定 tokio 生态；若未来切换运行时成本较高（接受，tokio 是 MUST）
- `panic-on-missing` 在 Docker Compose 环境要求 `.env` 文件完整；需在 `feat-infra-deploy-compose` 提案中提供 `.env.example` 覆盖所有必要变量
- **RISK-B005 技术局限**：单 crate 下目录划分为约定，不是编译期约束；跨层 `use` 不会报错，只能通过 code review gate（PR 检查）和 clippy lint（可选）阻断。可接受理由：骨架阶段团队规模小，约定成本低；未来若越界引用发生多次，则提案将 `handlers`/`adapters`/`db` 拆成独立 workspace crate 以获得编译期强制

## Open Questions

- NanoBot 实际存储路径（`~/.nanobot/` vs `~/.local/state/nano-bots/`，RISK-007）适配逻辑：由后续 NanoBot 部署提案处理，骨架阶段不涉及
