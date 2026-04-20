> **Metadata**
> - **Source**: `.context/domain/source/IM-Agent-Bridge-PRD.md`, `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Generated At**: `2026-04-12 23:17`
> - **Last Modified**: `2026-04-12 23:28`
> - **Generator**: `Context-Dev-Agent v1.0`

---

# IM Agent Bridge — Context 目录

本目录是 AI 代理的"长期记忆"目录，包含 IM Agent Bridge 项目的结构化上下文资产。

## 项目概述

IM Agent Bridge 是一个**通用多 IM AI 接入骨架**，采用三层架构（Channel Layer → Bridge Layer → Core Layer），先以 Telegram 为首个验证渠道，聚焦消息收发、Agent 调用、真实工具调用（Shopify MCP）和回复回写，验证最小链路成立。

## 目录结构

```
.context/
├── README.md                # 本文件 — 目录总览
├── AGENTS.md                # AI 入口文件（指令、命令、更新规则）
├── criterion.md             # 项目准则 — 工程约束与技术规范
├── context-manifest.json    # 元数据清单
├── architecture/            # 架构约束
│   ├── README.md            # 架构模块索引
│   └── source/              # 源文档（只读）
│       └── IM-Agent-Bridge-TAD.md
├── domain/                  # 业务领域
│   ├── README.md            # 领域模块索引
│   └── source/              # 源文档（只读）
│       └── IM-Agent-Bridge-PRD.md
├── db/                      # 数据库设计
│   ├── README.md            # 数据库模块索引
│   └── source/              # 源文档（只读）
│       └── IM-Agent-Bridge-TAD.md
└── openspec/                # 集成占位目录（用于与根目录 openspec/ 集成）
```

> ⚠️ 项目级 SSoT（`SSoT/`）和 OpenSpec（`openspec/`）位于**项目根目录**，不在 `.context/` 内部。
> `.context/openspec/` 仅作为集成占位目录；实际 OpenSpec SSoT 目录为根目录 `openspec/`（当前未生成，需执行 `/context-openspec`）。
> 详见 `criterion.md` §1 仓库结构约束 和 §7 SSoT 文件路径。

## 文件职责分工

| 文件 | 职责 |
|------|------|
| `README.md` | 目录总览 — 你正在读的文件（.context/ 目录树、源文档归档路径） |
| `AGENTS.md` | AI 统一入口 — 命令索引、更新规则、必读顺序 |
| `criterion.md` | 项目准则 — 技术约束的权威来源 |
| `context-manifest.json` | 元数据清单 — 源文件归档状态追踪 |

## 关键源文档（归档路径）

| 源文档 | 归档路径 | 说明 |
|--------|---------|------|
| PRD v1.1 | `domain/source/IM-Agent-Bridge-PRD.md` | 产品需求文档（权威来源） |
| TAD v1.1 | `architecture/source/IM-Agent-Bridge-TAD.md` | 技术架构设计文档（权威来源） |
| DB Schema | `db/source/IM-Agent-Bridge-TAD.md` | 数据库设计（从 TAD §8 提取，与架构文档同源） |

> 💡 仅当源文档（PRD/架构等）变化时才需更新 `.context/`；业务代码变更不触发重生成。
