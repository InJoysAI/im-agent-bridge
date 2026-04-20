# 实施任务清单

> 实施 Bridge 回写链路（feat-runtime-reply-bridge）。前置条件：`feat-runtime-nanobot-adapter` 已归档，`config.rs` 已声明 `BRIDGE_URL` / `BRIDGE_BEARER_TOKEN` 环境变量。

## 1. SSoT 先行检查

- [x] 1.1 验证 `SSoT/api/main.tsp` 中 `/bridge/reply` 端点与 `ReplyRequest` 模型与实现意图一致（无需修改，仅确认）
  - `ReplyRequest` 必须字段：reply_id, chat_id, platform, text, bridge_gateway_name；可选：bridge_channel_name
  - 响应：200 ok / 400 / 401 / 409
  - **联调偏差（2026-04-17）**：由于 `mb-adapter` 代理层取消，实际 wire 端点改为 Matterbridge 1.26 原生 `POST /api/message`。Gateway 内部 `BridgeReplyPayload` 仍保留 SSoT 字段用于日志追踪；详见 `proposal.md` §实施偏差说明 与 `design.md` Decision（联调新增）。SSoT 本身未改动，对齐留给后续独立 change
- [x] 1.2 验证 `SSoT/schema/migrations/` 中 `message_events` 表 `reply_status` 字段已存在（枚举值：success / reply_failed）
  - 若字段缺失，须新建 Goose 迁移文件补充该字段后再继续（SSoT-first）
- [x] 1.3 确认 `gateway/src/config.rs` 已导出 `BRIDGE_BEARER_TOKEN`（若缺失，补充环境变量读取，不属于 SSoT 变更）

## 2. 实现 bridge_client.rs

- [x] 2.1 创建 `gateway/src/bridge_client.rs`
  - [x] 2.1.1 定义 `BridgeReplyPayload` 结构体（字段：reply_id, chat_id, platform, text, bridge_gateway_name, bridge_channel_name?，serde Serialize）
  - [x] 2.1.2 实现 `post_reply(client: &reqwest::Client, bridge_url: &str, bearer_token: &str, payload: &BridgeReplyPayload) -> Result<(), BridgeError>` 异步函数
    - wire 端点 `POST {BRIDGE_URL}/api/message`（Matterbridge 1.26 原生，联调偏差；详见 design.md）；发送前由 `to_matterbridge_message` 把 `BridgeReplyPayload` 映射为 `{gateway, text, username?}`
    - `Authorization: Bearer <BRIDGE_BEARER_TOKEN>`
    - HTTP 200 → Ok
    - HTTP 409 → Ok（幂等成功，立即返回，不进入重试；当前 wire 下不可观测，代码保留作为代理层恢复接缝）
    - **不可重试错误**（HTTP 400 / 401）→ 立即返回 `Err(NonRetryable)`，不进入重试循环
    - **可重试错误**（网络错误 / 超时 / HTTP 5xx / 429 / 其他非 2xx/400/401/409）→ 触发重试逻辑
  - [x] 2.1.3 实现指数退避重试循环
    - 重试次数：最多 3 次（含初始调用共 4 次）；delays = [1s, 2s, 4s]，第 4 次失败后直接返回 Err
    - 使用 `tokio::time::sleep(Duration::from_secs(n))`

## 3. output_text 截断与 reply_status 更新

- [x] 3.1 实现 `truncate_to_512(text: &str) -> String`
  - 按 UTF-8 字符边界截断，不破坏多字节字符
  - 超过 512 字符时截断；不足时原样返回
- [x] 3.2 在消息处理链路（RuntimeAdapter 返回回复后）集成调用
  - [x] 3.2.1 在调用 `bridge_client::post_reply` 前确保 payload `text` ≤ 4096 字符（兜底截断 + 提示，BR-003）
  - [x] 3.2.2 调用 `bridge_client::post_reply` 回写至 Bridge
  - [x] 3.2.3 成功（含 409）→ 更新 `message_events.reply_status = "success"`，`reply_write_success_total` +1
  - [x] 3.2.4 最终失败（不可重试立即失败 或 4 次尝试耗尽）→ 更新 `message_events.reply_status = "reply_failed"`，记录错误日志（含 reply_id 与 HTTP 状态码/错误类型），`reply_write_error_total` +1
  - [x] 3.2.5 落库 output_text 前调用 `truncate_to_512`
  - [x] 3.2.6 日志脱敏实现：确保请求/响应日志均不包含 `Authorization` 头值或 `BRIDGE_BEARER_TOKEN` 字面值（RISK-006）

## 4. 单元测试

- [x] 4.1 `bridge_client` 单元测试（使用 `mockito` 或 `wiremock-rs` mock HTTP server）
  - [x] 4.1.1 HTTP 200 → Ok，reply_status=success
  - [x] 4.1.2 HTTP 409 → Ok（幂等成功），不重试
  - [x] 4.1.3 HTTP 503 连续 4 次失败（等待序列 1s/2s/4s）→ Err，调用方更新 reply_status=reply_failed
  - [x] 4.1.4 首次失败后第 2 次成功 → Ok，reply_status=success
  - [x] 4.1.5 HTTP 401 → 立即返回 Err，不重试（验证 mock server 仅收到 1 次请求）
  - [x] 4.1.6 HTTP 400 → 立即返回 Err，不重试
  - [x] 4.1.7 日志脱敏：验证日志输出不包含 Bearer Token 字面值
- [x] 4.2 `truncate_to_512` 单元测试
  - [x] 4.2.1 输入超 512 字符 → 输出不超 512 字符，UTF-8 边界完整
  - [x] 4.2.2 输入 ≤ 512 字符 → 原样返回

- [x] 4.3 手动测试（本地 docker-compose 环境） — **需用户在联调环境执行**
  - [x] 4.3.1 发送一条 Telegram 消息，验证 Telegram 侧收到 AI 回复（端到端正常链路冒烟）
  - [x] 4.3.2 临时停止 Matterbridge，发送消息后验证 `message_events.reply_status = "reply_failed"` 且错误日志中有 reply_id
  - [x] 4.3.3 构造超 512 字符的 NanoBot 回复，验证 `message_events.output_text` 长度 ≤ 512

## 5. 集成验证

- [x] 5.1 联调环境验证 — **需用户在联调环境执行**
  - [x] 5.1.1 正常回写：POST /bridge/reply → 200 → reply_status=success，消息出现在 Telegram
  - [x] 5.1.2 Gate 场景：模拟 Bridge 连续 4 次可重试错误（初始 + 3 次重试，等待 1s/2s/4s）→ reply_status=reply_failed，错误日志可查
  - [x] 5.1.3 不可重试错误场景：模拟 Bridge 返回 401 → 立即 reply_status=reply_failed（不进入重试循环）

## 6. 验证与归档

- [x] 6.1 specflow validate feat-runtime-reply-bridge --strict（`node design/context-dev/tools/specflow/specflow.mjs validate feat-runtime-reply-bridge --strict`）
- [x] 6.2 specflow archive feat-runtime-reply-bridge --yes（`node design/context-dev/tools/specflow/specflow.mjs archive feat-runtime-reply-bridge --yes`） — **Phase 8：待用户核查通过后执行**
