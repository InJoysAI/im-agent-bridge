# 项目准则 (Criterion)

> **Metadata**
> - **Source**: `.context/domain/source/IM-Agent-Bridge-PRD.md`, `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-12 23:17`
> - **Last Modified**: `2026-04-17 14:17`
> - **Generator**: `Context-Dev-Agent v1.0`

---

## 1. 仓库结构约束

```text
im-agent-bridge/
├── .context/                # AI 上下文资产（由 /context-init 生成）
├── SSoT/                    # Single Source of Truth
│   ├── schema/migrations/   # Goose SQL 迁移文件
│   └── api/                 # TypeSpec API 契约
│       ├── tspconfig.yaml
│       ├── main.tsp
│       └── models/
├── design/                  # Context-Dev 工具链（AI 命令、模板、脚本）
├── gateway/                 # Core Layer — Gateway 服务（Rust）（待生成）
├── deploy/                  # 部署配置（待生成）
│   ├── edge-server/         # Edge Server（Matterbridge）
│   └── internal-server/     # Internal Server（Gateway + NanoBot + PostgreSQL + Shopify MCP）
└── openspec/                # OpenSpec 提案管理（待 /context-openspec 生成）
```

---

## 2. 三维约束体系

| 维度 | 工具 | 约束规则 |
|------|------|---------|
| **需求层** | OpenSpec (`openspec/config.yaml`) | 功能变更必须先创建提案（`/context-openspec proposal <change-id> [roadmap-doc]`），评审通过后再开发 |
| **数据层** | Goose SQL | 数据库变更必须通过 Goose 迁移脚本管理；路径 `SSoT/schema/migrations/` |
| **API 层** | TypeSpec | API 契约变更必须先修改 `SSoT/api/main.tsp` 并编译通过，再实现代码；路径 `SSoT/api/` |

> **生效前提**：
> - **需求层 (OpenSpec)**：仅在根目录 `openspec/` 初始化后生效。当前未初始化时，跳过提案约束，但建议尽早执行 `/context-openspec` 启用。
> - **数据层 / API 层**：已初始化，约束立即生效。

> **未启用的层**：
> - IPC 层：不适用（非桌面端项目）
> - 共享层：不适用（仅 API 层，无 IPC 层）

---

## 3. 技术栈强制约束

### 3.1 三层架构（固定边界）

```yaml
Architecture: 三层架构 — Channel Layer / Bridge Layer / Core Layer
Constraint: 层间边界固定，不可跨层调用

MUST:
  - Bridge Layer 仅与 Core Layer 的 Gateway 通信
  - Gateway 是 Core 对外的唯一入口
  - Runtime 不直接对接 Telegram 主入口
  - Bridge 不直接调用 Runtime Agent

MUST NOT:
  - 突破三层架构边界
  - Bridge 直接访问 Runtime 或数据库
  - Runtime 直接访问 Bridge 或 Telegram
```

### 3.2 Channel Layer

```yaml
Platform: Telegram（MVP 第一版唯一渠道）
Message_Type: 仅文本消息

MUST:
  - 只承载消息入口与消息出口
  - 非文本消息在 Gateway 层忽略或返回提示

MUST NOT:
  - 包含业务逻辑
  - 直接连接 Runtime
```

### 3.3 Bridge Layer

```yaml
Implementation: Matterbridge（API 模式）
Protocol: HTTP + Bearer Token（私有网络，MVP；生产升级 HTTPS）

MUST:
  - 接收 Telegram 消息并通过 Matterbridge API 暴露给 Gateway 适配器消费
  - 支持 Gateway 调用回写接口将消息转发回 Telegram
  - 仅作为消息桥接，不承担业务语义

MUST NOT:
  - 处理业务逻辑
  - 直接调用 Runtime Agent
  - 直接访问数据库
  - 管理 session
```

### 3.4 Core Layer — Gateway

```yaml
Language: Rust
Role: Core Layer 对外唯一入口

MUST:
  - 接收 Bridge 入站消息并校验 Bearer Token
  - 根据渠道来源标识查询 channel_bindings 解析 bot_id
  - 执行消息标准化（生成统一消息结构）
  - 生成并维护 session_id
  - 调用 Runtime Adapter
  - 对 Runtime 返回结果进行统一组织
  - 调用 Bridge API 完成消息回写
  - 写入消息状态/错误日志/链路日志
  - 入站消息强制 4096 字符上限（超过时拒绝处理并返回提示，记录日志）
  - 回复消息强制 4096 字符上限（截断并附加截断提示）

MUST NOT:
  - 承担模型推理能力
  - 做 MCP 选择或传递目标 MCP 实例
  - 保存 MCP 凭证和 MCP 实例明细
```

