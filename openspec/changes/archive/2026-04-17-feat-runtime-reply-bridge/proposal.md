# Change: Bridge 回写链路

## Why

当前 Gateway 完成 Runtime 调用并生成回复文本后，缺少将回复回写到 Bridge（Matterbridge）的实现。`feat-runtime-nanobot-adapter` 已完成 Gateway → NanoBot 的请求/响应链路，但处理结果尚未通过 Bridge 回写到 Telegram。

本变更实现 Gateway 的回写客户端模块 `bridge_client.rs`，完成消息链路最后一跳：

- 通过 HTTP POST 将 AI 回复发送至 Matterbridge，再由 Matterbridge 转发至 Telegram
- 实现 3 次指数退避重试（1s/2s/4s），HTTP 409 视为成功（幂等语义）
- 最终失败时标记 `message_events.reply_status = reply_failed`，并记录错误日志
- 落库前将 `output_text` 截断至 512 字符（BR-070 数据最小化）

**业务价值**：用户可以收到 AI Agent 的回复消息，形成完整的对话闭环（Telegram → Matterbridge → Gateway → NanoBot → Matterbridge → Telegram）。

## 实施偏差说明（Implementation Deviation — 2026-04-17）

联调阶段发现原设计路径不可直接落地，经在当前 change 范围内做出以下受控偏差；此节记录已落地现状，相关 SSoT 对齐留给后续独立 change 处理：

1. **取消 `mb-adapter` 中间代理**：`feat-infra-matterbridge-deploy` 原计划引入独立的 `mb-adapter` 服务，对外暴露 `POST /bridge/reply` 并转译为 Matterbridge 原生协议。联调时确认 Matterbridge 1.26 `BindAddress` 在私网内直接可达，且没有足够业务复杂度去证明中间层。因此直接由 Gateway 调用 Matterbridge 原生 API。
2. **回写实际端点**：`POST {BRIDGE_URL}/api/message`（Matterbridge 1.26 官方消息投递端点）。`SSoT/api/main.tsp` 声明的 `POST /bridge/reply` 仍代表 Gateway 在 Bridge 代理层恢复时的内部契约接口，但当前没有任何组件实现该端点。
3. **Wire payload**：`{gateway, text, username?}`（Matterbridge `config.Message` 子集）。Gateway 内部仍保留 `BridgeReplyPayload` 结构（含 reply_id / chat_id / platform / bridge_gateway_name / bridge_channel_name）用于日志追踪与未来代理层恢复，发送前由 `to_matterbridge_message` 做字段映射。
4. **幂等保证弱化**：Matterbridge 1.26 `/api/message` 不识别 `reply_id`，重试在理论上可能导致 Telegram 收到重复消息。当前通过"仅对真正可重试错误（5xx/429/网络超时）重试 + Matterbridge 1.26 内部 Buffer=1000 限流"共同将重复投递控制在罕见场景（验收中未观测到重复）。
5. **后续 SSoT 对齐建议**：开立新 change（例如 `fix-bridge-reply-ssot-align`）选择下述之一：
   - (A) 将 `SSoT/api/main.tsp` 的 `Bridge.reply` 声明改为描述 Matterbridge 1.26 原生外部契约；或
   - (B) 重新引入 `mb-adapter` 服务以实现 SSoT 所声明的 `/bridge/reply` 端点。

## What Changes

### 新增功能

