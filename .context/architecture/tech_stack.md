# 技术栈 (Tech Stack)

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-13 15:14`
> - **Generator**: `Context-Agent v1.0`

---

## 约束等级说明

| 等级 | 含义 | 违反后果 |
|------|------|---------| 
| **MUST** | 必须使用 | 架构不合规 |
| **SHOULD** | 推荐使用 | 需说明理由 |
| **MUST NOT** | 禁止使用 | 架构不合规 |

---

## 🔧 后端技术栈

| 技术 | 版本 | 约束 | 用途 |
|------|------|------|------|
| **Rust** | stable (latest) | MUST | Gateway 主语言 |
| **NanoBot** | latest ([HKUDS/nanobot](https://github.com/HKUDS/nanobot)) | MUST | MVP 默认 Agent Runtime |
| **Python** | 3.10+ (NanoBot 依赖) | MUST | NanoBot 运行环境（`pip install nanobot-ai`） |

---

## 🎨 前端技术栈

N/A – MVP 阶段无独立前端 / 管理后台。

---

## 💾 数据库技术栈

| 技术 | 版本 | 约束 | 用途 |
|------|------|------|------|
| **PostgreSQL** | 建议 15+（TAD 未指定版本） | MUST | 主数据库：session 映射、Bot 配置、Channel 绑定、消息事件、运行时日志 |

> **MUST NOT**：不使用 Redis / 缓存层（MVP 简化）。

---

## ☁️ 基础设施

| 技术 | 约束 | 用途 |
|------|------|------|
| **Docker** | MUST | 容器化各服务 |
| **Docker Compose** | MUST | Internal Server 服务编排（gateway / nanobot / postgres / shopify-mcp-*）；本地开发单机用 |

> **部署模型**：多服务器部署—— Matterbridge 独立在 Edge Server，其余服务在 Internal Server。两台服务器通过私有网络（VPN / 云 VPC）互联。

---

## 🔄 DevOps 工具链

| 技术 | 约束 | 用途 |
|------|------|------|
| **Goose** | **MUST** | 数据库迁移管理（`SSoT/schema/migrations/`）；已初始化，约束立即生效（见 criterion.md §2） |
| **TypeSpec** | **MUST** | API 契约定义（`SSoT/api/main.tsp`）；已初始化，约束立即生效（见 criterion.md §2） |
| **Progenitor** | SHOULD | 从 OpenAPI 生成 Rust 客户端代码（Oxide Computer） |
| **@azure-tools/typespec-rust** | SHOULD | TypeSpec → Rust SDK 直接生成（备选方案） |

---

## 🔌 集成组件

| 技术 | 约束 | 用途 |
|------|------|------|
| **Matterbridge** | MUST | Telegram ↔ Gateway 桥接器 (Go, [42wim/matterbridge](https://github.com/42wim/matterbridge)) |
| **Telegram Bot API** | MUST | Channel Layer 消息入口/出口 |
| **Shopify MCP** (`geli2001/shopify-mcp`, Node.js) | MUST | Shopify 数据查询工具；以 `npx shopify-mcp` 子进程方式运行在 nanobot 容器内（stdio MCP）；每店铺一个实例；凭证通过 MCP config.json `tools.mcpServers` 传入；nanobot 容器需同时包含 Python + Node.js 运行时 |

---

## ⛔ 禁止项 (MUST NOT)

| 禁止项 | 说明 |
|--------|------|
| MCP Router | 不单独引入，MCP 由 Runtime 自主选择 |
| MCP 配置持久化 | PostgreSQL 不存 MCP 实例配置/密钥引用 |
| 独立管理后台 | MVP 不含 |
| 富媒体消息处理 | MVP 仅支持文本 |
| 独立长期记忆系统 | MVP 不含 |
| Gateway 侧 MCP 选择逻辑 | Gateway 不做 MCP 路由 |

---

## 📦 核心依赖

### 后端 (Gateway - Rust)

> [!NOTE]
> 以下 Rust 依赖为**实现级建议**，TAD 未指定具体框架。开发时可根据团队偏好调整。

| 依赖 | 用途 |
|------|------|
| **actix-web / axum** | SHOULD — HTTP 框架 |
| **sqlx / diesel** | SHOULD — PostgreSQL ORM/驱动 |
| **reqwest** | SHOULD — HTTP 客户端（调用 Runtime / Bridge） |
| **tokio** | MUST — 异步运行时 |
| **tracing** | MUST — 结构化日志（`feat-observability-logging` 正式落地） |
| **tracing-subscriber** | MUST — JSON 格式化输出 + 脱敏 Layer（features: `env-filter`, `json`） |
| **serde / serde_json** | MUST — JSON 序列化 |

### Runtime (NanoBot - Python)

| 依赖 | 用途 |
|------|------|
| **nanobot-ai** | MUST — NanoBot Python 包 (`pip install nanobot-ai`) |
| **MEMORY.md** | MUST — MCP 实例声明文件（TAD §9.4.1，Runtime 启动时读取，获取可用 MCP 列表） |
| **.env / 环境变量** | MUST — LLM API Key + Shopify MCP 凭证注入 |

**NanoBot 启动方式**（外部参考，需以 [NanoBot 官方文档](https://github.com/HKUDS/nanobot) 验证）：

```bash
# 安装
pip install nanobot-ai

