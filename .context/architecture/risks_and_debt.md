# 风险与技术债务 (Risks and Technical Debt)

> **Metadata**
> - **Source**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **Derived from**: `.context/architecture/source/IM-Agent-Bridge-TAD.md`
> - **性质**: 基于 TAD 的补充性风险/债务追踪，非 TAD 原文逐条可追溯
> - **Generated At**: `2026-04-13 13:52`
> - **Generator**: `Context-Agent v1.0`

---

## ⚠️ 技术风险

### 风险矩阵

| 影响程度 ↓ / 可能性 → | 低 | 中 | 高 |
|----------------------|----|----|-----|
| **高** | 中风险 | 高风险 | 极高风险 |
| **中** | 低风险 | 中风险 | 高风险 |
| **低** | 可忽略 | 低风险 | 中风险 |

### 风险清单

#### RISK-001: Runtime 单点故障

| 属性 | 值 |
|------|-----|
| **类别** | 架构风险 |
| **可能性** | 中 |
| **影响程度** | 高 |
| **风险等级** | 高风险 |
| **描述** | NanoBot 作为 MVP 唯一 Runtime，若进程崩溃或资源耗尽，所有消息处理中断 |
| **缓解措施** | 15s hard timeout 兜底 + 错误提示回写 + Docker restart policy |
| **应急预案** | 手动重启 NanoBot 容器 |

#### RISK-002: NanoBot 本地状态丢失

| 属性 | 值 |
|------|-----|
| **类别** | 架构风险 |
| **可能性** | 中 |
| **影响程度** | 中 |
| **风险等级** | 中风险 |
| **描述** | NanoBot 会话历史存储在本地磁盘 (`~/.local/state/nano-bots/`)，容器重建或磁盘故障将丢失所有会话上下文 |
| **缓解措施** | Docker Volume 挂载持久化目录 |
| **应急预案** | 会话丢失后用户需重新建立上下文（功能不中断，仅上下文丢失） |

#### RISK-003: Shopify MCP 可用性依赖

| 属性 | 值 |
|------|-----|
| **类别** | 集成风险 |
| **可能性** | 中 |
| **影响程度** | 中 |
| **风险等级** | 中风险 |
| **描述** | Shopify MCP 依赖 Shopify API 可用性，API 限流或宕机将导致工具调用失败 |
| **缓解措施** | MCP 10s 超时 + "工具暂不可用"友好提示 + `mcp_call_error_total` 指标监控 |
| **应急预案** | Runtime 退化为纯对话模式（无工具能力） |

#### RISK-004: PostgreSQL 不可用导致全面熄断

| 属性 | 值 |
|------|-----|
| **类别** | 架构风险 |
| **可能性** | 低 |
| **影响程度** | 高 |
| **风险等级** | 中风险 |
| **描述** | PostgreSQL 不可用时 Gateway 短路熄断所有入站请求，系统完全停止服务 |
| **缓解措施** | Docker restart policy + `db_unavailable_total` 告警 + 日志监控 |
| **应急预案** | 快速恢复 PostgreSQL 实例；设计上"宁可报错不可错乱"，不做无 DB 降级 |

#### RISK-005: Matterbridge 桥接稳定性

| 属性 | 值 |
|------|-----|
| **类别** | 集成风险 |
| **可能性** | 低 |
| **影响程度** | 高 |
| **风险等级** | 中风险 |
| **描述** | Matterbridge 作为唯一桥接器，若崩溃或 Telegram 连接断开，消息入站/回写完全中断 |
| **缓解措施** | Docker restart policy + 回写重试 3 次指数退避（1s/2s/4s）；入站由 `/api/stream` 改为 `/api/messages` 轮询模式（`feat-runtime-reply-bridge` 联调，消除订阅者状态导致的重启依赖）；回写 at-most-once（仅对可重试错误重试，减少重复投递） |
| **应急预案** | 手动重启 Matterbridge 容器 |

