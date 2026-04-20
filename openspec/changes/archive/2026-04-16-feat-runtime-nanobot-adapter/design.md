## Context

NanoBotAdapter 需要连接两个领域：Gateway 内部的标准消息模型（`StandardMessage`）与 NanoBot 的 OpenAI-style `/v1/chat/completions` 接口。核心设计挑战是正确处理 NanoBot 的协议约束（`session_id` 必传、`messages` 严格 1 条、不传 `stream`、`model` 必须与服务端配置一致），以及 `model` 值的存储与读取方案。

## Goals / Non-Goals
- Goals:
  - 通过 `RuntimeAdapter` trait 保留 Runtime 可替换性（BR-022）
  - 从 `bots.runtime_model` 读取 `model` 值，支持不同 Bot 实例独立配置 model_name
  - 严格遵循 NanoBot 协议约束，避免触发 HTTP 400（RISK-007）
- Non-Goals:
  - MCP 选择/路由（Gateway MUST NOT 介入，由 NanoBot 自主选择，criterion.md §3.4）
  - Gateway → Runtime 认证（已知技术债 TD-001，MVP Out 范围）
  - Streaming 支持（NanoBot 不支持 `stream: true`）

## Decisions
- **Decision**: `model` 字段存储选方案 A（`bots.runtime_model` 数据库列），不选方案 B（全局环境变量）。
  - **Alternatives considered**: 方案 B（`NANOBOT_MODEL_NAME` 环境变量）简单但无法支持多 Bot 实例使用不同 model_name 的未来需求；方案 A 通过 `NOT NULL DEFAULT 'nanobot'` 保证向后兼容，Expand-Contract 迁移安全，且与 `bots.runtime_endpoint` / `bots.runtime_type` 的设计对称一致。
- **Decision**: `NanoBotAdapter` 内部持有 `reqwest::Client`（连接池复用），在 Gateway 启动时创建并注入，不在每次调用时新建。
  - **Alternatives considered**: 每次新建 `reqwest::Client` 开销大，且无法复用连接池；注入方式与 Rust 所有权模型契合，便于测试时替换 mock client。

## Risks / Trade-offs
- NanoBot 内部 `session_id` 存储格式为 `api:{session_id}`（如 `api:telegram:private:123456`），Gateway 传原始 `session_id` 值即可；若未来 NanoBot 修改此行为，需在 Adapter 层更新映射。
- `RUNTIME_SESSION_NOT_FOUND` 映射依赖 NanoBot 错误响应结构；NanoBot 未明确文档化此错误码触发条件，需实测确认（见 Open Questions）。
- `reqwest::Client` 持有连接池，需确保 Gateway 的 `tokio` 异步运行时与 `reqwest` 版本兼容。

## Migration Plan
- `00004_bots_runtime_model.sql` Up 为纯 `ALTER TABLE ADD COLUMN`（`TEXT NOT NULL DEFAULT 'nanobot'`），符合 Expand-Contract 安全变更规范，现有数据行无需手动回填。
- Down 为 `ALTER TABLE DROP COLUMN`，仅开发/回滚场景使用。

## Open Questions
- NanoBot 在何种条件下返回 session-not-found 类型错误（HTTP 状态码？错误响应结构？）？需本地实测确认，以正确实现 `RUNTIME_SESSION_NOT_FOUND` 映射逻辑。
