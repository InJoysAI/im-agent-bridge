# runtime-adapter Specification

## Purpose
TBD

## Requirements
### Requirement: RuntimeAdapter trait
系统必须（MUST）通过 `RuntimeAdapter` trait 实现 Runtime 调用的 Strategy Pattern，按 `bots.runtime_type` 在 Gateway 内部分发到对应 Adapter 实现，确保 Runtime 可替换性（BR-022）。

#### Scenario: 按 runtime_type 分发
- **WHEN** Gateway 处理 `runtime_type = "nanobot"` 的 Bot 消息
- **THEN** 实例化 `NanoBotAdapter` 并调用其 `process` 方法
- **AND** 返回结果封装为 `StandardReply` 传递给 Gateway 主链路

---

### Requirement: bots.runtime_model 字段
系统必须（MUST）在 `bots` 表中存储 `runtime_model TEXT NOT NULL DEFAULT 'nanobot'` 字段，供 NanoBotAdapter 构建请求时用作 `model` 值。

#### Scenario: Goose 迁移执行（Up）
- **WHEN** 执行 `SSoT/schema/migrations/00004_bots_runtime_model.sql` Up 迁移
- **THEN** `bots` 表新增 `runtime_model TEXT NOT NULL DEFAULT 'nanobot'` 列
- **AND** 现有数据行 `runtime_model` 自动填充为 `'nanobot'`

#### Scenario: Goose 迁移执行（Down）
- **WHEN** 执行 `00004_bots_runtime_model.sql` Down 迁移（**仅限本地开发 / CI 回滚使用**）
- **THEN** `bots` 表 `runtime_model` 列被移除，数据库恢复至迁移 00003 状态
- **⚠️** 生产环境**禁止**直接执行 DROP COLUMN（会丢失数据）；生产回滚须遵循 Expand-Contract 策略（Rename + 保留期），见 `.context/db/migrations_and_ssot.md`

#### Scenario: Bot 配置读取包含 runtime_model
- **WHEN** Gateway 查询 `bots` 表获取 Bot 配置
- **THEN** 查询结果包含 `runtime_model` 字段且值非空

---

### Requirement: NanoBotAdapter 协议适配
系统必须（MUST）在 `NanoBotAdapter` 中严格按照 NanoBot `/v1/chat/completions` 接口规范构建请求并解析响应。

#### Scenario: 正常请求构建
- **WHEN** Gateway 调用 `NanoBotAdapter.process(msg, bot_config)`
- **THEN** 发送 `POST {bot.runtime_endpoint}/v1/chat/completions`，Body 包含 `model`（取自 `bot.runtime_model`）、`messages: [{role: "user", content: msg.text}]`（严格 1 条）、`session_id: msg.session_id`
- **AND** 请求不包含 `stream` 字段

#### Scenario: session_id 必传
- **WHEN** `NanoBotAdapter.process` 被调用
- **THEN** 请求体中 `session_id` 字段必须存在且与 `msg.session_id` 一致

#### Scenario: 正常响应解析
- **WHEN** NanoBot 返回 HTTP 200 且 `choices[0].message.content` 存在
- **THEN** 提取该字段作为回复文本，封装为 `StandardReply { status: "success", text }`

---

### Requirement: 错误码映射
系统必须（MUST）将 NanoBot HTTP 调用的各类失败映射为统一 `RuntimeError` 枚举，并对应 error_code 规范（`openspec/config.yaml` Domain Context）。

#### Scenario: 请求超时
- **WHEN** NanoBot HTTP 调用超过 15s 未返回
- **THEN** 映射为 `RuntimeError::Timeout`，error_code = `RUNTIME_TIMEOUT`
- **AND** 用户侧收到 "抱歉，当前无法处理您的请求，请稍后再试。"

#### Scenario: 连接不可达
- **WHEN** `reqwest` 连接 NanoBot endpoint 失败（连接被拒绝或网络不通）
- **THEN** 映射为 `RuntimeError::Unavailable`，error_code = `RUNTIME_UNAVAILABLE`

#### Scenario: 响应格式异常
- **WHEN** NanoBot 返回 HTTP 2xx 但 Body 不含 `choices[0].message.content`
- **THEN** 映射为 `RuntimeError::BadResponse`，error_code = `RUNTIME_BAD_RESPONSE`
- **AND** 记录错误日志，不向用户发送异常数据

#### Scenario: Session 不存在
- **WHEN** NanoBot 返回错误响应且错误含义为 session 不存在（**精确 HTTP 状态码与错误体结构须由 tasks 3.2 探针测试确认后填入实现**）
- **THEN** 映射为 `RuntimeError::SessionNotFound`，error_code = `RUNTIME_SESSION_NOT_FOUND`
- **AND** Gateway 清空 `sessions.runtime_session_key` 并重建一次
- **⚠️** 在 tasks 3.2 探针完成前，禁止硬编码 SessionNotFound 触发条件（避免判断逻辑被穿透）

---

### Requirement: 回复超长截断
系统必须（MUST）对 NanoBotAdapter 返回的回复文本超出 4096 字符时截断并附加截断提示（BR-003）。

#### Scenario: 超长回复截断
- **WHEN** `choices[0].message.content` 长度 > 4096 字符
- **THEN** 截断至 4096 字符并追加截断提示（如 "…（内容已截断）"）
- **AND** 封装为 `StandardReply { status: "success", text: <截断后文本> }`

#### Scenario: 正常长度回复不截断
- **WHEN** `choices[0].message.content` 长度 ≤ 4096 字符
- **THEN** 原文返回，不附加截断提示