- `gateway/src/bridge_client.rs`：Bridge 回写 HTTP 客户端
  - `BridgeReplyPayload` 结构体：保留 SSoT `ReplyRequest` 字段命名（reply_id/chat_id/platform/text/bridge_gateway_name/bridge_channel_name?）用于日志追踪与未来代理层恢复
  - `MatterbridgeMessage` 内部结构 + `to_matterbridge_message` 映射函数：把内部 payload 映射为 Matterbridge `config.Message` 子集 `{gateway, text, username?}`
  - `post_reply` 异步函数：`POST {BRIDGE_URL}/api/message` + `Authorization: Bearer <BRIDGE_BEARER_TOKEN>`（见"实施偏差说明" §2）
  - 3 次指数退避重试逻辑（1s → 2s → 4s，使用 `tokio::time::sleep`）
  - HTTP 2xx → 视为成功；HTTP 409 → 视为幂等成功（代理层恢复后才会实际触发；Matterbridge 1.26 不返回 409，但保留该分支防御性处理）
  - HTTP 400 / 401 → 立即 `Err(NonRetryable)`，不重试
  - 其他 4xx / 5xx / 网络错误 / 超时 → 可重试；4 次尝试全部失败 → `Err(RetriesExhausted)`
- `truncate_to_512(text: &str) -> String`：output_text 落库前 512 字符截断函数
- `enforce_bridge_text_limit(text: &str) -> String`：bridge_client 入口兜底 4096 字符截断（BR-003）

### 修改功能

- `gateway/src/main.rs`（或消息处理 handler）：在 RuntimeAdapter 返回回复后，调用 `bridge_client::post_reply`，并根据结果更新 `message_events.reply_status`（`success` 或 `reply_failed`）；落库 output_text 前调用截断函数

### 技术实现

- `BRIDGE_URL` 和 `BRIDGE_BEARER_TOKEN` 已由 `feat-infra-gateway-scaffold` 的 `config.rs` 从环境变量声明，本变更直接引用
- 使用 `reqwest` 异步 HTTP 客户端（SHOULD 级依赖，已在 Gateway Cargo.toml 中）
- 重试延时：`tokio::time::sleep(Duration::from_secs(n))`，n ∈ {1, 2, 4}（初始调用 + 3 次重试 = 最多 4 次总尝试）
- **重试分类**：仅对可重试错误（网络 / 超时 / HTTP 5xx / 429）重试；不可重试错误（HTTP 400 / 401）立即失败不重试
- **安全脱敏**：日志均不得包含 `Authorization` 头值或 `BRIDGE_BEARER_TOKEN`（RISK-006）
- **双截断边界**：回写前 text ≤ 4096 字符（BR-003，RuntimeAdapter 已截断，本变更 bridge_client 入口兜底）；落库 output_text ≤ 512 字符（BR-070）
- **SSoT 当前偏差状态**：Gateway 内部 `BridgeReplyPayload` 字段仍以 `SSoT/api/main.tsp` `ReplyRequest` 为源头命名；实际 wire 端点 `/api/message` 与 payload 形状 `{gateway, text, username?}` 来源于 Matterbridge 1.26 外部协议，目前未纳入 SSoT。`.context/architecture/api_strategy.md` 中额外列举的 404/500/502 为历史说明，本变更不依赖。SSoT 对齐留给独立后续 change 处理（见"实施偏差说明" §5）

## Impact

### 涉及的规范（Specs）

- **新增**：`specs/bridge-reply/spec.md` — Bridge 回写链路行为规范（HTTP 调用、指数退避重试、幂等语义、reply_status 更新、output_text 截断）

### 涉及的代码

- **新增**：
  - `gateway/src/bridge_client.rs`

- **修改**：
  - `gateway/src/main.rs`（或消息处理 handler）— 调用 bridge_client 并更新 reply_status
  - `gateway/src/config.rs`（如需补充 BRIDGE_BEARER_TOKEN 引用，视 scaffold 实现情况而定）

### 依赖关系

- **依赖**：`feat-runtime-nanobot-adapter`（已归档）— 提供 Gateway → NanoBot 处理链路，本变更在其输出后接入
- **被依赖**：`feat-e2e-integration-test` — 端到端集成测试需要完整消息链路（含回写）

### 风险与注意事项

