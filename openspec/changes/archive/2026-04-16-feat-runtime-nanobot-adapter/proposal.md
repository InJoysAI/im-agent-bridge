# Change: NanoBotAdapter 实现

## Why

`feat-gateway-message-pipeline` 已建立 Gateway 消息标准化与 Runtime 调用的主链路骨架，但 Runtime 调用环节尚无实际实现——系统缺少将 `StandardMessage` 转换为 NanoBot HTTP 请求的适配层。

NanoBot 的 `/v1/chat/completions` 接口中 `model` 字段协议层面可选，但若不传则服务端会使用内部默认值，可能导致不同环境运行行为不一致。当前 `bots` 表没有 `runtime_model` 字段，无法为不同 Bot 实例独立配置该参数（参见 `openspec/changes/archive/2026-04-16-feat-nanobot-deploy/tasks.md` curl 样例）。
Gateway 将始终主动传入 `model`（取自 `bots.runtime_model`）以避免服务端默认值差异引入不确定性；这是 Gateway 实现级的加固约束，不是对协议的改变。

本变更通过三步完成闭环：① 以 Goose 迁移添加 `bots.runtime_model`；② 实现 `RuntimeAdapter` trait（Strategy Pattern）；③ 实现 `NanoBotAdapter`，严格遵循 NanoBot 协议约束（`session_id` 必传、`messages` 严格 1 条、不传 `stream`、15s hard timeout、从 `bots.runtime_model` 取 `model` 值）。

## Scope

### ✅ In（本提案负责）
- `RuntimeAdapter` trait + `NanoBotAdapter`（Strategy Pattern，按 `runtime_type` 分发）
- `bots.runtime_model` 字段（Goose 迁移 `00004_bots_runtime_model.sql`）
- NanoBot 协议严格约束：`session_id` 必传、`messages` 严格 1 条、不传 `stream`、15s hard timeout
- `RuntimeError` 枚举 + 4 种 error_code 映射（TIMEOUT / UNAVAILABLE / BAD_RESPONSE / SESSION_NOT_FOUND）
- Session-not-found 处置：清空 `sessions.runtime_session_key` 并重建一次
- 回复 >4096 字符截断 + 截断提示（BR-003）

### ❌ Out（本提案明确不做）
- MCP 路由 / MCP 实例选择（Gateway MUST NOT 介入，见 `.context/criterion.md` §3.4）
- Gateway↔Runtime 安全认证机制（已知技术债 TD-001，MVP 阶段内网无认证）
- Bridge 回写（属后续提案 `feat-runtime-reply-bridge`）
- Runtime 调用日志持久化（属后续提案 `feat-persist-runtime-logs`）
- 限流（属前置链路 `feat-gateway-message-pipeline` 或 `cross_cutting_concepts`）
- 入站 >4096 字符拒绝（已由前置提案 `feat-gateway-message-pipeline` 覆盖）
- MCP 凭证落库（不新增任何凭证字段，见 `.context/criterion.md` §3.4）
- 引入 `thread_id`/`group_id` 等额外上下文字段（BR-015，协议不支持）
- 将 `NanoBotAdapter` 或 `RuntimeAdapter` 暴露为 Gateway 主入口（BR-023，Runtime MUST NOT 承担主入口）
- 收集或持久化用户历史行为 / 构建用户画像（BR-072，Out）

---

## What Changes

### 新增功能
- `RuntimeAdapter` trait + `NanoBotAdapter`（Strategy Pattern，按 `bots.runtime_type` 分发）
- `bots.runtime_model` 字段（Goose 迁移 `SSoT/schema/migrations/00004_bots_runtime_model.sql`，`TEXT NOT NULL DEFAULT 'nanobot'`）
- 统一 `RuntimeError` 枚举：`Timeout | Unavailable | BadResponse | SessionNotFound`，映射至系统 error_code 规范
- 回复超长截断：>4096 字符截断并追加截断提示（BR-003）

### 技术实现
- `gateway/src/adapters/runtime.rs`：定义 `RuntimeAdapter` trait + `RuntimeError` enum
- `gateway/src/adapters/nanobot.rs`：`reqwest` HTTP 客户端（15s timeout）；请求体 `{model: runtime_model, messages:[{role:"user",content:text}], session_id}`；响应解析 `choices[0].message.content`
- `SSoT/schema/migrations/00004_bots_runtime_model.sql`：`ALTER TABLE bots ADD COLUMN runtime_model TEXT NOT NULL DEFAULT 'nanobot'`（含 Up / Down）

## Impact

### 涉及的规范（Specs）
- **新增**：`specs/runtime-adapter/spec.md` — RuntimeAdapter trait 与 NanoBotAdapter 协议适配规范（含 `bots.runtime_model` 字段、错误码映射、回复截断）

### 涉及的代码
- **新增**：
  - `SSoT/schema/migrations/00004_bots_runtime_model.sql`
  - `gateway/src/adapters/runtime.rs`
  - `gateway/src/adapters/nanobot.rs`

- **修改**：
  - `gateway/src/db/bots.rs`（或 Bot 配置查询模块）：查询结果 struct 加入 `runtime_model: String` 字段
  - `gateway/src/handlers/inbound.rs`（或 Runtime 分发入口）：按 `runtime_type` 实例化对应 Adapter

### 依赖关系
- **依赖**：`feat-gateway-message-pipeline`（已完成）
- **被依赖**：`feat-runtime-reply-bridge`、`feat-persist-runtime-logs`

### 风险与注意事项
- **RISK-001**（Runtime 单点故障）：15s hard timeout + `RUNTIME_TIMEOUT` 映射确保 Gateway 可隔离 NanoBot 进程崩溃，不向上层漏出未处理错误
- **RISK-007**（TAD 与工具能力差距）：NanoBot 不支持 `stream: true`（返回 HTTP 400）；`model` 不匹配返回 HTTP 400；必须严格遵守协议约束
- **RISK-B001**（Runtime 候选能力不完全匹配）：`RuntimeAdapter` trait 保留替换空间；若实施中发现 NanoBot API 不兼容，在 Adapter 层解决，不外溢 Gateway 主链路
- `bots.runtime_model` 为 `NOT NULL DEFAULT 'nanobot'`，向后兼容现有数据行，Expand-Contract 迁移安全

### 验证标准
- ✅ `session_id` 字段在每次 NanoBot 请求体中存在且为正确格式
- ✅ Gateway 调用 NanoBot 超时（>15s）时 error_code = `RUNTIME_TIMEOUT`，用户收到 "抱歉，当前无法处理您的请求，请稍后再试。"
- ✅ NanoBot 返回回复文本 >4096 字符时截断至 4096 并附加截断提示
- ✅ `00004_bots_runtime_model.sql` Up / Down 迁移均可无错执行
- ✅ 单元测试覆盖：正常调用、超时、连接不可达、响应格式异常、4096 截断五类场景
