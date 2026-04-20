# 实施任务清单

> feat-e2e-integration-test: 端到端集成验证。前置提案（feat-runtime-reply-bridge / feat-nanobot-shopify-mcp / feat-observability-logging / feat-infra-matterbridge-deploy）必须归档完毕后方可执行。SSoT 未更改（无 DB 迁移、无 API 合约变更）。不涉及新错误码。

## 0. 前置门禁检查

- [x] 0.1 确认所有前置提案已完成 specflow archive
  - [x] 0.1.1 `feat-runtime-reply-bridge` 已归档
  - [x] 0.1.2 `feat-nanobot-shopify-mcp` 已归档
  - [x] 0.1.3 `feat-observability-logging` 已归档
  - [x] 0.1.4 `feat-infra-matterbridge-deploy` 已归档
  - [x] 0.1.5 核查 `openspec/changes/archive/` 目录覆盖路线图全部已实施提案（criterion.md §6 门禁）
    - `ls openspec/changes/archive/`（输出 change-id 清单与路线图对照，无路线图已完成但 archive 目录缺失的提案）
- [x] 0.2 确认 Runtime 部署产物就绪（`MEMORY.md` 与 `.env` 配置正确，见 TAD §9.4.1）
- [x] 0.3 确认 Shopify MCP 测试店铺可访问

## 1. 完整环境拉起验证（0.25 天）

- [x] 1.1 执行 `docker compose up -d` 启动全部服务
- [x] 1.2 验证各服务 health check 通过
  - [x] 1.2.1 Gateway health check 返回 HTTP 200
  - [x] 1.2.2 NanoBot 可接受请求
  - [x] 1.2.3 PostgreSQL 连接正常
  - [x] 1.2.4 Matterbridge 已连接 Telegram（Edge Server）
- [x] 1.3 验证 Gateway 启动日志：无 DB 连接失败、无 Bearer Token 配置缺失
- [x] 1.4 核对端口监听与 Docker Compose 暴露策略（`deployment_view.md §部署约束`）
  - Gateway `:8080` 未对外暴露（仅 Internal Server 内网监听，无公网绑定 `0.0.0.0:8080`）
  - Matterbridge API `:4242` 仅通过 Edge Server 私网 IP 绑定，禁止公网可达
  - PostgreSQL `:5432` 未对外暴露（仅 Internal Server 内网）

## 2. 测试数据准备

> 基础测试数据通过 `scripts/seed_db.sh` 注入，需在 PostgreSQL 可用后、E2E 场景执行前完成。
> 注入数据是 bot_id 解析（`channel_bindings`）和 Runtime 调用（`bots.runtime_endpoint`）的基础，任何字段与实际部署配置不对齐均会导致主链路失败。

- [x] 2.1 执行 seed 脚本插入基础测试数据
  - `DATABASE_URL=postgres://... bash scripts/seed_db.sh`
  - 插入 `bots`：`id=11111111-1111-4111-8111-111111111111`，`bot_name='default-bot'`，`runtime_type='nanobot'`
  - 插入 `channel_bindings`：`id=22222222-2222-4222-8222-222222222222`，`platform='telegram'`
- [x] 2.2 验证 `bots.runtime_endpoint` 与 NanoBot 实际端口对齐（deployment_view.md: NanoBot `:8900`）
  - `SELECT id, runtime_endpoint FROM bots WHERE id='11111111-1111-4111-8111-111111111111';`
  - 若端口不符（如 seed 默认 `:9000`），以实际部署配置更新：
    `UPDATE bots SET runtime_endpoint='http://nanobot:8900/runtime' WHERE id='11111111-1111-4111-8111-111111111111';`
- [x] 2.3 验证 `channel_bindings.bridge_gateway_name` 与 Matterbridge 网关名对齐
  - deployment_view.md 定义 `matterbridge.toml` 中 `[[gateway]] name="CBECOpsBot"`；seed 默认值为 `'default'`
  - 不对齐将导致 Gateway 无法从 `channel_bindings` 解析出 `bot_id`（BR-004 失败）
  - 若不一致，更新为实际 Matterbridge gateway name：
    `UPDATE channel_bindings SET bridge_gateway_name='CBECOpsBot' WHERE id='22222222-2222-4222-8222-222222222222';`
- [x] 2.4 验证 `channel_bindings.bridge_channel_name` 对齐
  - seed 默认 `NULL`（COALESCE 降级语义：以空字符串 `''` 参与幂等键）
  - 联调时检查实际入站 Matterbridge 消息的 `channel` 字段值（对应 `${TELEGRAM_CHAT_ID}`）
  - 若 Matterbridge 消息携带非空 channel，需在 `channel_bindings` 中新增或更新 `bridge_channel_name` 以匹配
- [x] 2.5 查询验证 seed 数据完整性
  - `SELECT id, bot_name, runtime_type, runtime_endpoint, is_enabled FROM bots WHERE id='11111111-1111-4111-8111-111111111111';`
  - `SELECT id, bot_id, platform, bridge_gateway_name, bridge_channel_name, is_enabled FROM channel_bindings WHERE bot_id='11111111-1111-4111-8111-111111111111';`

