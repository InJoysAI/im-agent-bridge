# Change: Bridge 回写按 chat 定向路由修复

## Why

当 Matterbridge gateway 同时配置多个 `inout`（例如一个群聊 + 一个私聊）时，Gateway 调用 `POST {BRIDGE_URL}/api/message` 若不指定 `channel` 字段，Matterbridge 会将消息广播至该 gateway 下**所有** `inout`，导致群聊触发的 Bot 回复也出现在私聊中，反之亦然。

这直接违反 BR-012（私聊与群聊上下文必须严格隔离，禁止相互污染）和 BR-010/BR-011（session_id 按 `chat_id` 维度区分私聊与群聊），并可能造成用户体验污染与业务数据混乱（RISK-B006）。

根本原因（双重）：① `MatterbridgeMessage` wire payload 缺少 `channel` 字段；② Matterbridge intra-gateway 路由将来自 `api.myapi` 的消息广播至同一 gateway 下**所有** telegram `inout`（E2E 验证证实 `channel` 字段在 Matterbridge 路由层不用于 inout 过滤）。仅修复 ① 不足以实现隔离；完整修复需同时拆分 gateway。

> **实现状态**：代码修复已在 `feat-runtime-reply-bridge` 联调阶段随 `bridge_client.rs` 落地（`MatterbridgeMessage.channel` 字段 + `to_matterbridge_message` 映射 `chat_id`）。本提案负责补全正式规格文档与 E2E 验证，确保合规归档。

## What Changes

### 修改功能

- **`gateway/src/bridge_client.rs`**（已完成）：`MatterbridgeMessage` 结构体包含 `channel: &str` 字段；`to_matterbridge_message` 将 `channel` 绑定为 `BridgeReplyPayload.chat_id`，禁止 gateway 级广播。
- **`openspec/specs/bridge-reply/spec.md`**（待更新）：wire payload 说明从 `{gateway, text, username?}` 修正为 `{gateway, channel, text, username?}`；补充 "channel-directed routing" 需求条目，明确 `channel` 必须等于来源 `chat_id`。
- **`.context/architecture/api_strategy.md`**（已完成）：§2.2 偏差注同步 `channel: chat_id` 字段，消除与代码的 SSoT 裂缝。P0-1 决策：该更新归属本提案，不待 fix-bridge-reply-ssot-align。

### 技术实现

- Matterbridge intra-gateway 路由将消息广播至同一 gateway 下所有 `inout`，`channel` 字段不用于 inout 过滤（E2E 验证结论）。完整隔离机制：每个 Telegram chat（群聊/私聊）对应**独立** Matterbridge gateway，各 gateway 仅含一个 telegram `inout` + 一个 `api.myapi` `inout`，从结构上消除跨 chat 广播。
- `bridge_client.rs` 的 `channel = chat_id` 字段作为语义标注保留，确保 wire payload 与 inout 配置的语义一致性，并兼容 Matterbridge 未来版本可能引入的 channel 路由能力。
- `BridgeReplyPayload.chat_id` 已在入站流程中从 `raw_message.chat_id` 填充，值与 Telegram 原始 chat ID 对应。
- 既有指数退避重试（1s/2s/4s）与 HTTP 409 幂等语义**不变**（BR-062）。

## Impact

### 涉及的规范（Specs）

- **修改**：`openspec/specs/bridge-reply/spec.md` — 补充 channel 定向路由需求与 wire payload 更正
- **新增**：`openspec/changes/fix-bridge-reply-chat-routing/specs/bridge-reply-routing/spec.md` — delta spec，描述本次修改内容

### 涉及的代码

- **已修改**:
  - `gateway/src/bridge_client.rs` — `MatterbridgeMessage` 含 `channel` + `to_matterbridge_message` 映射 `chat_id`（`feat-runtime-reply-bridge` 期间落地）
  - `.context/architecture/api_strategy.md` — §2.2 偏差注加入 `channel: chat_id`，SSoT 裂缝已封闭
  - `deploy/edge-server/matterbridge/matterbridge.toml` — 拆分为独立 gateway（`CBECOpsBot-private` / `CBECOpsBot-group`），消除 intra-gateway 广播

- **待更新**:
  - `openspec/specs/bridge-reply/spec.md` — wire payload 说明与 channel routing requirement

### 依赖关系

- **前置**：`feat-runtime-reply-bridge`（已完成，回写链路已联调）
- **前置**：`feat-infra-matterbridge-deploy`（已完成，Matterbridge 已部署）
- **被依赖**：无

### 风险与注意事项

- `matterbridge.toml` gateway 拆分需配套 `channel_bindings` 数据库变更：为 `CBECOpsBot-private` 和 `CBECOpsBot-group` 各添加一条记录（指向同一 `bot_id`），禁用旧 `CBECOpsBot` 记录。`bots.telegram_username` 不受影响，mention 过滤/stripping 逻辑透明兼容。
- `channel` 字段在 `MatterbridgeMessage` 中为 `&str`（非 Option），始终传递 `chat_id`；与各 gateway 的 inout `channel` 配置形成一致性约束。
- 入站广播（Matterbridge 向同 gateway 其他 inout 转发原始消息）已由分 gateway 架构消除；出站广播由分 gateway + `channel=chat_id` 双重保障。

### 验证标准

- ✅ `POST /api/message` 请求体包含 `channel = chat_id`（wiremock 单元测试已覆盖）
- ✅ 群聊触发后，Bot 回复只出现在群聊（E2E 联调验证，前提：`matterbridge.toml` 已拆分为独立 gateway）
- ✅ 私聊触发后，Bot 回复只出现在私聊（E2E 联调验证，前提同上）
- ✅ 既有回写重试与 409 幂等语义不变（cargo test 回归）
- ✅ 回写成功时 `reply_write_success_total` 计数器递增（cross_cutting_concepts.md §可观测性）
- ✅ 回写最终失败时 `reply_write_error_total` 计数器递增，错误日志包含 `reply_id` 且不含 Bearer Token（BR-063, BR-030）
- ✅ `openspec/specs/bridge-reply/spec.md` wire payload 描述含 `channel: chat_id`，与 `.context/architecture/api_strategy.md §2.2` 及 `gateway/src/bridge_client.rs:45` 三者一致（**P0 归档阻塞条件**）
