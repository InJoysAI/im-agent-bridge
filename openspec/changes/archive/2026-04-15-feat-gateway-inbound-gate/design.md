## Context

`feat-gateway-inbound-gate` 在 Gateway 骨架（`feat-infra-gateway-scaffold`）与 DB 连接层（`feat-gateway-db-layer`）就绪后，建立 `POST /gateway/inbound` 入站网关的安全边界与流量管控层。涉及三个非平凡设计决策：Bearer Token 比较方式、Token Bucket 限流器实现方案、以及 API 契约生成链路。

## Goals / Non-Goals

- Goals:
  - constant-time Bearer Token 校验，防止时序侧信道攻击（RISK-006）
  - 轻量内存 Token Bucket 限流器，按 chat_id 独立计数，LRU 清理防内存泄漏
  - `SSoT/api/main.tsp` TypeSpec 契约与 `InboundRequest` Rust struct 一致性验证
- Non-Goals:
  - 跨进程/跨实例共享限流状态（MVP 单进程，不引入 Redis）
  - Token 自动轮换机制（手动轮换，MVP）
  - OpenAPI → Rust 自动代码生成（手写 struct，仅验证契约一致性）

## Decisions

- **Decision: Bearer Token 使用 `constant_time_eq` crate 进行恒时比较**
  - 原因：字符串直接比较（`==`）在不等长或首字节不同时会短路，可被时序侧信道攻击利用，推断 Token 前缀。`constant_time_eq` 保证比较时间与字节内容无关（RISK-006；security_policy.md §API 安全）。
  - Alternatives considered：`hmac::Hmac` HMAC 比较（过重，Bearer Token 是对称秘密，无需 HMAC 语义）；直接 `==` 比较（存在侧信道风险，拒绝）。

- **Decision: Token Bucket 使用 `Arc<Mutex<HashMap<String, TokenBucket>>>` 内存实现，LRU 策略驱逐 60s 未活跃的键**
  - 原因：MVP 单进程部署，无跨实例限流需求；内存方案零依赖、零延迟。LRU 驱逐（每次请求时顺带清理超时键）防止长尾 chat_id 累积导致内存无限增长。criterion.md MUST NOT 明确禁止引入 Redis（缓存层）。
  - Alternatives considered：`governor` crate（GCRA 算法，较复杂，MVP 过重）；`dashmap`（无锁，可在性能瓶颈时升级）；Redis（MUST NOT）。
  - 限流参数：capacity=5 tokens，refill_rate=5 tokens/sec（对应 BR-055 阈值），窗口 1s。

- **Decision: `gateway/src/models/inbound.rs` 内联手写 Rust model structs，顶部注释指向 SSoT**
  - 原因：实施过程中评估了 `openapi-generator-cli -g rust` 的输出——工具生成完整 Rust 子 crate（含 `Cargo.toml`、`apis/` 模块、reqwest 依赖），对仅需 model structs 的场景严重过重（RISK-007 回退方案）。本项目 API contract 由我们自己控制、model 数量少且稳定（3 个 struct、2 个 enum），手写成本极低；结合顶部 SSoT 注释和 `make api-compile` 契约验证，可持续保证字段一致性。
  - 实现细节：
    1. `gateway/src/models/inbound.rs` 直接定义 `InboundRequest`、`RawMessage`、`ChatType`、`MessageType`、`InboundResponse`、`InboundStatus`，字段与 `SSoT/api/main.tsp` 严格对齐
    2. 文件顶部注释：`// SSoT: SSoT/api/main.tsp — 变更时先修改 SSoT 再更新此文件`
    3. `gateway/src/generated/` 加入 `.gitignore`（`make api-gen-rs` 生成产物供参考，不纳入编译树）
    4. `make api-compile` 保持有效，TypeSpec → OpenAPI YAML 契约验证链路完整
  - Alternatives considered：`openapi-generator-cli -g rust` 生成子 crate（产出整个 Rust 项目骨架，过重，RISK-007 回退路径）；`typify` crate 的 build.rs 集成（仅生成类型，无额外 crate，适合字段多且频繁变动的场景，可在 model 数量增长后升级）；直接使用生成产物作为路径依赖（评估后因 reqwest 版本冲突和额外编译开销放弃）。

- **Decision: 非文本消息拦截在 Deserialize 后、Bearer 校验通过后立即执行**
  - 原因：Deserialize 后方可读取 `message_type` 字段；Bearer 校验先于业务逻辑（任何未认证请求都不解析 body）；拦截点在 `message_type` 读取处，早于 DB 查询与 Runtime 调用，代价最小（BR-001）。

## Risks / Trade-offs

- Token Bucket 内存存储：Gateway 重启后限流计数器清零，窗口期内重启可被短暂绕过。MVP 可接受（重启场景极低频）。
- `HashMap` + `Mutex` 在高并发下存在锁争用。MVP 消息量预期低，可接受；若成为瓶颈可升级至 `dashmap`。

## Migration Plan

无数据迁移。本变更不引入新 DB Schema 变更（不新增 Goose 迁移文件）。

## Open Questions

- `make api-gen-rs` 完整 TypeSpec → Rust codegen 接入时机：由 `feat-runtime-nanobot-adapter` 或 `feat-runtime-reply-bridge` 提案决策，本提案仅预留 Makefile target。