#### RISK-006: Bearer Token 泄露

| 属性 | 值 |
|------|-----|
| **类别** | 安全风险 |
| **可能性** | 低 |
| **影响程度** | 高 |
| **风险等级** | 中风险 |
| **描述** | Bridge ↔ Gateway 的 Bearer Token 若泄露，攻击者可伪造入站消息 |
| **缓解措施** | 环境变量注入 + 禁止代码仓库 + 内网优先 + 白名单来源限制 |
| **应急预案** | 立即轮换 Token + 审查 message_events 异常记录 |

#### RISK-007: TAD 设计与第三方工具实际能力差距

| 属性 | 值 |
|------|-----|
| **类别** | 架构风险 |
| **可能性** | 高 |
| **影响程度** | 中 |
| **风险等级** | 高风险 |
| **描述** | TAD 设计与实际工具能力存在差距，需在实施阶段开发适配层 |
| **差距明细** | • TAD Push 模型 (POST /gateway/inbound) vs Matterbridge 原生 Pull API (GET /api/messages 轮询) —— **已落地**：`feat-runtime-reply-bridge` 联调由 `/api/stream` 改为 `/api/messages` 定期轮询，消除订阅者状态重启依赖 |
| | • TAD `/bridge/reply` 代理端点 vs Matterbridge 1.26 原生 `POST /api/message` —— **已落地偏差**：`feat-runtime-reply-bridge` 直接对接原生端点，SSoT 对齐留给 `fix-bridge-reply-ssot-align` |
| | • TAD MEMORY.md MCP 声明 vs HKUDS/nanobot config.json → tools.mcpServers —— 需开发配置映射逻辑 |
| | • TAD 存储路径 `~/.local/state/nano-bots/` vs HKUDS/nanobot 实际路径 `~/.nanobot/` |
| **缓解措施** | 在实施提案中明确每个差距的适配方案和工作量估算 |
| **应急预案** | 如适配复杂度超出预期，可提议修订 TAD 以缩小差距 |

---

## 🔧 技术债务

### TD-001: Gateway ↔ Runtime 无认证

| 属性 | 值 |
|------|-----|
| **类型** | 安全债务 |
| **严重性** | 中 |
| **描述** | MVP 阶段 Gateway ↔ Runtime 使用裸 HTTP，无 Token/mTLS |
| **影响** | 同一网络中的其他服务可直接调用 Runtime |
| **目标解决时间** | Post-MVP |
| **解决方案** | 添加 Token 或 mTLS 认证 |

### TD-002: 无独立管理后台

| 属性 | 值 |
|------|-----|
| **类型** | 架构债务 |
| **严重性** | 中 |
| **描述** | Bot 配置、Channel 绑定等均通过直接 SQL 管理，无 UI 界面 |
| **影响** | 运维效率低，非技术人员无法操作 |
| **目标解决时间** | Post-MVP |
| **解决方案** | 构建 Admin API + Web 管理后台 |

### TD-003: 单 IM 单 Runtime 限制

| 属性 | 值 |
|------|-----|
| **类型** | 架构债务 |
| **严重性** | 低 |
| **描述** | MVP 仅支持 Telegram + NanoBot，架构已预留多 IM/多 Runtime 扩展点但未实现 |
| **影响** | 无法接入其他 IM 平台或替换 Runtime |
| **目标解决时间** | v2.0 |
| **解决方案** | 实现 Channel Adapter 抽象 + Runtime Adapter 多实现 |

### TD-004: 超时后不取消下游请求

| 属性 | 值 |
|------|-----|
| **类型** | 架构债务 |
| **严重性** | 低 |
| **描述** | Gateway 超时后不取消 Runtime 下游请求（MVP 简化），超时的 Runtime 请求仍在执行 |
| **影响** | 资源浪费，Runtime 继续处理已超时的请求 |
| **目标解决时间** | Post-MVP |
| **解决方案** | 实现 CancellationToken / Abort 机制 |