### 3.5 Core Layer — Runtime Adapter

```yaml
Type: Gateway 内部模块（非独立服务）
Protocol: Gateway ↔ Runtime 使用独立 HTTP 接口

MUST:
  - 将 Gateway 标准请求转换为 Runtime HTTP 请求格式
  - 将 Runtime 输出转换为标准回复对象
  - 将 session_id / chat_id / chat_type / user_id 映射到 Runtime 请求字段
  - 处理 Runtime 返回超长文本（4096 字符硬截断 + 截断提示）

MUST NOT:
  - 直接访问 Bridge 或 Telegram
  - 承担持久化职责
  - 承担 MCP 路由职责
```

### 3.6 Core Layer — Agent Runtime

```yaml
Default: NanoBot（MVP 首选候选）
MCP_Management: 通过 MEMORY.md 声明 MCP 实例，凭证通过 .env 注入
MCP_Instances: Shopify MCP（shopify-{store-slug} 命名格式）

MUST:
  - 管理最小上下文记忆
  - 基于 MEMORY.md 与运行环境自主选择 MCP
  - 直接调用 Shopify MCP
  - 可通过适配层被 Gateway 调用
  - 替换时不影响 Channel / Bridge 设计边界

MUST NOT:
  - 作为 Telegram 主入口
  - 接管 Bridge 能力
  - 承担 Gateway 的消息标准化和回写职责
  - 依赖数据库来选择 MCP
```

> **Runtime 侧运行时文件说明**：
> - `MEMORY.md` — NanoBot Agent Runtime 的上下文记忆文件，包含 MCP 实例声明（命名、品类、地区等）。**该文件属于 Agent Runtime 部署产物**，由运维在 Runtime 部署时手工创建或通过配置管理工具注入，不属于本仓库版本管理范围。格式规范见 TAD §9.4.1。
> - `.env` / `.env.*` — MCP 凭证注入文件（Shopify client_id / client_secret / domain）。**同样属于 Runtime 部署产物**，通过环境变量或 Secret Manager 注入，不入库。

### 3.7 数据持久化

```yaml
Type: PostgreSQL
Access: MVP 阶段主要由 Gateway 访问

MUST:
  - 存储 session 映射（sessions 表）
  - 存储 Bot 配置（bots 表）
  - 存储 Channel 绑定（channel_bindings 表）
  - 存储消息状态与错误索引（message_events 表）
  - 存储 Runtime 调用日志（runtime_logs 表）
  - 所有 Bot 实例共享同一 PostgreSQL 实例，通过 bot_id 实现逻辑隔离

MUST NOT:
  - 存储 MCP 实例配置
  - 存储 MCP 密钥引用（client_id_ref / client_secret_ref）
  - 在 Runtime 中强依赖数据库
```

---

## 4. 安全约束

- **认证方式**: Bridge ↔ Gateway 使用 `Authorization: Bearer <token>`
- **密钥管理**: 禁止硬编码；Telegram Token / Bridge Bearer Token / PostgreSQL 密码均通过环境变量或 Secret Manager；Shopify MCP 凭证由对应 MCP 实例在 .env 中加载
- **网络要求**: Bridge ↔ Gateway 使用 HTTP + Bearer Token（私有网络，MVP）；Edge Server 与 Internal Server 通过私有网络（VPN/云 VPC）互联；Bridge API 与 Gateway API 仅在私有网络内暴露；禁止公网裸露；生产环境应升级 HTTPS（TD-007）
- **最小权限**: Bridge 只访问 Gateway；Runtime 不直接访问 Bridge；MCP 只由 Runtime 发起调用；PostgreSQL 主要由 Gateway 访问
- **防注入**: Bridge API 必须校验 Bearer Token；Gateway 必须校验来源；仅允许受控字段进入 Runtime
- **数据治理**: message_events.input_text/output_text 截断至 512 字符；runtime_logs 仅 error 时写入 payload 并脱敏 PII；message_events 保留 30 天；runtime_logs 保留 14 天

---

## 5. 接口契约

### 5.1 Bridge ↔ Gateway

