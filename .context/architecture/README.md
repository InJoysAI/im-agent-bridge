# Architecture — 架构约束

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-12 23:17`
> - **Generator**: `Context-Agent v1.0`

## 源文档

| 文件 | 类型 | 说明 |
|------|------|------|
| `source/IM-Agent-Bridge-TAD.md` | TAD | 技术架构设计文档 v1.1 |

> ⚠️ `source/` 目录中的文件为权威来源，谨慎修改。若总结与源文档冲突，以 `source/` 为准。

## 总结文件索引

| 文件 | 说明 | 优先级 |
|------|------|--------|
| [`README.md`](README.md) | 本文件：架构模块入口、文件索引、核心架构摘要 | 必读 |
| [`system_design.md`](system_design.md) | 系统设计总览：架构图、组件职责、NFR/SLO、部署拓扑 | 必读 |
| [`tech_stack.md`](tech_stack.md) | 技术栈约束：MUST / SHOULD / MUST NOT 分级 | 必读 |
| [`security_policy.md`](security_policy.md) | 安全策略：认证、授权边界、加密、敏感数据处理 | 推荐 |
| [`api_strategy.md`](api_strategy.md) | API 策略：三接口契约、幂等、错误码、限流 | 推荐 |
| [`runtime_view.md`](runtime_view.md) | 运行时视图：5 个核心场景时序图、超时预算、状态机 | 推荐 |
| [`deployment_view.md`](deployment_view.md) | 部署视图：Docker Compose 拓扑、容器划分、网络分区 | 推荐 |
| [`cross_cutting_concepts.md`](cross_cutting_concepts.md) | 跨切面概念：日志/指标/追踪、错误处理、限流、配置管理 | 推荐 |
| [`risks_and_debt.md`](risks_and_debt.md) | 风险与技术债务：7 风险 + 6 技术债追踪 | 推荐 |

## 读取优先级

1. **日常任务** → 读取总结文件（快速）
2. **提案检查** → 读取总结 + 约束验证
3. **遇到不确定/细节问题** → 回溯 `source/` 目录验证

## 核心架构

### 三层架构

| 层 | 实现 | 职责 |
|----|------|------|
| Channel Layer | Telegram | 消息入口与出口 |
| Bridge Layer | Matterbridge (API 模式) | Telegram ↔ Gateway 桥接 |
| Core Layer | Gateway (Rust) + NanoBot + PostgreSQL | 消息标准化、会话管理、Runtime 调用、持久化、回写 |

### 核心设计原则

- **单一入口**: Core 对外只有 Gateway 一个入口
- **桥接与业务分离**: Matterbridge 只负责桥接
- **Runtime 可替换**: Gateway 不依赖 Runtime 具体实现
- **上下文归属显式化**: session_id 由 Gateway 生成管理
- **Runtime 自主工具选择**: 基于 MEMORY.md 自主选择 MCP
- **安全默认**: Bridge ↔ Gateway 在 MVP 使用私有网络 HTTP + Bearer Token，生产升级 HTTPS
- **MVP 简化**: 不引入 MCP 配置持久化

### 关键接口

| 接口 | 方向 | 端点 |
|------|------|------|
| 入站 | Bridge → Gateway | `POST /gateway/inbound` |
| 回写 | Gateway → Matterbridge | `POST {BRIDGE_URL}/api/message`（wire；SSoT 契约为 `POST /bridge/reply`） |
| 处理 | Gateway → Runtime | `bots.runtime_endpoint`（按 `runtime_type` 分发） |
