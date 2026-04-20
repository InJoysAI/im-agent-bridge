# Change: Channel 解析 + Session 管理

## Why

Gateway 入站消息处理需要两项关键能力：

1. **bot_id 解析**：根据入站消息的 `platform` / `bridge_gateway_name` / `bridge_channel_name` 查询 `channel_bindings` 表，解析出对应的 `bot_id`。采用"精确匹配 → COALESCE 降级匹配 → 404 拒绝"三级策略，确保每条入站消息都能找到唯一归属的 Bot 实例（BR-004）。

2. **Session 管理**：根据 `chat_type` 生成标准 `session_id`（私聊：`telegram:private:{chat_id}`，群聊：`telegram:group:{chat_id}`），并通过 `sessions` 表 upsert 建立/复用会话上下文，为后续 Runtime 调用提供稳定的上下文标识（BR-010、BR-011）。

前置提案 `feat-gateway-inbound-gate` 已完成 Bearer Token 校验与 Token Bucket 限流。本提案在其基础上，为入站 handler 补全 Channel 解析与 Session 生命周期管理，是后续消息管道（`feat-gateway-message-pipeline`）的上游依赖。

## 范围边界

| 类型 | 内容 |
|------|------|
| ✅ In | channel_bindings 查询（来源三元组 → bot_id 解析，COALESCE 降级匹配） |
| ✅ In | session_id 生成函数（私聊/群聊规则，BR-010/011） |
| ✅ In | sessions upsert（冲突键升级为 `(bot_id, session_id)`，支持多 Bot 隔离） |
| ✅ In | sessions / 后续读写携带 bot_id 过滤条件（BR-032） |
| ✅ In | 新增 Goose 迁移 `00003`：sessions.session_id UNIQUE → `(bot_id, session_id)` 联合唯一 |
| ❌ Out | 入站幂等去重（`uq_message_events_inbound_dedup`）— 移至 `feat-gateway-message-pipeline` 与 message_events INSERT 同提案；**本提案不返回 `ignored_duplicate`** |
| ❌ Out | message_events INSERT（`feat-gateway-message-pipeline`） |
| ❌ Out | 群聊按 User 粒度拆分会话（技术债 TD-006，Post-MVP） |

> **消歧备注**：路线图提案 5 "业务目标"列提到幂等去重，但路线图"范围表"已标为 Out，**以范围表为准**。

## What Changes

### 新增功能
- `gateway/src/db/channel_bindings.rs`：channel_bindings 查询函数，通过来源三元组 `(platform, bridge_gateway_name, bridge_channel_name)` 解析 bot_id，采用精确匹配 + COALESCE 降级匹配两步策略（channel_bindings 是 bot_id 的解析源头，查询谓词为来源三元组，不以 bot_id 过滤）
- `gateway/src/db/sessions.rs`：sessions 表 upsert 函数，冲突键升级为 `(bot_id, session_id)` 联合唯一，支持多 Bot 共享实例下的数据隔离（BR-032）
- `generate_session_id()` 辅助函数：按 `chat_type` 生成标准 `session_id` 字符串（BR-010、BR-011）

### 修改功能
- `gateway/src/db/mod.rs`：注册 `channel_bindings` 和 `sessions` 子模块
- `gateway/src/handlers/inbound.rs`：在 Bearer Token 校验 + 限流检查之后，集成 channel 解析与 session upsert 调用链

