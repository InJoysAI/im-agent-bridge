# 业务规则 (Business Rules)

> **Metadata**
> - **Source**: `.context/domain/source/IM-Agent-Bridge-PRD.md`
> - **Generated At**: `2026-04-12 23:38`
> - **Generator**: `Context-Agent v1.0`

---

## 命名约定

- **BR-XXX**: 业务规则编号
- **MUST**: 必须遵守（违反将导致系统错误或业务风险）
- **SHOULD**: 建议遵守（违反影响用户体验）
- **MAY**: 可选（增强功能）

---

## 消息接入与处理

### BR-001: 文本消息类型限定
- **强制等级**: MUST
- **规则**:
  - 第一版仅支持文本消息
  - 非文本消息不进入主处理链路
  - 非文本消息不得导致服务异常
- **验收点**: 非文本消息安全忽略或返回提示，不触发异常

### BR-002: 单条文本消息长度限制
- **强制等级**: MUST
- **规则**:
  ```
  IF 入站文本消息长度 > 4096 字符
  THEN 不进入主处理链路
  AND 返回提示（如"消息过长，请缩短后重试"）
  AND 记录日志（包含原始长度）
  ```
- **来源**: PRD §3.3.2（继承 Telegram 平台限制）
- **验收点**: 超长消息被拒绝并返回用户可见提示，不进入后续处理

### BR-003: 回复消息长度限制
- **强制等级**: MUST
- **规则**:
  ```
  IF 回复文本长度 > 4096 字符
  THEN 截断回复至 4096 字符
  AND 附加截断提示（如 "...（内容已截断）"）
  ```
- **来源**: PRD §3.3.5
- **验收点**: 所有回复消息不超过 4096 字符，超长时有可见截断提示

### BR-004: 标准消息结构字段完整性
- **强制等级**: MUST
- **规则**:
  - 标准消息对象必须包含以下字段：`event_id`、`platform`、`chat_id`、`chat_type`、`user_id`、`session_id`、`text`、`timestamp`、`bot_id`
  - `bot_id` 由 Gateway 根据入站消息的渠道来源标识（`platform` / `bridge_gateway_name` / `bridge_channel_name`）查询 `channel_bindings` 解析得出
  - `bot_id` 不由外部请求直接传入
- **验收点**: 进入 Runtime 的消息结构统一，Runtime 不依赖 Telegram 原始协议格式

### BR-005: 回复输出格式限定
- **强制等级**: MUST
- **规则**:
  - 第一版只输出文本
  - 不输出按钮、卡片、图片、文件或富媒体
- **验收点**: 所有成功结果统一转换为文本回复，所有失败结果统一转换为可理解提示

---

## 会话管理

### BR-010: Session ID 生成规则（私聊）
- **强制等级**: MUST
- **规则**:
  ```
  IF chat_type == "private"
  THEN session_id = "telegram:private:{chat_id}"
  ```
- **来源**: PRD §3.3.2
- **验收点**: 同一私聊消息进入同一私聊上下文

### BR-011: Session ID 生成规则（群聊）
- **强制等级**: MUST
- **规则**:
  ```
  IF chat_type == "group"
  THEN session_id = "telegram:group:{chat_id}"
  ```
- **来源**: PRD §3.3.2
- **验收点**: 同一群聊消息进入同一群聊上下文

### BR-012: 私聊与群聊上下文隔离
- **强制等级**: MUST
- **规则**:
  - 同一用户在群聊和私聊中分别向系统发起请求时，必须视为两个独立上下文
  - 私聊上下文与群聊上下文不得相互污染
- **来源**: PRD §2.3 场景 H
- **验收点**: 私聊与群聊上下文彼此隔离

### BR-013: 群聊共享上下文策略
- **强制等级**: MUST
- **规则**:
  - MVP 阶段按群聊维度复用共享 `session_id`
  - 群内消息共享同一上下文
  - 暂不支持群聊内按用户拆分上下文
- **来源**: PRD §3.5.1
- **验收点**: 群聊消息正确共享上下文

### BR-014: 上下文退化容忍
- **强制等级**: SHOULD
- **规则**:
  ```
  IF 上下文丢失（Runtime 侧）
  THEN 允许退化为无上下文单轮处理
  ```
- **来源**: PRD §3.5.2
- **验收点**: 上下文丢失时系统不崩溃，退化为单轮处理

### BR-015: Session ID 额外字段限制
- **强制等级**: MUST
- **规则**:
  - MVP 阶段不引入 `thread_id`
  - MVP 阶段不单独引入 `group_id`
- **来源**: PRD §1.4, §10.2 决议项 12
- **验收点**: 代码中不存在 `thread_id` 或 `group_id` 字段

---

## 架构边界

### BR-020: 三层架构边界
- **强制等级**: MUST
- **规则**:
  - 系统固定为三层架构：Channel Layer → Bridge Layer → Core Layer
  - Bridge Layer 不处理业务语义
  - Bridge Layer 不直接调用 Runtime Agent
  - Bridge Layer 仅与 Core Layer 的 Gateway 通信
