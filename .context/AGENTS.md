# IM Agent Bridge Context v1.0

> **Metadata**
> - **Source**: `design/context-dev/templates/context-agents.md.template`
> - **Generated At**: `2026-04-12 23:17`
> - **Last Modified**: `2026-04-12 23:28`
> - **Generator**: `Context-Dev-Agent v1.0`

---

## 🚀 AI 入口说明

> **⚠️ 重要**: 本文件是 **AI 编辑工具的统一入口**。
>
> 当你（AI）打开此文件，意味着你正在进入一个已初始化的项目上下文。
>
> **执行前必读**:
> 1. `criterion.md` — 项目准则，包含必须遵守的技术约束
> 2. `context-manifest.json` — 了解当前源文件和生成状态
>
> **可用命令**: 见下方 [相关命令](#相关命令) 表格

---

## 目录说明

`.context/` 是 AI 代理的"长期记忆"目录，包含项目的结构化上下文资产。

> 📂 完整 `.context/` 目录树见 `README.md` — 本文件不重复维护目录树。
>
> ⚠️ `SSoT/` 和 `openspec/` 位于**项目根目录**，不在 `.context/` 内部。
> `.context/openspec/` 仅作为集成占位目录；实际 OpenSpec SSoT 目录为根目录 `openspec/`（当前未生成，需执行 `/context-openspec` 后方可使用）。

## 核心文件

| 文件 | 用途 |
|------|------|
| `criterion.md` | 项目准则 — AI 必须遵守的工程约束 |
| `context-manifest.json` | 元数据清单 — 追踪源文件归档状态 |
| `architecture/source/IM-Agent-Bridge-TAD.md` | 技术架构设计文档（权威来源） |
| `domain/source/IM-Agent-Bridge-PRD.md` | 产品需求文档（权威来源） |
| `db/source/IM-Agent-Bridge-TAD.md` | 数据库设计（从 TAD §8 提取） |

## 更新规则

- **增量更新**: 使用 `/context-update` 重生成变更文件
- **新增源文档**: 使用 `/context-update` 添加新源文件
- **全量重生成**: 删除 `context-manifest.json` 后重新运行 `/context-init`

## 与 OpenSpec 集成

- `.context/` 提供项目约束和业务规则
- 根目录 `openspec/` 管理变更提案和任务追踪
- AI 在创建提案时必须读取 `.context/criterion.md`

> **当前状态**：根目录 `openspec/` 尚未生成。需执行 `/context-openspec` 完成初始化后，OpenSpec 工作流方可使用。

## 相关命令

| 命令 | 用途 |
|------|------|
| `/context-init` | 初始化 .context/ 目录 |
| `/context-check` | 检查工具链与 MCP |
| `/context-openspec` | 生成 context 总结 + OpenSpec 集成 |
| `/context-openspec proposal <change-id> [roadmap-doc]` | 生成变更提案（可显式指定提案大纲/路线图文件） |
| `/context-start` | 基于提案开始开发 |
| `/context-update` | 增量更新 context |
