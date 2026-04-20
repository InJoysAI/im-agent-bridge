## ADDED Requirements

### Requirement: channel_bindings bot_id 解析
系统必须（MUST）根据入站消息的来源三元组 `(platform, bridge_gateway_name, bridge_channel_name)` 查询 `channel_bindings` 表解析出 `bot_id`，采用"精确匹配 → COALESCE 降级匹配 → 404"三级策略（BR-004）。channel_bindings 是 bot_id 的解析源头，查询谓词为来源三元组；一旦解析出 bot_id 后，对 sessions 及后续表的所有读写必须携带 bot_id 过滤条件（BR-032）。

#### Scenario: 精确匹配 channel_bindings 成功
- **WHEN** 入站消息的 `(platform, bridge_gateway_name, bridge_channel_name)` 三元组在 `channel_bindings` 表中存在精确启用记录（`is_enabled = true`）
- **THEN** 返回对应的 `bot_id`，不触发降级匹配
- **AND** 处理继续进入 session 生成阶段

#### Scenario: COALESCE 降级匹配 channel_bindings 成功
- **WHEN** 精确三元组匹配无结果，但 `(platform, bridge_gateway_name, bridge_channel_name IS NULL)` 存在宽泛网关级别绑定
- **THEN** 降级解析出正确的 `bot_id`
- **AND** `session_id` 按正常规则生成，不受降级影响

#### Scenario: channel_bindings 完全缺失返回 404
- **WHEN** 精确匹配与降级匹配均无结果（`channel_bindings` 中无对应启用记录）
- **THEN** 返回 HTTP 404
- **AND** 记录 WARN 级别日志，含 `platform` / `bridge_gateway_name` / `bridge_channel_name` 字段
- **AND** 不继续处理（不调用 Runtime，不写 `sessions` 表）

---

### Requirement: session_id 生成规则
系统必须（MUST）根据 `chat_type` 按规定格式生成 `session_id`，私聊与群聊必须严格隔离，不得相互污染（BR-010, BR-011, BR-012, BR-013）。

#### Scenario: 私聊消息生成 session_id
- **WHEN** 入站消息 `chat_type = "private"`，`chat_id = "123456"`
- **THEN** `session_id = "telegram:private:123456"`

#### Scenario: 群聊消息生成 session_id
- **WHEN** 入站消息 `chat_type = "group"`，`chat_id = "789012"`
- **THEN** `session_id = "telegram:group:789012"`

#### Scenario: 私聊与群聊上下文严格隔离
- **WHEN** 同一 `chat_id` 分别以 `chat_type = "private"` 和 `chat_type = "group"` 发送消息
- **THEN** 生成两个不同的 `session_id`（`telegram:private:{chat_id}` 与 `telegram:group:{chat_id}`）
- **AND** 两者在 `sessions` 表中为独立记录，不共享 `runtime_session_key`

---

### Requirement: sessions 表 upsert 幂等
系统必须（MUST）在 channel 解析成功后对 `sessions` 表执行 upsert，基于 `(bot_id, session_id)` 联合唯一约束保证幂等，且所有操作携带 `bot_id` 隔离条件（BR-032, BR-040）。

#### Scenario: 新 session 首次创建
- **WHEN** `session_id` 在 `sessions` 表中不存在
- **THEN** 插入新记录，字段 `session_id` / `bot_id` / `platform` / `chat_id` / `chat_type` 填充完整
- **AND** `created_at` 和 `updated_at` 设为当前时间

#### Scenario: 已有 session upsert 幂等更新
- **WHEN** 相同 `session_id` 的消息再次到达
- **THEN** 更新 `updated_at` 为当前时间，不重复插入
- **AND** 返回现有 `session_id`，不报错，不触发唯一约束异常

#### Scenario: BR-032 bot_id 隔离：不同 Bot 的相同 session_id 独立存储
- **WHEN** 两个不同 `bot_id` 各自管理相同的 `chat_id`，产生相同格式的 `session_id`（如均为 `telegram:private:12345`）
- **THEN** 两者以 `(bot_id, session_id)` 联合唯一约束（迁移 `00003`）各自独立插入，不发生冲突
- **AND** 任何 sessions 查询均以 `bot_id` 作为过滤条件，两 Bot 数据互不可见（BR-032）