## 3. 主链路 BDD P0 场景手动执行（1 天）

> 对照 `testing_strategy.md` 模块 1–8，逐场景验证

- [x] 3.1 模块 1: Channel 接入（Telegram 文本消息）
  - [x] 3.1.1 场景 1: 文本消息成功接入 → 消息到达 Gateway（检查日志 trace_id）
  - [x] 3.1.2 场景 2: 非文本消息处理 → 不进入主链路，不产生异常
  - [x] 3.1.3 场景 3: 回复回写到原会话 → Telegram 收到 AI 回复（chat_id 对应）

- [x] 3.2 模块 2: Bridge 消息桥接
  - [x] 3.2.1 场景 1: 消息稳定桥接 → Bridge 转发到 Gateway，不修改语义
  - [x] 3.2.2 场景 2: Bridge 配置变更不影响 Gateway → 重启 Bridge 后 Gateway 接口结构不变

- [x] 3.3 模块 3: 消息标准化
  - [x] 3.3.1 场景 1: 标准消息对象包含所有必须字段（event_id/platform/chat_id/chat_type/user_id/session_id/text/timestamp/bot_id）
  - [x] 3.3.2 场景 2: bot_id 由 channel_bindings 解析（验证 DB 中 channel_bindings 记录）
  - [x] 3.3.3 场景 3: 入站消息超长（5000 字）→ 拒绝处理 + 返回提示 + 日志记录原始长度（BR-002）

- [x] 3.4 模块 4: Runtime 调用
  - [x] 3.4.1 场景 1: Runtime 正常调用 → 15s 内返回标准回复对象
  - [x] 3.4.2 场景 2（见第 4 节异常注入）: 停止 NanoBot → 15s 超时 → 回写错误提示
  - [x] 3.4.3 场景 3: Runtime 返回格式异常 → 记录日志 + 中断回写 + 通用错误提示

- [x] 3.5 模块 5: Shopify MCP 工具调用
  - [x] 3.5.1 场景 1: MCP 正常调用 → 10s 内返回结果，Runtime 组织为文本回复
  - [x] 3.5.2 场景 2（异常注入）: Shopify MCP 不可达 → "工具暂不可用"
  - [x] 3.5.3 场景 3（异常注入）: MCP 执行失败 → 用户可理解的失败信息

- [x] 3.6 模块 6: 会话边界管理
  - [x] 3.6.1 场景 1: 私聊 session_id = "telegram:private:{chat_id}"（检查 sessions 表）
  - [x] 3.6.2 场景 2: 同一群聊两用户共享 session_id = "telegram:group:{chat_id}"
  - [x] 3.6.3 场景 3: 私聊与群聊 session_id 严格隔离（BR-012）

- [x] 3.7 模块 7: PostgreSQL 持久化
  - [x] 3.7.1 场景 1: session 映射正确持久化 → 后续相同 chat_id 可查到 session
  - [x] 3.7.2 场景 2（见第 4 节异常注入）: 停止 PG → HTTP 503 + "系统暂时不可用" + 告警日志
  - [x] 3.7.3 场景 3: bot_id 配置读取正确（channel_bindings 查询结果）

- [x] 3.8 模块 8: 幂等、限流与安全关键约束
  - [x] 3.8.1 场景 1: 重复入站幂等去重（BR-042）→ 构造两条具有相同幂等键的入站请求（`platform` + `bridge_gateway_name` + `bridge_channel_name` + `bridge_message_id` 均相同，criterion.md §3.4 / SSoT `uq_message_events_inbound_dedup`）→ 验证：第二次 Gateway 返回 HTTP 200 且响应体含 `{"status":"ignored_duplicate"}`；`message_events` 无新增行（MUST NOT 重复写入）；不重复调用 Runtime
  - [x] 3.8.2 场景 2: 限流触发 → 第 6 条/秒 → HTTP 429，不写 message_events（BR-055）
  - [x] 3.8.3 场景 3（见第 4 节异常注入）: PG 不可达时熔断 → HTTP 503，短路所有业务处理（BR-041）
  - [x] 3.8.4 场景 4: 回写幂等防重复 → reply_id 唯一约束冲突 → 不重复发送 + 告警日志（BR-062）
  - [x] 3.8.5 场景 5: 回写失败指数退避重试（BR-062）→ 关闭 Matterbridge 进程或修改 Gateway 回写地址为不可达地址，发起正常请求，查验 Gateway 日志确认依次触发 1s / 2s / 4s 三次退避重试后放弃，HTTP 409 出现时应视为成功不再重试
  - [x] 3.8.6 场景 6: 未授权请求拦截（BR-031）→ 使用 cURL 构造不含 Authorization 头或携带无效 Bearer Token 的请求发送至 Gateway，验证返回 HTTP 401 Unauthorized，请求不进入主处理链路