### 技术实现
- channel_bindings 查询采用 COALESCE 降级语义：先精确匹配 `(platform, bridge_gateway_name, bridge_channel_name)`；若无结果则降级匹配 `(platform, bridge_gateway_name, bridge_channel_name IS NULL)`；仍无结果返回 404（复用 `idx_channel_bindings_lookup` 索引，已在 `00001_init.sql` 就位）
- sessions upsert：`INSERT ... ON CONFLICT (bot_id, session_id) DO UPDATE SET updated_at = NOW(), last_user_id = EXCLUDED.last_user_id`（基于新建的 `uq_sessions_bot_session` 联合唯一索引，见迁移 `00003`）
- sessions 函数签名含 `bot_id: Uuid` 参数，所有 sessions 读写携带 bot_id 过滤条件（BR-032）；channel_bindings 函数的查询谓词是来源三元组，无需也无法以 bot_id 过滤
- **新增 Goose 迁移** `SSoT/schema/migrations/00003_sessions_bot_session_unique.sql`：将 `sessions.session_id UNIQUE` 降级为普通列，新建 `(bot_id, session_id)` 联合唯一索引（解决多 Bot 同 chat_id 串写问题）
- 无需修改 `SSoT/api/main.tsp`（`POST /gateway/inbound` 的 404 响应已在 API 契约中定义）

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/channel-session/spec.md` — Channel 解析与 Session 管理行为规范（bot_id 解析三场景、session_id 生成规则、sessions upsert 幂等）

### 涉及的代码
- **新增**：
  - `SSoT/schema/migrations/00003_sessions_bot_session_unique.sql`（sessions 唯一约束升级）
  - `gateway/src/db/channel_bindings.rs`
  - `gateway/src/db/sessions.rs`
  - `generate_session_id()` 函数（位于 `models/` 或 `handlers/inbound.rs` 内联辅助函数）

- **修改**：
  - `gateway/src/db/mod.rs`（注册新子模块）
  - `gateway/src/handlers/inbound.rs`（集成 channel 解析 + session upsert 调用）

### 依赖关系
- **依赖**：`feat-gateway-inbound-gate`（已完成）
- **被依赖**：`feat-gateway-message-pipeline`

### 风险与注意事项
- `RISK-B004`（Session 设计过重致 MVP 膨胀）— 缓解：仅实现 `telegram:private/group:{chat_id}` 轻量规则，不引入用户级粒度，不引入 thread_id / group_id（BR-015）
- `RISK-B006`（群聊共享上下文语义混淆）— 缓解：① `upsert_session()` 必须写入 `last_user_id` 字段（每条消息覆盖更新），确保当前发言用户可被 Runtime 感知；② 后续 `feat-gateway-message-pipeline` 的 RuntimeProcessRequest 必须透传 `user_id` / `sender_name`，由 Runtime 自行区分发言人（降低 Runtime 端混淆风险）；③ 本项为已知 TD-006，Post-MVP 升级路径为按 User 粒度拆分 session_id

### 验证标准
- ✅ bot_id 解析：精确匹配、COALESCE 降级匹配、完全缺失 404 三种场景全部通过
- ✅ session_id 格式：私聊 → `telegram:private:{chat_id}`；群聊 → `telegram:group:{chat_id}`
- ✅ BR-032：sessions 所有读写均携带 bot_id 过滤条件；channel_bindings 以来源三元组解析 bot_id（无需 bot_id 过滤，由 `uq_channel_bindings_source` 唯一约束保证确定性）
- ✅ sessions upsert 幂等：`ON CONFLICT (bot_id, session_id)` 多次 upsert 不报错，`updated_at` 更新
- ✅ 多 Bot 隔离：不同 bot_id + 相同 chat_id → 两条独立 session 记录，不冲突

### 关联 Context 资产
| Scope | 资产路径 | 关联说明 |
|-------|---------|---------|
| criterion | `.context/criterion.md` | §3.4 Gateway MUST（bot_id 解析、session 生成）；§3.7 DB MUST（bot_id 隔离） |
| domain | `.context/domain/business_rules.md` | BR-004 bot_id 解析规则；BR-010/011 session_id 格式；BR-012/013 上下文隔离；BR-015 禁止 thread_id；BR-032 Bot 配置隔离 |
| architecture | `.context/architecture/api_strategy.md` | §1 POST /gateway/inbound 404 响应语义（channel_bindings 缺失） |
| db | `.context/db/schema_design.md` | channel_bindings 表结构；sessions 表结构；COALESCE 降级索引定义 |

### 提案大纲对齐（Roadmap Alignment）
| 字段 | 内容 |
|------|------|
| roadmap_source_primary | `openspec/proposal-roadmap.md` |
| roadmap_source_supplement | N/A |
| phase | Phase 1 |
| business_goal | channel_bindings → bot_id 解析（精确/降级/404）；session_id 生成；sessions upsert |
| dependencies | `feat-gateway-inbound-gate`（已完成） |
| acceptance_criteria | 精确/降级/404 三场景通过；session_id 格式正确；BR-032 全覆盖；upsert 幂等 |
