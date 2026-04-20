# runtime-persistence Specification

## Purpose
TBD

## Requirements
### Requirement: runtime_logs 条件写入
系统必须（MUST）仅在 NanoBotAdapter 完成 Runtime 调用且结果为 `status=error` 时，向 `runtime_logs` 写入行记录；正常调用（`status=success`）不得创建任何 `runtime_logs` 行。

#### Scenario: 正常调用不写入行
- **WHEN** NanoBotAdapter 完成 Runtime 调用，结果为 `status=success`
- **THEN** `runtime_logs` 表中不新增任何行记录

#### Scenario: 错误调用写入行
- **WHEN** NanoBotAdapter 调用 Runtime 失败（超时 / 不可达 / 响应格式异常），`status=error`
- **THEN** `runtime_logs` 写入一行记录，含 `event_id`、`bot_id`、`runtime_type`、`status='error'`、`error_code`、`latency_ms`
- **AND** `request_payload` 和 `response_payload` 均已脱敏写入（无 PII 字段）

---

### Requirement: PII 脱敏
系统必须（MUST）在将 `request_payload` / `response_payload` 写入 `runtime_logs` 之前，移除所有 PII 字段。

#### Scenario: request_payload 脱敏
- **WHEN** `status=error`，准备写入 `request_payload`
- **THEN** `request_payload` 仅包含白名单内的安全字段：`session_id`、`event_id`、`runtime_type`、`model`
- **AND** `user_id`、`input_text` 及其他非白名单字段均不出现在 `request_payload` 中

#### Scenario: response_payload 脱敏
- **WHEN** `status=error`，准备写入 `response_payload`
- **THEN** `response_payload` 仅包含白名单内的安全字段：`error_type`、`error_message`、`status_code`
- **AND** `error_message` 必须截断至最多 **512 字符**，超出部分替换为 `"...[truncated]"`
- **AND** `error_message` 中符合以下模式的敏感片段必须替换为 `"[REDACTED]"`（见 security_policy.md 凭证禁止入日志）：
  - Bearer Token：`Bearer\s+[A-Za-z0-9._\-]+`
  - Shopify secret/token：`shp[a-zA-Z]+_[0-9a-fA-F]{32,64}`
- **AND** 其余非白名单字段（含可能的用户消息回显）均不出现在 `response_payload` 中

---

### Requirement: latency_ms 记录
系统必须（MUST）在写入 `runtime_logs` 行时记录 Runtime 调用耗时。

#### Scenario: 错误调用记录 latency_ms
- **WHEN** NanoBotAdapter 调用 Runtime 失败，写入 `runtime_logs`
- **THEN** `runtime_logs.latency_ms` 为非负整数，单位毫秒，反映从发起 Runtime HTTP 请求到最终失败的总耗时（**含所有重试/降级**）

---

### Requirement: runtime_logs 写入失败不阻断主链路
系统必须（MUST）确保 `runtime_logs` 写入失败时不影响主消息处理链路。

#### Scenario: DB 写入失败不阻断
- **WHEN** 写入 `runtime_logs` 时 PostgreSQL 操作返回错误
- **THEN** Gateway 记录 `tracing::warn!` 告警日志，继续完成消息回写流程
- **AND** 递增可观测指标计数器 `runtime_log_write_failures_total`（供 Phase 4 Prometheus 采集，见 cross_cutting_concepts.md 可观测指标体系）
- **AND** 不向用户返回任何与 `runtime_logs` 写入相关的错误信息