- **入站**: `GET {BRIDGE_URL}/api/messages` + `POST /gateway/inbound` — Gateway 内建 poller 轮询 Matterbridge 消息缓冲区并写入 Gateway
  - 认证: `Authorization: Bearer <token>`
  - 幂等键: `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id)`
  - 字段映射: 入站协议字段 `raw_message.message_id` 对应持久化字段 `bridge_message_id`；`bridge_channel_name` 空时以空字符串参与幂等索引（COALESCE 降级语义）
- **回写**: `POST {BRIDGE_URL}/api/message`（wire）— Gateway 将回复发回 Matterbridge，再由 Matterbridge 转发至 Telegram（SSoT 内部契约保留为 `POST /bridge/reply`）
  - 认证: `Authorization: Bearer <token>`
  - 幂等键: `reply_id`（契约；当前 Matterbridge `/api/message` 不识别该字段，重试遵循 at-most-once 原则）

### 5.2 Gateway ↔ Runtime

- **处理**: `bots.runtime_endpoint` — Gateway 按 `runtime_type` 分发到 RuntimeAdapter，直接调用 Runtime 原生接口
  - 请求: 标准消息请求对象（event_id, platform, chat_id, chat_type, user_id, session_id, text, timestamp, bot_id, metadata）
  - 响应: 标准回复对象（reply_id, bot_id, platform, chat_id, reply_type, text, session_id, status, metadata）

### 5.3 session_id 生成规则

- 私聊: `telegram:private:{chat_id}`
- 群聊: `telegram:group:{chat_id}`
- MVP 不引入 thread_id / group_id

### 5.4 错误码规范

| error_code | 含义 |
|------------|------|
| `RUNTIME_TIMEOUT` | Gateway → Runtime 超时（15s） |
| `RUNTIME_UNAVAILABLE` | Runtime 不可达/拒绝连接 |
| `RUNTIME_BAD_RESPONSE` | Runtime 响应格式不符合 Schema |
| `RUNTIME_SESSION_NOT_FOUND` | Runtime 侧会话不存在/已失效 |
| `MCP_TIMEOUT` | Runtime → MCP 超时（10s） |
| `MCP_UNAVAILABLE` | MCP 不可达/报错 |

---

## 6. 变更工作流（SSoT-first）

```
需求变更
    ↓
创建提案：/context-openspec proposal <change-id> [roadmap-doc]
    ↓
验证提案：node design/context-dev/tools/specflow/specflow.mjs validate <提案ID> --strict
    ↓
更新服务端/契约（如有）
    ↓
实现业务逻辑
    ↓
运行测试
    ↓
归档：node design/context-dev/tools/specflow/specflow.mjs archive <提案ID> --yes
```

---

## 7. SSoT 文件路径

> 以下所有路径均相对于项目根目录。

| 层 | 文件 | 用途 | 状态 |
|----|------|------|------|
| 需求层 | `openspec/config.yaml` | 项目信息 | 待 `/context-openspec` 生成 |
| 需求层 | `openspec/proposal-roadmap.md` | 提案路线图 | 待生成 |
| 需求层 | `openspec/specs/` | 当前规范（真理源） | 待生成 |
| 需求层 | `openspec/changes/` | 变更提案目录 | 待生成 |
| 数据层 | `SSoT/schema/migrations/` | Goose SQL 迁移文件目录 | ✅ 已初始化 |
| API 层 | `SSoT/api/tspconfig.yaml` | TypeSpec 编译配置（OpenAPI3 + JSON Schema） | ✅ 已初始化 |
| API 层 | `SSoT/api/main.tsp` | API 契约入口（Bridge ↔ Gateway / Gateway ↔ Runtime） | ✅ 已初始化 |
| API 层 | `SSoT/api/models/` | API 模型目录 | ✅ 已初始化 |

---

## 8. 性能与超时约束

| 阶段 | 超时上限 | 说明 |
|------|---------|------|
| Bridge → Gateway | ~200ms | 本地/内网通信 |
| Gateway → Runtime | ≤ 15s | hard timeout |
| Runtime → MCP | ≤ 10s | 嵌套在 Runtime 超时内 |
| Gateway → Bridge 回写 | ~500ms | 本地/内网通信 |
| **端到端 P95 目标** | **≤ 5s** | 正常场景目标 |

---

## 9. 统一入口

本文件（`.context/criterion.md`）是项目约束的**权威来源**。

> 💡 仅当源文档（PRD/架构等）变化时才需更新 `.context/`；业务代码变更不触发重生成。