- **验收点**: Bridge 配置变更不影响 Gateway 对内接口结构

### BR-021: Gateway 唯一入口
- **强制等级**: MUST
- **规则**:
  - Gateway 是 Core Layer 对 Bridge Layer 的唯一入口
  - Bridge 消息全部通过 Gateway 进入 Core
  - Gateway 不依赖具体 Runtime 实现
- **验收点**: 不存在绕过 Gateway 直接访问 Runtime 的路径

### BR-022: Runtime 可替换性
- **强制等级**: MUST
- **规则**:
  - Runtime 必须可替换
  - Gateway 只依赖接口，不依赖具体 Runtime
  - Runtime 选型变更不影响 Bridge 层
  - 替换 Runtime 时无需修改 Channel / Bridge 核心逻辑
- **来源**: PRD §3.4.2
- **验收点**: Runtime 接口不兼容时，在适配层解决，不向 Bridge 外溢

### BR-023: Runtime 不承担主入口
- **强制等级**: MUST
- **规则**:
  - Runtime 不直接接管 Telegram 主入口
  - 不使用 NanoBot 自带的 Channel / Gateway 作为本系统主入口
- **来源**: PRD §7.2
- **验收点**: 系统主入口由 Bridge + Gateway 管控

---

## 安全

### BR-030: 凭证保护
- **强制等级**: MUST
- **规则**:
  - Telegram、Bridge、MCP 凭证不得硬编码
  - 使用环境变量或安全配置
  - 凭证不得明文暴露
- **验收点**: 代码中无硬编码凭证

### BR-031: Bridge ↔ Gateway 安全通信
- **强制等级**: MUST
- **规则**:
  - Bridge 与 Gateway 之间采用 `Authorization: Bearer <token>` 进行身份验证
  - Bridge ↔ Gateway 外部通信采用 HTTPS
- **来源**: PRD §4.1, §7.1
- **验收点**: 无未授权请求可注入到 Gateway

### BR-032: 配置隔离
- **强制等级**: MUST
- **规则**:
  - 每个 Bot 实例配置逻辑隔离（通过 `bot_id` 区分，共享 PostgreSQL）
  - 所有 Bot 实例共享同一 PostgreSQL 实例
- **来源**: PRD §4.1
- **验收点**: 不同 Bot 配置互不影响

### BR-033: MCP 凭证管理
- **强制等级**: MUST
- **规则**:
  - 凭证管理架构：`1 Bot : 1 Runtime : N MCP 实例`
  - MCP 实例通过 `MEMORY.md` 文档声明与管理
  - Shopify API 凭证（Store URL, Client ID, Secret）通过 `.env` 注入
  - Runtime 不通过数据库动态配置 MCP 实例（MVP 阶段）
- **来源**: PRD §3.3.4, §7.2, §10.2 决议项 13
- **验收点**: MCP 实例声明与凭证注入路径正确

### BR-034: MCP 工具发现与默认启用策略
- **强制等级**: MUST
- **规则**:
  - MVP 阶段采用自动发现模式，默认启用 MCP 实例暴露的全部工具
  - 启用范围以 MCP 实例自身暴露的工具列表为准（即信任 MCP 实例的工具边界）
  - 若发现不安全工具：通过 `MEMORY.md` 配置限制工具列表 + 记录审计日志
- **来源**: PRD §3.3.4, §10.2 决议项 5
- **验收点**: MCP 工具自动发现并默认启用，审计日志可追溯工具调用记录

---

## 持久化

### 消息处理状态枚举定义

> 以下枚举对应 `message_events` 表字段，必须严格使用，不得扩充或混用。

| 字段 | 枚举值 | 语义 |
|------|--------|------|
| `message_events.status` | `pending` | 已写入，等待处理 |
| | `processing` | 正在调用 Runtime |
| | `done` | Runtime 已返回结果（无论回写是否成功） |
| | `error` | 处理失败（Runtime 超时 / 不可达 / 格式异常等） |
| `message_events.reply_status` | `success` | 回写到 Bridge 成功 |
| | `reply_failed` | 回写失败（包含重试剩尽后仍失败） |

---

### BR-040: PostgreSQL 持久化范围
- **强制等级**: MUST
- **规则**:
  - MVP 阶段引入 PostgreSQL 用于存储：session_id 映射关系、Bot 实例配置、必要上下文元数据
  - 持久化由 Gateway 负责
  - Runtime 不强依赖数据库
- **来源**: PRD §3.6.1
- **验收点**: Session 映射关系可正确持久化与查询，Bot 配置可正确读取

### BR-041: 数据库不可用处理
- **强制等级**: MUST
- **规则**:
  ```
  IF PostgreSQL 不可达
  THEN 返回统一错误提示（如"系统暂时不可用，请稍后重试"）
  AND 不允许在持久化失效状态下继续写入不一致会话
  AND 记录系统级告警日志
  ```
- **来源**: PRD §2.3 场景 I
- **验收点**: 数据库不可用时系统不崩溃，返回统一错误提示

