# 测试策略 (Testing Strategy)

> **Metadata**
> - **Source**: `.context/domain/source/IM-Agent-Bridge-PRD.md`
> - **Generated At**: `2026-04-12 23:38`
> - **Generator**: `Context-Agent v1.0`

---

## 🎯 验收标准格式 (BDD Gherkin)

所有验收标准采用 Given / When / Then 语法：

```gherkin
场景: [场景名称]
  Given [已知条件/前置状态]
  When [用户执行的动作]
  Then [预期结果]
```

---

## 📋 模块验收标准

### 模块 1: Channel 接入（Telegram 文本消息）

#### 场景 1: 文本消息成功接入
```gherkin
Given Telegram Bot 已配置并运行
  And 用户在 Telegram 中与 Bot 会话
When 用户发送一条文本消息
Then 消息成功进入系统处理链路
  And 消息到达 Gateway
```

#### 场景 2: 非文本消息处理
```gherkin
Given Telegram Bot 已配置并运行
When 用户发送一张图片消息
Then 消息不进入主处理链路
  And 系统不产生异常
  And 系统提示"当前仅支持文本消息"或安全忽略
```

#### 场景 3: 回复回写到原会话
```gherkin
Given 用户在 Telegram chat_id=12345 中发送了消息
  And 系统已成功处理并生成回复
When Gateway 将回复发送到 Bridge Layer
Then 回复发送到 Telegram chat_id=12345 原会话
  And 用户可在 Telegram 中看到回复
```

---

### 模块 2: Bridge 消息桥接

#### 场景 1: 消息稳定桥接
```gherkin
Given Matterbridge 已配置并连接 Telegram
When Telegram 消息到达 Bridge Layer
Then 消息被转发到 Core Layer 的 Gateway
  And Bridge 不修改消息业务语义
```

#### 场景 2: Bridge 配置变更不影响 Gateway
```gherkin
Given Bridge 配置发生变更（如增加新渠道标签）
When 重启 Bridge Layer
Then Gateway 对内接口结构不受影响
```

---

### 模块 3: Core Layer — 消息标准化

#### 场景 1: 标准消息对象完整
```gherkin
Given Bridge 层转发了一条 Telegram 消息
When Gateway 接收并标准化消息
Then 标准消息对象包含所有必须字段
  And 字段包括: event_id, platform, chat_id, chat_type, user_id, session_id, text, timestamp, bot_id
  And Runtime 不依赖 Telegram 原始协议格式
```

#### 场景 2: bot_id 解析
```gherkin
Given 入站消息包含渠道来源标识（platform / bridge_gateway_name / bridge_channel_name）
When Gateway 执行消息标准化
Then Gateway 查询 channel_bindings 表解析出 bot_id
  And bot_id 不由外部请求直接传入
```

#### 场景 3: 入站消息超长拒绝
```gherkin
Given 用户发送一条长度为 5000 字符的文本消息
When Gateway 接收到该消息
Then 消息不进入主处理链路
  And 系统返回提示（如"消息过长，请缩短后重试"）
  And 系统记录日志（包含原始消息长度）
```

---

### 模块 4: Core Layer — Runtime 调用

#### 场景 1: Runtime 正常调用
```gherkin
Given Gateway 已生成标准消息请求对象
When Gateway 调用 Runtime Agent
Then Runtime 返回标准文本回复对象
  And 回复在 15s 内返回
```

#### 场景 2: Runtime 无响应
```gherkin
Given Gateway 已生成标准消息请求对象
When Gateway 调用 Runtime Agent 超过 15s 无响应
Then 系统返回通用失败提示
  And 系统记录错误日志
```

#### 场景 3: Runtime 返回格式异常
```gherkin
Given Runtime 返回了非标准格式的数据
When Gateway 解析 Runtime 响应
Then 系统记录日志
  And 中断回写流程
  And 返回通用错误提示
```

---

### 模块 5: Core Layer — Shopify MCP 工具调用

#### 场景 1: MCP 正常调用
```gherkin
Given Runtime 识别到用户需要查询 Shopify 数据
When Runtime 调用 Shopify MCP 工具
Then MCP 在 10s 内返回结果
  And Runtime 将结果组织为文本回复
```

#### 场景 2: MCP 不可达
```gherkin
Given Shopify MCP 服务不可达
When Runtime 尝试调用 MCP
Then 系统返回"工具暂不可用"
```

#### 场景 3: MCP 执行失败
```gherkin
Given Shopify MCP 服务可达但执行出错
When MCP 返回错误结果
Then 系统返回用户可理解的失败信息
```

---

### 模块 6: 会话边界管理

#### 场景 1: 私聊独立上下文
```gherkin
Given 用户 A 在私聊中发送了消息
When Gateway 生成 session_id
Then session_id = "telegram:private:{chat_id}"
  And 该消息进入独立的私聊上下文
```

#### 场景 2: 群聊共享上下文
```gherkin
Given 用户 A 和用户 B 在同一群聊中发送消息
When Gateway 分别生成 session_id
Then 两条消息的 session_id 相同 = "telegram:group:{chat_id}"
  And 共享同一群聊上下文
```

#### 场景 3: 私聊与群聊隔离
```gherkin
Given 用户 A 在私聊和群聊中分别发送了消息
When Gateway 分别生成 session_id
Then 私聊 session_id = "telegram:private:{private_chat_id}"
  And 群聊 session_id = "telegram:group:{group_chat_id}"
  And 两个上下文彼此隔离
```

