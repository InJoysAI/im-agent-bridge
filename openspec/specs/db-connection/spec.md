# db-connection Specification

## Purpose
TBD

## Requirements
### Requirement: PgPool 连接池初始化
系统必须（MUST）在 Gateway 启动时初始化 sqlx `PgPool`，连接上限为 100，DATABASE_URL 从环境变量读取；连接池初始化失败时 Gateway 进程必须退出并输出明确错误信息。

#### Scenario: 正常启动时连接池健康
- **WHEN** DATABASE_URL 有效且 PostgreSQL 可达
- **THEN** `health_check()` 返回 `Ok(())`
- **AND** Gateway 启动日志包含 "db pool initialized" 或等价信息

#### Scenario: DATABASE_URL 缺失时启动失败
- **WHEN** 环境变量 DATABASE_URL 未配置
- **THEN** Gateway 进程在启动时退出，exit code 非 0
- **AND** 错误信息明确指出缺失字段名（`DATABASE_URL`）

---

### Requirement: DB 不可用时短路熔断并向用户回写可见提示
系统必须（MUST）在 PostgreSQL 不可达时，阻断任何业务写入，并通过 Bridge reply API 向用户发送可见的错误提示 `"系统暂时不可用，请稍后重试"`，同时对 mb-adapter 返回 HTTP 503（BR-041、BR-063）。熔断检查必须在 payload 解析后、bot_id 解析前执行，以保证 chat_id 和 platform 字段可用于回写。

#### Scenario: PostgreSQL 不可达时熔断生效并回写用户提示
- **WHEN** PostgreSQL 服务不可达（连接超时或拒绝连接），且入站 inbound payload 已解析（chat_id、platform 可用）
- **THEN** Gateway 调用 Bridge reply API，向该 `chat_id` 发送文本 `"系统暂时不可用，请稍后重试"`
- **AND** Gateway 对 mb-adapter 返回 HTTP 503 Service Unavailable
- **AND** 系统日志记录 ERROR 级别告警，包含 `db_unavailable` 字段（权威级别：`cross_cutting_concepts.md` §日志规范）
- **AND** `db_unavailable_total` Counter 递增 1

#### Scenario: PostgreSQL 恢复后熔断自动解除
- **WHEN** PostgreSQL 服务在不可用后重新可达
- **THEN** 后续入站请求正常处理（不再触发熔断回写）
- **AND** 无需重启 Gateway 进程

#### Scenario: 熔断期间不写入任何 DB 记录
- **WHEN** DB 不可用且有入站消息到达
- **THEN** 不写入 message_events 或任何业务表
- **AND** 不调用 Runtime Adapter

---

### Requirement: Goose 迁移验证
系统必须（MUST）在 Gateway 启动时（或 CI 流程中）验证 Goose 迁移已正确应用，确认 5 张核心表（bots、channel_bindings、sessions、message_events、runtime_logs）及全部索引均存在。

#### Scenario: 迁移完整时验证通过
- **WHEN** `SSoT/schema/migrations/00001_init.sql` 和 `00002_channel_bindings_unique.sql` 均已应用
- **THEN** 5 张核心表均存在于数据库
- **AND** 所有幂等索引（`uq_message_events_inbound_dedup`、`uq_message_events_reply_id` 等）均已创建

#### Scenario: 迁移未应用时验证失败
- **WHEN** 目标数据库中缺少核心表（如 `bots` 表不存在）
- **THEN** Gateway 启动时输出明确错误日志，指出迁移未完成
- **AND** 运维可通过 `goose -dir SSoT/schema/migrations up` 修复

---

### Requirement: bot_id 参数贯穿 DB 函数签名（BR-032 规范）
系统必须（MUST）要求所有数据库访问函数在签名中携带 `bot_id: Uuid` 参数，禁止在无 bot_id 过滤条件的情况下执行跨 Bot 的全表查询（BR-032 多 Bot 数据隔离）。

#### Scenario: DB 函数签名包含 bot_id 参数
- **WHEN** 后续提案（`feat-gateway-channel-session` 等）实现 DB 查询函数
- **THEN** 所有 DB 函数签名包含 `bot_id: Uuid` 参数（代码审查确认）
- **AND** 禁止出现无 bot_id 过滤的全表 SELECT/UPDATE/DELETE

---

### Requirement: 开发环境 Seed 数据支持
系统应（SHOULD）提供 `scripts/seed_db.sh` 脚本，用于在开发/测试环境快速录入默认 bot 实例和 channel_bindings 映射数据，以支持后续集成测试。

#### Scenario: seed 脚本执行成功
- **WHEN** 在已完成 Goose 迁移的开发数据库上执行 `bash scripts/seed_db.sh`
- **THEN** `bots` 表中存在至少 1 条默认 bot 记录
- **AND** `channel_bindings` 表中存在对应的渠道绑定记录