- **RISK-005**（Matterbridge 桥接稳定性）：Matterbridge 崩溃或 Telegram 连接断开时，4 次尝试（初始 + 3 次重试）仍会失败，最终标记 `reply_failed`；需通过 `reply_write_error_total` 指标监控
- **回写失败用户无感知**：`reply_failed` 为后台状态，MVP 阶段用户不会收到"投递失败"提示（架构约束，可接受）
- **截断双边界**：output_text 有两个截断点（4096 字符面向 Telegram，512 字符面向 DB），实现时需严格区分职责
- **At-most-once 投递（联调新增）**：Matterbridge 1.26 `/api/message` 无 `reply_id` 识别，重试在理论上可能导致 Telegram 重复消息；当前通过"仅对可重试错误（5xx/429/transport 错误）重试"把重复概率压到极低。若未来观测到重复，应回补 `mb-adapter` 代理层或在 Matterbridge 侧引入 dedup

### 验证标准

- ✅ Bridge 回写遇到可重试错误时经初始调用 + 3 次重试（共 4 次尝试，等待 1s/2s/4s）均失败后，`message_events.reply_status = "reply_failed"` 并记录错误日志
- ✅ Bridge 回写遇到不可重试错误（HTTP 400/401）时立即标记 `reply_failed`，不触发重试
- ✅ Bridge 返回 HTTP 409 时，`message_events.reply_status = "success"`（幂等视成功）
- ✅ Bridge 回写成功（HTTP 200）时，`message_events.reply_status = "success"`
- ✅ `message_events.output_text` 超 512 字符时，落库内容截断至 512 字符

### 提案大纲对齐（Roadmap Alignment）

| 字段 | 内容 |
|------|------|
| roadmap_source_primary | openspec/proposal-roadmap.md |
| roadmap_source_supplement | N/A |
| phase | Phase 2 |
| business_goal | POST /bridge/reply 调用（Bearer Token，reply_id 幂等）；对可重试错误执行 3 次指数退避重试（1s/2s/4s，初始 + 3 次重试 = 4 次尝试），409 视为成功；4 次尝试均失败或遇不可重试错误（400/401）后标记 reply_failed，更新 message_events.reply_status |
| dependencies | 前置: feat-runtime-nanobot-adapter（已归档）；被依赖: feat-e2e-integration-test |
| acceptance_criteria | 4 次尝试耗尽或不可重试错误立即失败 → reply_failed；409 → success；回写成功 → reply_status=success；output_text 超 512 字符截断落库 |

### 关联 Context 资产

| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.4 Gateway MUST 回写；§3.5 回复 4096 截断；§4 Bearer Token + 数据治理；BR-062/063/070/030/003 |
| architecture | `.context/architecture/api_strategy.md` | §2 POST /bridge/reply 接口规范；§2.5 幂等策略（reply_id + 409）【以 SSoT 为准，文档中 404/500/502 为历史说明】 |
| architecture | `.context/architecture/cross_cutting_concepts.md` | 回写指数退避 1s/2s/4s；指标 `reply_write_success_total` / `reply_write_error_total`；日志关键字段 |
| architecture | `.context/architecture/security_policy.md` | Bearer Token 从 env 注入、禁止日志记录；私网 + 最小权限 |
| architecture | `.context/architecture/risks_and_debt.md` | RISK-005 Matterbridge 稳定性；RISK-006 Bearer Token 泄露；TD-007 无 TLS（MVP 私网可接受） |
| domain | `.context/domain/business_rules.md` | BR-062 回写失败重试；BR-070 数据最小化；BR-063 错误可见性；BR-030 凭证保护；BR-003 回复 4096；BR-031 通信安全；BR-042 幂等 |
| domain | `.context/domain/edge_cases.md` | 回写异常用户无感知、未授权注入边界 |
| db | `.context/db/schema_design.md` | message_events.reply_status 枚举（success/reply_failed）；reply_error_code / reply_error_message 可选字段 |
| db | `.context/db/migrations_and_ssot.md` | SSoT-first 检查口径（本变更不触发迁移） |
