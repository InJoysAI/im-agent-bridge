## Context

`feat-runtime-reply-bridge` 实现 Gateway 的 Bridge 回写客户端，位于消息链路最后一跳（RuntimeAdapter 输出 → Bridge → Telegram）。核心设计决策集中在重试策略实现方式、409 幂等语义处理，以及 output_text 双重截断边界的职责划分。

## Goals / Non-Goals

- Goals:
  - 实现 `bridge_client.rs` 封装 POST /bridge/reply 调用与 3 次指数退避重试逻辑
  - 明确 output_text 两个截断点的职责边界（4096 for Bridge 展示 vs 512 for DB 持久化）
  - 通过 `reply_status` 字段为可观测性和监控提供状态追踪锚点
- Non-Goals:
  - 超过 3 次重试（roadmap out_of_scope）
  - 回写失败后向用户发送"投递失败"Telegram 提示（MVP 架构约束）
  - 动态调整重试延时（MVP 固定 1s/2s/4s）

## Decisions

- **Decision**: 重试策略选择 `tokio::time::sleep` 手写延时循环，不引入第三方 `backoff` crate；总尝试最多 4 次（初始 + 3 次重试，延时序列 1s/2s/4s）
  - **Alternatives considered**: `backoff` crate 提供更丰富的策略（jitter、max_elapsed_time 等），但引入额外依赖；MVP 仅需 3 次固定退避，手写循环更简洁且零依赖

- **Decision**: 重试错误分类——仅对可重试错误重试，不可重试错误立即失败
  - 可重试：网络错误 / 超时 / HTTP 5xx / 429
  - 不可重试：HTTP 400（请求格式错误）/ HTTP 401（Token 错误）——重试无法改善状况，且添加日志噪声，掊盖配置问题
  - **Alternatives considered**: “无脑重试所有非 200/409”实现更简单，但会对 401 等配置类错误产生 4 倍无效压力 + 4 倍错误日志，违反 criterion §4 可运维性预期

- **Decision**: `/bridge/reply` 请求/响应契约以 `SSoT/api/main.tsp` 为单一真相源
  - SSoT 定义响应码集合：200 / 400 / 401 / 409
  - `.context/architecture/api_strategy.md` 中额外提及的 404/500/502 为历史说明，本变更不依赖；若实际遇到 5xx 均归入"可重试错误"走退避分支
  - **Follow-up**: 建议后续单独文档修订提案对齐 `api_strategy.md` 与 SSoT

- **Decision（联调新增）**: 实际 wire 端点采用 Matterbridge 1.26 原生 `POST {BRIDGE_URL}/api/message`，而非 SSoT 声明的 `/bridge/reply`
  - **背景**：原设计假设 `feat-infra-matterbridge-deploy` 会引入独立的 `mb-adapter` 服务实现 `/bridge/reply`；联调时确认 Matterbridge 1.26 `BindAddress` 在私网直接可达，无业务复杂度证明中间层的必要
  - **Wire payload**：`{gateway, text, username?}`（Matterbridge `config.Message` 子集）。Gateway 内部 `BridgeReplyPayload` 仍按 SSoT `ReplyRequest` 字段存储（reply_id/chat_id/platform/bridge_gateway_name/bridge_channel_name），由 `to_matterbridge_message` 函数在发送前做映射；留作未来恢复代理层的接缝
  - **Alternatives considered**:
    - (A) 保留 `mb-adapter` 设计并实现之——代码量至少 +300 行，仅为匹配 SSoT 契约，当前需求不支持
    - (B) 修改 SSoT `main.tsp` 直接删除 `Bridge.reply`——混淆"Gateway 内部契约"与"Matterbridge 外部协议"两个不同概念域
    - (C, 采用) 临时偏差 + 在 proposal 中显式记录 + 另开 change 做 SSoT 对齐
  - **Trade-off**：损失 `reply_id` 级幂等（Matterbridge 不识别此字段）；保留防御性 409 处理分支以便未来恢复代理层

- **Decision**: output_text 双重截断职责严格分离
  - RuntimeAdapter 负责截断至 4096 字符（BR-003）：保证 Telegram 展示合规，发送给 Bridge 的文本不超过 4096 字符
  - 消息处理 handler 在落库前负责截断至 512 字符（BR-070）：保证持久化最小化
  - 两个截断点不可合并，职责不同

- **Decision**: HTTP 409 → 立即视为成功返回，不进入重试循环（api_strategy.md §2.5，BR-062）
  - Matterbridge 收到相同 reply_id 说明回复已投递，重复发送无意义
  - 与 SSoT API 契约对齐（`SSoT/api/main.tsp` Bridge.reply 已声明 409 语义）

## Risks / Trade-offs

- `RISK-005`：Matterbridge 4 次尝试后仍失败，用户无法收到 Telegram 回复；`reply_failed` 状态通过 `reply_write_error_total` 指标可被监控追踪
- `RISK-006`：Bearer Token 泄露——通过日志脱敏 Requirement + 单元测试硬验证不记录 `Authorization`
- 截断双边界的运维注意事项：生产排查时，Telegram 中展示的文本（≤4096 字符）与 DB 中存储的 output_text（≤512 字符）可能不同，需知悉
- **At-most-once 投递（联调新增）**：Matterbridge 1.26 `/api/message` 不识别 `reply_id`，重试在理论上可能造成 Telegram 重复消息。缓解：仅对可重试错误（5xx/429/transport）重试；验收中未观测到重复。若后续观测到重复，恢复 `mb-adapter` 或让 Matterbridge 侧加 dedup

## Migration Plan

无 DB Schema 变更（`message_events.reply_status` 字段已在前序迁移中建立）。

**API 契约状态**：
- SSoT `SSoT/api/main.tsp` 的 `Bridge.reply` 端点定义保留原样（未改动），作为"内部契约"文档存在
- 实际实现使用 Matterbridge 1.26 外部协议 `POST /api/message`（不在 SSoT 管理范围内）
- 两者差异由本 change 的"实施偏差说明"节在 proposal 中显式承认，SSoT 对齐建议放在独立的 `fix-bridge-reply-ssot-align`（尚未创建）中处理

## Open Questions

- [ ] 确认 `feat-infra-gateway-scaffold` 的 `config.rs` 是否已导出 `BRIDGE_BEARER_TOKEN`；若未导出，实施时补充（不触发新提案，属于 config 扩展）
- [ ] **Follow-up SSoT 对齐**：决定走路线 A（SSoT 改为描述 Matterbridge 外部协议）还是路线 B（实现 mb-adapter 以兑现 SSoT）；暂不阻塞本 change 归档
