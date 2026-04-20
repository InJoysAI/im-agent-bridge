# Design: feat-gateway-mention-filter

> 仅记录非显而易见的决策；常规实现细节见 `proposal.md` / `tasks.md`。

## Decision 1: 过滤点选在 Gateway 入口层（对比 Matterbridge / NanoBot）

**选项**：

| 方案 | 粒度 | 多租户 | LLM 成本 | 配置生命周期 |
|------|------|--------|----------|---------------|
| A. Matterbridge `MessageMatchRegex` | 桥接全局 | ❌（单实例单正则） | 0 | 需改 TOML 重启 |
| B. NanoBot 系统提示词 `<SKIP>` | 单 agent | ✅ | **高**（每次进推理） | prompt 模板 |
| C. Gateway + DB 配置（本提案） | 单 bot | ✅ | 0 | `UPDATE bots` 即生效 |

**选 C**。理由：

- 与 BR-032 "所有 bot 配置走 DB" 的规范一致，避免 env / TOML 配置漂移。
- 零 LLM 成本，符合 Phase 5 "运营增强 / 降噪" 目标。
- 过滤发生在 `message_events` 写库之前，避免"已记录后丢弃"导致的审计 / 统计噪音。

## Decision 2: Expand-Contract 迁移策略（NULL + 默认 false）

**策略**：

1. **Expand**：`00005_bots_mention_filter.sql` 仅 `ADD COLUMN IF NOT EXISTS`，`telegram_username NULL`、`require_mention DEFAULT FALSE`，既有行不受影响。
2. **部署**：Gateway 新代码读取时 `require_mention.unwrap_or(false)` 等价退化为全量响应。
3. **Contract**：运营按需 `UPDATE bots SET telegram_username=..., require_mention=true WHERE id=...` 开启。

> 不纳入 NOT NULL 约束 — 允许 `telegram_username` 长期为 NULL（不开启 mention 的 Bot 无需填写）。

## Decision 3: 匹配算法（子串 + `@` 前缀 + 大小写不敏感）

- **实现**：`text.to_ascii_lowercase().contains(&format!("@{}", username.to_ascii_lowercase()))`。
- **为何要求 `@` 前缀**：若 `telegram_username = "Ops"`，避免普通文本 `Operations` 误命中；但不强制 Telegram 标准的 `@username ` 结尾空格，以兼容消息末尾 @（如 `"查下订单 @CBECOpsBot"`）。
- **为何不用正则**：避免引入 `regex` 依赖与 ReDoS 风险；子串命中对单条消息 O(n)，远低于限流阈值 5 msg/sec/chat_id 的 CPU 预算。
- **不支持别名**：roadmap 已明确 Out。

## Decision 4: `ignored_no_mention` 为 HTTP 200（非 4xx）

**权衡**：是否应作为 400 / 499 语义？

- 否。该响应表示"请求合法、被预期过滤"，Bridge 侧 MUST NOT 重试；采用 400 会污染 mb-adapter 的错误率监控。
- 与既有 `ignored_duplicate`（幂等去重）语义对齐：合法输入 + 预期过滤 = 200 + `status` 字段区分。
- `InboundResponse.status` 的三值枚举在 SSoT (`main.tsp`) 统一声明，避免各端散落。

## Decision 5: BR-032 严格读取路径

- Bot 加载仍通过 `bots::get_by_id(bot_id)`；`bot_id` 由 `channel_bindings` 基于 `(platform, bridge_gateway_name, chat_id)` 解析得到（既有路径）。
- 明确拒绝引入 `bots::get_by_telegram_username()` 类反查 — 即使存在多 bot 共享同 username 的极端场景，也应由运营侧修复数据而非代码兼容。

## Decision 6: 入站判定链顺序（回应评审 Q1/Q2）

**固定顺序**（`gateway/src/handlers/inbound.rs` 内自上而下）：

```
1. Bearer Token 校验（BR-031）           — 401 拒绝
2. 字段/JSON 合法性校验（BR-004）        — 400 拒绝
3. message_type ≠ text 拦截（BR-001）    — 400 拒绝
4. 空文本/仅空白校验（BR-001 / edge_cases.md:17-18） — 400 拒绝
5. 文本长度 > 4096 拦截（BR-002）        — 400 拒绝
6. bots::get_by_id(bot_id) 加载 Bot 配置（BR-032）
7. @mention 过滤（本提案新增）           — 200 ignored_no_mention
8. Token Bucket 限流（BR-055）           — 429 拒绝
9. 写入 message_events + 调用 Runtime
```

**关键位次理由**：

- **第 7 步（Mention）置于第 8 步（限流）之前**：若颠倒，群聊闲聊会提前耗尽 `chat_id` 令牌桶（阈值 5 msg/sec/chat_id），真正 @ 机器人的消息反而会被 429 短路。将过滤前置可直接丢弃噪音，让令牌桶只计入"有效请求"。**权衡**：理论上群内可以用闲聊发起 CPU 层面的小规模泵（每条消息仅一次子串匹配 O(n)，n ≤ 4096），不会触达 Runtime；若未来需要对抗此类泵，可追加"每 chat_id 每秒最大子串匹配次数"这一独立计数器，不影响主链路。
- **第 7 步置于第 3-5 步（空文本 / 长度）之后**：空文本或超长文本会直接 400；在它们之前做 mention 匹配会无谓消耗 CPU 并增加 panic 面（尽管 `contains` 本身不 panic，但把顺序讲清楚可避免后续误重构）。
- **第 7 步置于第 6 步（Bot 加载）之后**：mention 判定需要 `bot.telegram_username` / `bot.require_mention`，必须先从 DB 取到 Bot 才能决策；这也满足 BR-032 的"读取必须经 get_by_id"约束。
- **第 6 步已在 handler 既有链路中存在**（由 `channel_bindings` 解析 `bot_id` 后调用），本提案不新增 DB round-trip。

**非决策**：是否把 mention 过滤命中的事件纳入 `gateway_inbound_ignored_total{reason="no_mention"}` Prometheus 指标 — 交由运维看板需求决定，不在本提案硬约束内。

## 非决策 / 跳过

- **错误码**：不涉及（200 正常响应），跳过 `tools/errcodes/` 流程。
- **灰度策略**：由运营侧手工 `UPDATE` 单 Bot 验证，无需代码级 feature flag。
- **监控指标**：可选扩展 `gateway_inbound_ignored_total{reason="no_mention"}` Prometheus 计数器 — 若本提案范围内一并加入，则在 `tasks.md` 3.x 补一条；roadmap 条目未强制要求，MVP 仅保证日志可查。
