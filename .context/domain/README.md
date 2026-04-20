# Domain 业务领域知识

> **Metadata**
> - **Source**: `.context/domain/source/IM-Agent-Bridge-PRD.md`
> - **Generated At**: `2026-04-12 23:42`
> - **Generator**: `Context-Agent v1.0`

---

## 目录用途

存储从 PRD 产品需求文档中提取的结构化业务领域知识，供 AI Agent 和开发者在提案创建、代码生成、业务逻辑验证时快速参考。

---

## 文件列表

| 文件 | 用途 | 优先级 |
|------|------|--------|
| `README.md` | 本文件：Domain 模块入口、文件索引、业务规则快速索引 | 必须 |
| `business_rules.md` | 核心业务规则（BR-XXX），按领域分类，含 MUST/SHOULD/MAY 强制等级 | 必须 |
| `user_journeys.md` | 核心用户旅程、角色画像、异常流程映射、Session 路由决策图 | 推荐 |
| `edge_cases.md` | 异常与边缘情况处理（消息、Session、Runtime、MCP、DB、回写、安全） | 推荐 |
| `testing_strategy.md` | BDD Gherkin 验收标准、测试金字塔、性能/安全测试目标、UAT 流程 | 推荐 |
| `risks_and_debt.md` | 业务/项目风险（RISK-B00X）、依赖管理、项目债务（PD-00X） | 可选 |
| `domain_model.md` | 核心领域对象定义（字段、类型、约束、关系、ER 图） | 推荐 |

---

## 源文档

| 源文档 | 路径 | 类型 |
|--------|------|------|
| IM Agent Bridge PRD v1.1 | `source/IM-Agent-Bridge-PRD.md` | 产品需求文档 |

---

## 读取优先级

1. **日常任务** → 读取总结文件（快速）
2. **提案检查** → 读取总结 + `business_rules.md` 验证
3. **遇到不确定/细节问题** → **回溯 `source/` 目录验证**

> ⚠️ **若总结与源文档冲突，以 `source/` 目录中的源文档为准**

---

## 业务规则快速索引

| 领域 | BR 编号范围 | 概要 |
|------|------------|------|
| 消息接入与处理 | BR-001 ~ BR-005 | 文本限定、长度截断、标准消息结构、回复格式 |
| 会话管理 | BR-010 ~ BR-015 | session_id 生成、上下文隔离、退化策略 |
| 架构边界 | BR-020 ~ BR-023 | 三层架构、Gateway 唯一入口、Runtime 可替换 |
| 安全 | BR-030 ~ BR-033 | 凭证保护、安全通信、配置隔离、MCP 凭证管理 |
| 持久化 | BR-040 ~ BR-042 | PostgreSQL 范围、DB 不可用处理、数据清理策略 |
| 性能 | BR-050 ~ BR-053 | 端到端超时、Runtime/MCP 超时、故障隔离 |
| 错误处理 | BR-060 ~ BR-063 | Runtime/MCP/回写失败、错误可见性 |
| 数据隐私 | BR-070 ~ BR-072 | 消息最小化、PII 脱敏、画像限制 |