# 启动 OpenAI-compatible API 服务器（端口 8900）
pip install "nanobot-ai[api]"
nanobot serve
```

> [!WARNING]
> 以上命令来自 HKUDS/nanobot 项目文档，**非 TAD 原文定义**。实施前须以 NanoBot 当前版本官方文档为准。

**Runtime Adapter 调用映射 (TAD §9.2-9.3)**：

Gateway 内部按 `bots.runtime_type` 分发到对应的 `RuntimeAdapter` 实现（Strategy Pattern），直接调用 `bots.runtime_endpoint`：

| 标准字段 | NanoBot 请求字段 | 约束 |
|---------|----------|---------|
| `session_id` | `session_id` | **必传**；缺省会退为 `api:default` 导致会话串扰 |
| `text` | `messages: [{"role": "user", "content": text}]` | 严格限 1 条消息，多传返回 400 |
| `model`（可选） | `model` | 必须与服务端 `model_name` 一致（默认 `"nanobot"`） |
| 响应文本 | `choices[0].message.content` | `usage` 字段始终为全零 |

> [!NOTE]
> HKUDS/nanobot `nanobot serve` 提供 HTTP API（端口 8900）。该 API 共享 OpenAI 的 URL 路径和响应格式，但请求语义不同：服务端自管理历史记录，调用方每次只发当前消息。
> 另提供 `GET /v1/models` 和 `GET /health` 端点可用于健康检查。

**Runtime Adapter Endpoint**

| 端点 | 约束 | 用途 |
|------|------|------|
| `POST /v1/chat/completions` | MUST | 发送消息给 Runtime |
| `GET /v1/models` | SHOULD | 获取可用模型列表 |
| `GET /health` | SHOULD | 健康检查 |

**MCP 实例声明 (TAD §9.4.1)**：

Runtime 在启动时通过 `MEMORY.md` 获取当前环境可用的 MCP 实例：

```text
=== 可访问的 Shopify 店铺 ===
MCP Server: shopify-cool-gadgets | 显示名称: 酷玩小屋 | 品类: 电子产品 | 地区: 美国(US) | 币种: USD | 时区: America/New_York | 备注：xxxx
MCP Server: shopify-trendy-fashion | 显示名称: 潮流时尚 | 品类: 服饰 | 地区: 欧洲(EU) | 币种: EUR | 时区: Europe/Paris | 备注：xxxx
```

> [!NOTE]
> HKUDS/nanobot 原生使用 `config.json → tools.mcpServers` 管理 MCP。
> 实施时需开发适配逻辑，从 `MEMORY.md` 读取 MCP 实例声明并映射到 NanoBot 配置。

---

## AI 引用指南

当 AI 生成代码时：
1. 使用 MUST 等级技术，禁止使用 MUST NOT 技术
2. 优先使用 SHOULD 等级技术
3. Gateway 必须用 Rust 实现
4. 数据库迁移应优先使用 Goose (SHOULD, TAD §14 目录结构建议)
5. API 契约应优先使用 TypeSpec (SHOULD, TAD §14 目录结构建议)
6. NanoBot 是 Python 项目（HKUDS/nanobot），通过 Runtime Adapter 集成（TAD §9.3）
7. MCP 实例通过 `MEMORY.md` 声明（TAD §9.4.1），不通过 PostgreSQL 持久化