---

### 模块 7: PostgreSQL 持久化

#### 场景 1: Session 映射持久化
```gherkin
Given Gateway 生成了新的 session_id
When Gateway 写入 PostgreSQL
Then session_id 映射关系被正确持久化
  And 后续相同 chat_id 的消息可查询到该 session
```

#### 场景 2: 数据库不可用
```gherkin
Given PostgreSQL 服务不可达
When Gateway 尝试访问数据库
Then 系统返回"系统暂时不可用，请稍后重试"
  And 不允许在持久化失效状态下继续写入不一致会话
  And 记录系统级告警日志
```

#### 场景 3: Bot 配置读取
```gherkin
Given PostgreSQL 中存储了 Bot 配置
When Gateway 使用 bot_id 查询配置
Then 返回正确的 Bot 配置信息
```

---

### 模块 8: 幂等、限流与安全关键约束

#### 场景 1: 入站重复消息幂等去重
```gherkin
Given Gateway 已成功处理了一条入站消息
  And 该消息的 (platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id) 已写入 message_events
  And bridge_channel_name 为 NULL 时以空字符串 '' 参与幂等比较（COALESCE 降级语义）
When 相同 (platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id) 的消息再次到达 Gateway
Then Gateway 幂等识别重复消息
  And 不得重新写入 message_events
  And 不得重复调用 Runtime
  And 返回幂等成功（不报错）
```

#### 场景 2: 限流触发（Rate Limit 5 msg/sec/chat_id）
```gherkin
Given 同一个 chat_id 在 1 秒内向 Gateway 发送了 5 条消息
When 同一 chat_id 在同一秒内发送第 6 条消息
Then Gateway 返回 HTTP 429 Too Many Requests
  And 该条消息不进入主处理链路
  And 不得调用 Runtime
```

#### 场景 3: PostgreSQL 不可达时熔断返回 503
```gherkin
Given PostgreSQL 服务不可达
When Gateway 收到任一入站消息
Then Gateway 立即返回 HTTP 503 Service Unavailable
  And 不得继续处理任何业务请求（短路熔断）
  And 向用户返回"系统暂时不可用，请稍后重试"
  And 记录系统级告警日志
```

#### 场景 4: 回写幂等防止重复发送（reply_id）
```gherkin
Given Gateway 已成功将一条回复通过 Bridge 写出
  And 该回复的 reply_id 已写入 message_events（reply_status = "success"）
When Gateway 因重试或异常再次尝试使用相同 reply_id 进行回写
Then Gateway 检测到 reply_id 唯一约束冲突
  And 不得重复发送该回复到 Bridge
  And 记录幂等重复回写告警日志
```

---

## 🔺 测试金字塔

| 层级 | 测试类型 | 覆盖率目标 | 工具 |
|------|---------|-----------|------|
| **单元测试** | 消息标准化、session_id 生成、长度校验逻辑 | ≥ 80% | Rust test (cargo test) |
| **集成测试** | Gateway ↔ PostgreSQL、Gateway ↔ Runtime 适配层 | ≥ 70% | Rust integration tests + testcontainers |
| **E2E 测试** | Telegram → Bridge → Gateway → Runtime → 回写 | 关键路径 100% | 手动联调 / 自动化脚本 |

---

## ⚡ 性能测试目标

| 指标 | 目标值 | 测试场景 |
|------|--------|---------|
| 端到端 P95 响应时间 | ≤ 5s | 标准单条消息 |
| 端到端 Hard Timeout | ≤ 15s | 极端场景 |
| Runtime 调用超时 | ≤ 15s | 单次调用 |
| MCP 工具调用超时 | ≤ 10s | 单次调用（嵌套在 Runtime 内） |

---

## 🔒 安全测试要求

| 测试项 | 标准 | 验证方式 |
|--------|------|---------|
| HTTPS 通信 | Bridge ↔ Gateway 使用 HTTPS（内网） | 证书检查 + 网络抓包 |
| API 认证 | Bridge ↔ Gateway 使用 Bearer Token（`Authorization: Bearer <token>`） | 未携带或 Token 无效时应返回 **401**（无降级路径） |
| 凭证保护 | 无硬编码凭证 | 代码审查 |
| 配置隔离 | 跨 Bot 数据不可访问 | 使用不同 bot_id 交叉验证 |

---

## 📝 UAT 流程

| 阶段 | 环境 | 执行人 | 内容 |
|------|------|--------|------|
| **Alpha 验收** | 本地/Docker 环境 | 系统开发者 | 核心主链路联调通过 |
| **Beta 验收** | 测试 Telegram Bot | 系统开发者 + 运维 | 真实 Telegram 消息闭环 |
| **Sign-off** | 生产前环境 | 项目负责人 | P0 功能全部验证通过 |

---

## ✅ Definition of Done

- [ ] 所有 BDD 场景通过
- [ ] 单元测试覆盖率达标
- [ ] Telegram 文本消息接入成功率 ≥ 95%
- [ ] 回复回写成功率 ≥ 95%
- [ ] Shopify MCP 成功调用率 ≥ 90%
- [ ] 性能测试通过（P95 ≤ 5s）
- [ ] 安全测试无高危漏洞
- [ ] UAT 签收完成
