# 实施任务清单

> **Change**: `fix-bridge-reply-chat-routing` | **优先级**: P0 | **预计时间**: 0.5 天
>
> 代码修复（`MatterbridgeMessage.channel = chat_id`）已在 `feat-runtime-reply-bridge` 期间完成。
> 本清单重点为：单元测试回归确认、规格文档更新、E2E 验证、以及 OpenSpec 流程收尾。

## 1. 确认现有代码实现

- [x] 1.1 确认 `gateway/src/bridge_client.rs` 中 `MatterbridgeMessage` 含 `channel: &str` 字段
  - 字段位于 struct 定义（`gateway`, `channel`, `text`, `username?`）
- [x] 1.2 确认 `to_matterbridge_message` 将 `channel` 绑定为 `BridgeReplyPayload.chat_id`
  - 禁止使用固定值如 `"api"`；必须取 `&payload.chat_id`
- [x] 1.3 确认 `MatterbridgeMessage` 上方注释说明禁止 gateway 级广播的原因

## 2. 单元测试确认与补充

- [x] 2.1 确认 `http_200_returns_ok` wiremock 测试已断言请求体包含 `"channel": chat_id`
  - 验证命令：`cargo test -p gateway bridge_client::tests::http_200_returns_ok`
- [x] 2.2 运行全量 `bridge_client` 测试回归，确认既有重试与幂等语义不变
  - 验证命令：`cargo test -p gateway bridge_client`
- [x] 2.3 补充隔离场景单元测试（群聊 chat_id vs 私聊 chat_id 分别产生不同 channel 值）
  - 测试名建议：`channel_in_payload_equals_chat_id_for_group` / `channel_in_payload_equals_chat_id_for_private`
  - 断言：group `chat_id = "-100123"` → `channel = "-100123"`；private `chat_id = "456"` → `channel = "456"`

## 3. 规格文档更新

- [x] 3.0 更新 `.context/architecture/api_strategy.md` §2.2 偏差注（P0-1 决策：归属本提案）
  - [x] Wire payload 修正为 `{gateway, channel: chat_id, text, username?}`
  - [x] 新增 `channel` MUST 等于来源 `chat_id` 的约束说明
- [x] 3.1 **[P0-归档阻塞]** 更新 `openspec/specs/bridge-reply/spec.md` §"Bridge 回写 HTTP 调用"
  - [x] wire payload 修正为 `{gateway, channel: chat_id, text, username?}`；注明 `channel` MUST 等于 `chat_id`，禁止省略
  - 验收口径：`spec.md:10` 描述与 `api_strategy.md §2.2` 及 `bridge_client.rs:45` 三处一致
- [x] 3.2 在 `openspec/specs/bridge-reply/spec.md` 新增 "channel-directed routing" Requirement 条目
  - [x] 新增群聊/私聊定向 Scenario（双 inout 环境下互不干扰）

## 3.5 Matterbridge 配置与 DB 迁移

- [x] 3.5.1 更新 `deploy/edge-server/matterbridge/matterbridge.toml`：拆分为 `CBECOpsBot-private` 和 `CBECOpsBot-group` 两个独立 gateway
  - 每个 gateway 仅含一个 telegram inout + 一个 `api.myapi` inout
  - 消除 intra-gateway 广播（E2E 验证证实的根本隔离机制）
- [x] 3.5.2 手动执行 `channel_bindings` 数据库迁移
  - 为 `bridge_gateway_name = 'CBECOpsBot-private'` 添加绑定记录（指向原 `CBECOpsBot` 的同一 `bot_id`）
  - 为 `bridge_gateway_name = 'CBECOpsBot-group'` 添加绑定记录（同上）
  - 禁用旧 `bridge_gateway_name = 'CBECOpsBot'` 记录（`is_enabled = false`）
  - `bots.telegram_username` 不变，mention 过滤/stripping 不受影响
  ```sql
  -- 为新 gateway name 添加绑定，bot_id 与原 CBECOpsBot 一致
  INSERT INTO channel_bindings (id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled, created_at, updated_at)
  SELECT gen_random_uuid(), bot_id, platform, 'CBECOpsBot-private', bridge_channel_name, is_enabled, NOW(), NOW()
  FROM channel_bindings
  WHERE bridge_gateway_name = 'CBECOpsBot' AND is_enabled = true;

  INSERT INTO channel_bindings (id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled, created_at, updated_at)
  SELECT gen_random_uuid(), bot_id, platform, 'CBECOpsBot-group', bridge_channel_name, is_enabled, NOW(), NOW()
  FROM channel_bindings
  WHERE bridge_gateway_name = 'CBECOpsBot' AND is_enabled = true;

  -- 禁用旧记录
  UPDATE channel_bindings SET is_enabled = false, updated_at = NOW()
  WHERE bridge_gateway_name = 'CBECOpsBot';  
  ```

## 4. E2E 联调验证

- [x] 4.1 确认测试环境：`deploy/edge-server/matterbridge/matterbridge.toml` 已拆分为两个独立 gateway
  - `CBECOpsBot-group`：含 `telegram.mytelegram / ${TELEGRAM_CHAT_ID_GROUP}` + `api.myapi / api`
  - `CBECOpsBot-private`：含 `telegram.mytelegram / ${TELEGRAM_CHAT_ID_PRIVATE}` + `api.myapi / api`
  - 确认 `TELEGRAM_CHAT_ID_GROUP` / `TELEGRAM_CHAT_ID_PRIVATE` 环境变量已设置为实际 Telegram chat ID
  - 确认 Task 3.5.2 DB 迁移已执行（`channel_bindings` 含新 gateway name 记录）
- [x] 4.2 群聊回复定向验证（出站隔离）
  - 在群聊中 @bot 发送文本消息 → 确认 **Bot 回复只出现在群聊**，私聊不收到 Bot 的回复内容
  - （注：入站广播属于 Matterbridge 网关架构问题，超出本提案范围，不作为本任务验收条件）
- [x] 4.3 私聊回复定向验证（出站隔离）
  - 在私聊中向 Bot 发送文本消息 → 确认 **Bot 回复只出现在私聊**，群聊不收到 Bot 的回复内容
  - （注：同上，入站广播超出本提案范围）
- [x] 4.4 回归验证：既有单 `inout` 场景正常（单 inout gateway 回写不受影响）

## 5. OpenSpec 流程收尾

- [x] 5.1 执行 Specflow 严格验证（specflow validate fix-bridge-reply-chat-routing --strict）
  - [x] `node design/context-dev/tools/specflow/specflow.mjs validate fix-bridge-reply-chat-routing --strict`
  - [x] 预期：全部 artifact 存在且无未解析占位符残留
- [x] 5.2 代码审查
  - [x] 确认无 Bearer Token 硬编码（BR-030）
  - [x] 确认日志不泄露 token（RISK-006）
- [x] 5.3 合并后执行归档（specflow archive fix-bridge-reply-chat-routing --yes）
  - [x] `node design/context-dev/tools/specflow/specflow.mjs archive fix-bridge-reply-chat-routing --yes`