- [x] 3.9 模块补充: 超长回复截断（BR-003 / `criterion.md §3.4`）
  - [x] 3.9.1 构造 NanoBot 返回 >4096 字符回复的场景（如发送提示词“生成 5000 字详细文章”）
  - [x] 3.9.2 验证 Telegram 收到截断至 4096 字符的消息，附加“…[内容已截断]”标记（BR-003）
  - [x] 3.9.3 验证 Gateway 日志记录截断事件（含 trace_id，不含敏感内容）

- [x] 3.10 关键场景 DB 落库证据采集
  - [x] 3.10.1 主链路闭环后查询 `message_events`，验证 `reply_status = "success"` 且字段完整
    - `SELECT event_id, reply_status, created_at FROM message_events ORDER BY created_at DESC LIMIT 5;`
  - [x] 3.10.2 幂等去重场景后验证 `message_events` 无新增行（BR-042 MUST NOT 重复写入，幂等键为 `uq_message_events_inbound_dedup`）
    - `SELECT COUNT(*) FROM message_events WHERE bridge_message_id = '<重复的bridge_message_id>' AND platform = 'telegram' AND bridge_gateway_name = 'CBECOpsBot' AND COALESCE(bridge_channel_name,'') = COALESCE('<channel_name>','');`（结果应为 1，完整匹配 `uq_message_events_inbound_dedup` 唯一索引四字段）
  - [x] 3.10.3 限流场景后确认 `message_events` 无对应被拦截记录（限流在 Gateway 层拦截，不落库）
  - [x] 3.10.4 Runtime 超时场景后查询 `runtime_logs`，验证包含 RUNTIME_TIMEOUT 错误码，无敏感内容
    - `SELECT id, status, error_code, created_at FROM runtime_logs WHERE status='error' AND error_code='RUNTIME_TIMEOUT' ORDER BY created_at DESC LIMIT 3;`
  - [x] 3.10.5 DB 熔断恢复后验证 `message_events` 写入恢复正常

## 4. 异常场景注入测试（0.5 天）

- [x] 4.1 停止 NanoBot → 验证 15s 超时触发 → Gateway 回写错误提示（RUNTIME_TIMEOUT / RUNTIME_UNAVAILABLE）
- [x] 4.2 停止 PostgreSQL → 验证 HTTP 503 熔断（BR-041）
- [x] 4.3 重启 NanoBot → 验证 RUNTIME_SESSION_NOT_FOUND 时 Gateway 清空 runtime_session_key 并重建
- [x] 4.4 验证全部异常场景下日志脱敏（无 user_id 原文、无凭证明文输出）
- [x] 4.5 注入 MCP 网络故障 → 验证"工具暂不可用"返回（RISK-003 / BR-061）
  - 推荐方法：修改 `deploy/internal-server/nanobot/.env` 中 Shopify 凭证为无效值（如 `SHOPIFY_STORE1_CLIENT_ID=invalid`）并重启 nanobot 容器
  - 备用方法：修改 `nanobot-data/config.json` 中 shopify-mcp 的 `--domain` 指向不可达地址（如 `localhost:9999`）
  - 发起一条需要 Shopify 数据查询的用户请求
  - 验证 Telegram 收到"工具暂不可用"提示（不暴露 Shopify 凭证或技术细节）
  - 验证 `runtime_logs` 错误日志已脱敏（无 user_id 原文、无 Shopify client_secret 明文）

## 5. P95 响应时间验证（0.25 天）

- [x] 5.1 发送 20 条独立文本消息，记录端到端响应时间（Telegram 发送 → Telegram 收到回复）
- [x] 5.2 计算 P95 响应时间 ≤ 5s（`criterion.md §8`）
- [x] 5.3 如 P95 超标，记录测量数据并写入风险说明（RISK-003 / Shopify MCP 外部延迟）
  - 本轮采样 `total=20`，`ok=20`，`success_rate=100%`，`p95=3.748s`（未超标，风险说明不触发）

## 6. 可观测性验证

- [x] 6.1 验证每条消息均有 trace_id 贯通 Gateway → Runtime → 回写（`cross_cutting_concepts.md`）
- [x] 6.2 验证 10 个 Counter 指标可查询：
  - [x] 6.2.1 `messages_received_total` / `messages_replied_total`
  - [x] 6.2.2 `runtime_call_success_total` / `runtime_call_timeout_total`
  - [x] 6.2.3 `mcp_call_success_total` / `mcp_call_error_total`
  - [x] 6.2.4 `reply_write_success_total` / `reply_write_error_total`
  - [x] 6.2.5 `rate_limited_total` / `db_unavailable_total`

## 7. SSoT 核查

- [x] 7.1 确认本提案无 DB Schema 变更（SSoT 未更改，无需 Goose 迁移）
- [x] 7.2 确认本提案无 API 合约变更（SSoT 未更改，无需修改 SSoT/api/main.tsp）

## 8. 验证与归档

- [x] 运行 node design/context-dev/tools/specflow/specflow.mjs validate feat-e2e-integration-test --strict
  - 归档前已执行并通过；归档后同 change-id 路径已移入 archive，无法再次按原命令复跑
- [x] 运行 node design/context-dev/tools/specflow/specflow.mjs archive feat-e2e-integration-test --yes
