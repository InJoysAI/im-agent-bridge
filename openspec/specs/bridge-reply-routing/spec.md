# bridge-reply-routing Specification

## Purpose
TBD

## Requirements
### Requirement: 回写按来源 chat_id 定向路由（禁止 gateway 级广播）

系统必须（MUST）在 Matterbridge gateway 同时配置多个 `inout` 时，确保回写消息仅路由到与来源 `chat_id` 对应的 `inout`，不得广播至同一 gateway 下的其他 `inout`（BR-012 私聊/群聊上下文隔离，BR-010/BR-011 session_id 隔离语义）。

实现机制：E2E 验证确认 Matterbridge intra-gateway 路由不使用 POST payload 中的 `channel` 字段进行 inout 过滤。完整隔离需双重保障：① `matterbridge.toml` 将每个 Telegram chat 拆分为独立 gateway（各 gateway 仅含一个 telegram `inout` + 一个 api `inout`）；② `to_matterbridge_message` 将 `channel` 绑定为 `chat_id`，语义与 inout 配置保持一致。

#### Scenario: 群聊触发回复只投递到群聊

- **WHEN** 来源 `chat_id` 为群聊 ID（`chat_type = "group"`），Gateway 执行回写
- **THEN** `POST /api/message` 请求体中 `channel` 等于该群聊 `chat_id`
- **AND** 仅 `CBECOpsBot-group` gateway 的 telegram inout 收到该消息
- **AND** `CBECOpsBot-private` gateway 不收到该回复

#### Scenario: 私聊触发回复只投递到私聊

- **WHEN** 来源 `chat_id` 为私聊 ID（`chat_type = "private"`），Gateway 执行回写
- **THEN** `POST /api/message` 请求体中 `channel` 等于该私聊 `chat_id`
- **AND** 仅 `CBECOpsBot-private` gateway 的 telegram inout 收到该消息
- **AND** 群聊 `inout` 不收到该回复

---

## Gate 场景（Gherkin）

```gherkin
场景: 分独立 gateway 下回写定向路由
  Given matterbridge.toml 配置独立的 CBECOpsBot-group 和 CBECOpsBot-private gateway
  And 每个 gateway 仅含一个 telegram inout 与一个 api.myapi inout
  When 用户在群聊中 @bot 发送文本消息
  Then Gateway 调用 POST /api/message 时请求体包含 channel=群聊chat_id 和 gateway=CBECOpsBot-group
    And 群聊收到回复
    And 私聊不得收到该回复
```

### Requirement: Bridge 回写 HTTP 调用（wire payload 修正）

`openspec/specs/bridge-reply/spec.md` §"Bridge 回写 HTTP 调用" 中的 wire payload 说明更正如下：

**原文**（错误）：
> wire payload 为 `{gateway: bridge_gateway_name, text, username?}`

**修正后**：
> wire payload 为 `{gateway: bridge_gateway_name, channel: chat_id, text, username?}`

`channel` 字段必须（MUST）等于来源 `BridgeReplyPayload.chat_id`，禁止省略或使用固定值（如 `"api"`）。省略该字段将导致 Matterbridge 将消息广播至 gateway 下所有 `inout`，违反 BR-012 隔离语义。

#### Scenario: wire payload 包含 channel 字段

- **WHEN** Gateway 调用 `POST {BRIDGE_URL}/api/message` 执行回写
- **THEN** 请求体 JSON 包含 `"channel"` 字段，值等于 `BridgeReplyPayload.chat_id`
- **AND** 请求体同时包含 `"gateway"` 和 `"text"` 字段

---