### BR-042: 上下文元数据保留期、幂等权重与清理
- **强制等级**: MUST
- **规则**:
  ```
  上下文元数据保留期不超过 30 天
  清理方式：PostgreSQL 定时任务（如 pg_cron）或应用层 Gateway 定期清理
  清理对象：sessions 表及关联上下文元数据中超过 30 天的记录
  IF 清理任务执行失败
  THEN 记录系统告警日志，不阻断主流程
  ```
- **幂等规则**（MUST）:
  - **入站幂等键**（唯一复合索引）：`(platform, bridge_gateway_name, COALESCE(bridge_channel_name, ''), bridge_message_id)`
    — 同一来源同一消息只处理一次，重复到达时应返回幂等成功（不得重复写入 message_events）
  - **回写幂等键**（唯一索引）：`reply_id`
    — 同一回复只写出一次，小于或等于重试时不得重复回写
- **来源**: PRD §3.6.1, §6.3
- **验收点**: 超过 30 天的上下文元数据被自动清理，清理失败有告警；重复入站消息被幂等拒绝

---

## 性能

### BR-050: 端到端响应时间
- **强制等级**: MUST
- **规则**:
  - P95 端到端响应时间目标 ≤ 5 秒
  - Hard timeout ≤ 15 秒
- **来源**: PRD §4.2
- **验收点**: 联调阶段 P95 响应时间不超过 5 秒

### BR-051: Runtime 调用超时
- **强制等级**: MUST
- **规则**:
  - 单次 Runtime 调用最大等待时长 ≤ 15 秒
- **来源**: PRD §4.2
- **验收点**: 超时后返回通用失败提示

### BR-052: MCP 工具调用超时
- **强制等级**: MUST
- **规则**:
  - 单次 Shopify MCP 工具调用最大等待时长 ≤ 10 秒（嵌套在 Runtime 超时内）
- **来源**: PRD §4.2
- **验收点**: MCP 超时后返回"工具暂不可用"

### BR-055: 入站限流
- **强制等级**: MUST
- **规则**:
  ```
  限流维度：chat_id（每聊天会话独立计数）
  阈值：5 msg/sec/chat_id
  算法：Token Bucket
  IF 超过阈值
  THEN 返回 HTTP 429 Too Many Requests
  AND 该消息不进入主处理链路
  AND 不调用 Runtime
  AND 不写入 message_events
  ```
- **来源**: TAD §5.3, architecture/api_strategy.md §1.4
- **验收点**: 同一 chat_id 在 1 秒内发送第 6 条消息时收到 429；Runtime 不被触发

### BR-053: 模块故障隔离
- **强制等级**: MUST
- **规则**:
  - 单模块异常不导致整体崩溃
- **来源**: PRD §4.2
- **验收点**: 故障注入或联调验证通过

---

## 错误处理

### BR-060: Runtime 不可用
- **强制等级**: MUST
- **规则**:
  ```
  IF Runtime 无响应
  THEN 返回通用失败提示
  IF Runtime 返回格式异常
  THEN 记录日志并中断回写
  ```
- **来源**: PRD §2.3 场景 A, §3.3.3
- **验收点**: 用户收到明确错误提示

### BR-061: MCP 调用失败
- **强制等级**: MUST
- **规则**:
  ```
  IF Shopify MCP 不可达
  THEN 返回"工具暂不可用"
  IF MCP 执行失败
  THEN 返回用户可理解的失败信息
  ```
- **来源**: PRD §2.3 场景 B, §3.3.4
- **验收点**: 用户收到可理解的失败信息

### BR-062: 回写失败处理
- **强制等级**: MUST
- **规则**:
  ```
  IF 回复回写失败
  THEN 记录日志并返回失败状态
  IF 会话不存在或失效
  THEN 记录错误并停止回写
  ```
- **来源**: PRD §2.3 场景 D, §3.1.2
- **验收点**: 回写失败可被日志追溯

### BR-063: 错误可见性原则
- **强制等级**: MUST
- **规则**:
  - 错误必须可见，不能静默失败
  - 日志系统异常不得阻断主流程
- **来源**: PRD §4.4, §3.7.1
- **验收点**: 所有错误场景有可见输出或日志记录

---

## 数据隐私（NFR 补充摘要）

### BR-070: 消息数据最小化
- **强制等级**: MUST
- **规则**:
  - 消息内容可短期保留用于排障（截断存储，保留期不超过 30 天）
  - 不作为业务数据源
- **来源**: PRD §6.3

### BR-071: PII 脱敏
- **强制等级**: SHOULD
- **规则**:
  - 涉及 PII 的字段在落库时应脱敏或移除
- **来源**: PRD §6.3

### BR-072: 用户画像限制
- **强制等级**: MUST
- **规则**:
  - 不建设独立用户画像体系
- **来源**: PRD §6.3

---

## AI 引用指南

当 AI 生成业务逻辑代码时：
1. 首先查找对应的 BR-XXX 规则
2. 严格按照规则中的 IF-THEN 逻辑实现
3. 对于 MUST 级别规则，必须添加验证逻辑
4. 对于 SHOULD 级别规则，添加降级处理