### TD-005: 数据保留清理依赖手动

| 属性 | 值 |
|------|-----|
| **类型** | 运维债务 |
| **严重性** | 低 |
| **描述** | message_events (30天) / runtime_logs (14天) / sessions 的清理规则已定义，但 MVP 阶段可能需手动执行 |
| **影响** | 数据无限增长，磁盘占用升高 |
| **目标解决时间** | Post-MVP |
| **解决方案** | 分表追踪消减路径：`runtime_logs`（14 天）已由 `feat-runtime-log-retention` 落地；`message_events`（30 天）已由 `feat-message-event-retention` 落地；`sessions` 清理由后续提案 `feat-session-cleanup` 承接（pending） |

### TD-007: Bridge ↔ Gateway 无 TLS（MVP 私网部署）

| 属性 | 值 |
|------|-----|
| **类型** | 安全债务 |
| **严重性** | 中 |
| **描述** | MVP 阶段 Bridge ↔ Gateway 使用裸 HTTP + Bearer Token，仅依赖私有网络（VPN/云 VPC）隔离，无传输层加密 |
| **影响** | 私有网络被攻破时 Bearer Token 可能被窃听；Matterbridge 与 Gateway 间流量明文传输 |
| **目标解决时间** | Post-MVP |
| **解决方案** | 由独立后续提案 `feat-bridge-tls-upgrade` 承接：Gateway 启用 HTTPS（私有 CA 或受控证书）+ Matterbridge 配置证书信任，或部署轻量 TLS 终止代理 |

### TD-006: 群聊不按用户拆分上下文

| 属性 | 值 |
|------|-----|
| **类型** | 功能债务 |
| **严重性** | 低 |
| **描述** | 群聊场景共享 session_id，所有群成员共用 AI 上下文 |
| **影响** | 多人群聊场景下上下文混乱 |
| **目标解决时间** | v2.0 |
| **解决方案** | 引入 `thread_id` 或按 `user_id` 拆分群聊 session |

---

## 📊 追踪看板

| ID | 类型 | 名称 | 严重性 | 状态 | 目标版本 |
|----|------|------|--------|------|---------|
| RISK-001 | 风险 | Runtime 单点故障 | 高 | 监控中 | MVP |
| RISK-002 | 风险 | NanoBot 本地状态丢失 | 中 | 监控中 | MVP |
| RISK-003 | 风险 | Shopify MCP 可用性依赖 | 中 | 监控中 | MVP |
| RISK-004 | 风险 | PostgreSQL 不可用全面熄断 | 中 | 监控中 | MVP |
| RISK-005 | 风险 | Matterbridge 桥接稳定性 | 中 | 监控中 | MVP |
| RISK-006 | 风险 | Bearer Token 泄露 | 中 | 监控中 | MVP |
| RISK-007 | 风险 | TAD 设计与工具实际能力差距 | 高 | 监控中 | MVP |
| TD-001 | 技术债 | Gateway ↔ Runtime 无认证 | 中 | 待处理 | Post-MVP |
| TD-002 | 技术债 | 无独立管理后台 | 中 | 待处理 | Post-MVP |
| TD-003 | 技术债 | 单 IM 单 Runtime 限制 | 低 | 待处理 | v2.0 |
| TD-004 | 技术债 | 超时后不取消下游请求 | 低 | 待处理 | Post-MVP |
| TD-005 | 技术债 | 数据保留清理依赖手动 | 低 | 待处理 | Post-MVP |
| TD-006 | 技术债 | 群聊不按用户拆分上下文 | 低 | 待处理 | v2.0 |
| TD-007 | 技术债 | Bridge ↔ Gateway 无 TLS | 中 | 待处理 | Post-MVP |

---

## AI 引用指南

当 AI 进行架构规划时：
1. 检查风险清单，避免触发已识别风险
2. 优先解决高严重性技术债务
3. 在提案中标注相关风险缓解措施
4. 新增功能时评估对现有风险的影响
