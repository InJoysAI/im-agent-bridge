## base

### 产品需求树（PRD）与技术架构图（TAD）双向一致性审查 
```
/context-check review 请对 `docs` 目录下的核心文档（`IM-Agent-Bridge-PRD.md` 与 `IM-Agent-Bridge-TAD.md`）执行 **全量深度一致性与完备性交叉审查**，验证产品需求与技术架构之间的严格对齐关系。

**核心目标**
以 PRD 为业务域基准，以 TAD 为技术实现域基准，进行双向逐章比对。识别出产品需求在技术架构中的“遗漏”（未被支撑的功能）、“冲突”（技术设计违背业务规则）以及技术架构中的“冗余”（无业务价值的过度设计）或“臆造”（凭空出现的技术约束），生成一份具备高度可操作性的审查报告，确保系统设计逻辑严格自洽。

**审查范围**
- 业务域权威源：`.context/domain/source/IM-Agent-Bridge-PRD.md`
- 技术域权威源：`.context/architecture/source/IM-Agent-Bridge-TAD.md`

**审查维度与重点**

---

**维度一：业务需求与技术架构的链路闭环验证**
1. **核心场景流转对齐**
   - 提取 PRD 中定义的所有核心业务流程（如：初始化绑定、消息收发链路、告警推送等），验证 TAD 中是否都有对应的系统交互时序图、数据流向描述支撑。
   - 检查高频链路在架构中是否具备合理的性能和稳定性保障设计。

2. **边界与异常场景映射**
   - 对照 PRD 中列出的各类异常场景（如：降级态、高并发、接口阻断），验证 TAD 是否在架构层面设计了对应的熔断、重试、回滚或降级策略，且策略描述无矛盾。

---

**维度二：数据模型与 API 契约的镜像一致性**
3. **数据结构映射**
   - 提取 PRD 中涉及的业务实体对象与其关键属性要素，核对 TAD 的数据库 Schema / 数据结构设计是否一一映射，字段名称与含义是否存在偏差或是遗漏。

4. **API 与功能覆盖**
   - 核对 TAD 中规划的关键 API 端点，是否完美覆盖了 PRD 所要求前后端或三方系统的全部交互诉求。检查是否存在方向颠倒、权限错位的情况。

---

**维度三：术语与系统安全的一致性**
5. **Ubiquitous Language (通用语言) 统一校验**
   - 强制扫描两份文档，比对高频核心业务名词（如 Agent、Bot、Tenant、Store 等）、状态枚举（Online、Offline、Degraded 等），挑出在 PRD 和 TAD 中存在表述分歧的地方。

6. **安全与非功能性需求（NFR）对齐**
   - 从 PRD 提取权限控制、凭证管理（Token存放形式）、多租户隔离约束、PII 数据脱敏要求等安全基线。
   - 穿透核对待查 TAD，验证架构上的加解密方案、RLS、身份上下文传递等技术策略能否完全满足（或超过）PRD 定义的标准。

---

**执行步骤**

1. **结构化提取**：分别解析 PRD 和 TAD 两份文档的结构，提取核心实体、用例、时序和接口清单。
2. **正向映射 (PRD -> TAD)**：验证每一个需求是否都有明确的技术落地方案（防遗漏）。
3. **逆向追溯 (TAD -> PRD)**：验证每一个底层设计是否都能关联到具体的业务价值或系统质量要求（防冗余或过度设计）。
4. **精细校验**：拉通比对命名、数值、关键限制与约束（Rate Limit / 软硬删规则等）。
5. **意图判断与分级**：评估产生的 Gap，定性并归类为 `[MISSING]`(缺失), `[CONFLICT]`(冲突), `[DEVIATION]`(无意偏离), `[UNJUSTIFIED_TECH]`(未经论证的技术设计)。
6. **产出报告**。

**输出报告结构**

1. **一致性健康度摘要**
   - 给出总体评级（高/中/低），提炼前三的严重断层问题。

2. **核心业务流对齐矩阵**
   | PRD 业务场景 | TAD 对应技术模块 / 接口 | 覆盖状态 (✅完整 / ⚠️部分 / ❌缺失) | 简述偏差 |

3. **双向不一致差异清单 (核心交付)**
   - **PRD 位置**：(章节与具体功能点)
   - **TAD 位置**：(章节与具体技术实现)
   - **差异类型标签**：`[MISSING]` / `[CONFLICT]` / `[DEVIATION]` / `[UNJUSTIFIED]`
   - **严重程度**：(P0 阻塞/ P1 重要/ P2 建议)
   - **具体描述**：精炼描述偏差现象所在。

4. **术语级统一性核定表**
   | 冲突术语 | PRD 定义语境 | TAD 定义语境 |

5. **行动建议与跟进 (Action Items)**
   - 针对各条问题指出明确的**修改侧**：如“修改 PRD 以妥协技术性能极限” 还是 “修改 TAD 以填补业务漏洞”。

**质量要求**
- 所有的结论、差异指纹 **必须** 带上精确的章节引用与对应内容对比，绝不能泛泛而谈。
- 对于明显是由于技术文档书写滞后而造成的失真，需明确指出修订方案。
- 请以中文进行详尽输出，务必严谨、专业。

```

### 全量核查 .context 资产目录（不含 source）
```
/context-check review 请对 `.context/` 目录下的 **全部生成资产**（不含各模块 `source/` 子目录下的源文档）执行跨模块全量一致性审查，验证所有资产之间的定义统一性、引用正确性和 Manifest/索引同步状态。

**核心目标**
以 `.context/context-manifest.json` 为基准清单，遍历 `.context/` 下所有生成文件（排除 `*/source/*`），对每个模块内部的逻辑自洽性和模块间的交叉引用一致性进行系统性校验，识别术语矛盾、定义冲突、覆盖遗漏和索引失同步，生成一份可操作的全量审查报告。

**审查范围**

| 模块 | 生成文件（不含 source/） |
|------|------------------------|
| **Root** | `README.md`, `AGENTS.md`, `criterion.md`, `context-manifest.json` |
| **Architecture** | `README.md`, `system_design.md`, `tech_stack.md`, `security_policy.md`, `api_strategy.md`, `runtime_view.md`, `deployment_view.md`, `cross_cutting_concepts.md`, `risks_and_debt.md` |
| **Domain** | `README.md`, `domain_model.md`, `business_rules.md`, `user_journeys.md`, `edge_cases.md`, `testing_strategy.md`, `risks_and_debt.md` |
| **DB** | `README.md`, `schema_design.md`, `performance_tuning.md`, `migrations_and_ssot.md`, `security_hardening.md`, `observability.md` |
| **OpenSpec** | `openspec/integration.md` |

> UI / Legacy 模块当前无生成文件，跳过。

**审查维度**

---

**维度一：术语与定义统一性（跨模块）**

1. **通用术语一致性**
   - 以 `domain/domain_model.md` 和 `domain/business_rules.md` 为核心术语参考，检查所有文件中的业务术语是否统一，无别名/近义词混用
   - 重点核查以下易混用术语：
     - `Bot` / `bot` / `bot_id`（首字母大写规范与代码标识符的使用场景）
     - `channel_binding`（概念/错误语义）vs `channel_bindings`（表名）
     - `session_id` / `Session` / `runtime_session_key`（三者职责区分是否一致）
     - `event_id` / `message_id` / `bridge_message_id` / `reply_id`（四个 ID 语义是否在所有引用处一致）
     - `platform`（字段值如 `telegram`）与 `platform` 作为架构概念层的使用是否混淆
     - `Matterbridge` / `Bridge` / `bridge`（大小写与指代是否一致）
     - `NanoBot` / `nanobot` / `Runtime`（`runtime_type` 值与架构描述、代码标识符的一致性）
     - `Gateway` 作为组件名称（首字母大写规范）

2. **状态值定义一致性**
   - 提取以下状态集合，逐一对照在所有引用处的定义完全一致：
     - `message_events.status`（如 `pending` / `processing` / `done` / `error`）：在 `domain/business_rules.md`、`db/schema_design.md`、`architecture/runtime_view.md` 中是否枚举一致
     - `message_events.reply_status`（如 `success` / `reply_failed`）：在 `domain/business_rules.md`、`db/schema_design.md`、`domain/edge_cases.md` 中是否一致
     - `runtime_logs.status` 值：与 `architecture/cross_cutting_concepts.md` 错误处理描述是否一致

3. **安全约束一致性**
   - 以 `criterion.md` 安全约束段 和 `architecture/security_policy.md` 为基准，交叉验证以下约束在所有引用处的表述一致性：
     - Bearer Token 认证：无降级路径，校验失败必须返回 401，不得继续处理
     - PII 处理：`input_text`/`output_text` 落库前截断至 512 字符；`runtime_logs.request_payload`/`response_payload` 仅 `status=error` 时写入且必须脱敏 PII
     - 幂等机制：入站幂等键 `(platform, bridge_gateway_name, COALESCE(bridge_channel_name,''), bridge_message_id)` 和回写幂等键 `reply_id` 在所有引用处定义一致
     - DB 熔断：PostgreSQL 不可用时 Gateway 必须 503 短路，不得继续处理任何业务请求
   - 确认 `db/security_hardening.md` 的安全描述与 `architecture/security_policy.md` 无矛盾

---

**维度二：数据模型与 Schema 跨模块一致性**

4. **实体-表映射一致性**
   - 对照 `domain/domain_model.md` 的核心实体（Bot、ChannelBinding、Session、MessageEvent、RuntimeLog）与 `db/schema_design.md` 的表定义，验证：
     - 每个领域实体是否有对应表，命名是否一致
     - 字段名、数据类型、约束是否从领域模型准确映射
   - 验证 `db/schema_design.md` 中的 ER 图（Mermaid erDiagram）与其下方各表的字段定义表格完全对齐（字段名、PK/FK/UK 标注）

5. **迁移与 Schema 文档一致性**
   - 验证 `SSoT/schema/migrations/00001_init.sql` 中的实际 DDL（CREATE TABLE + CREATE INDEX）与 `db/schema_design.md` 表定义的字段、类型、约束完全一致，无遗漏或多出字段
   - 验证 `db/schema_design.md` 索引清单中所有索引在 `00001_init.sql` 的 Index 段均有对应创建语句（名称、列、UNIQUE 属性）
   - 检查 `db/migrations_and_ssot.md` 的 `实现决策记录（IMPL-001）` 中列出的影响文件与实际变更文件一致
   - 检查 `db/README.md` 的表设计摘要（关键字段列表）是否与 `db/schema_design.md` 同步

---

**维度三：架构与约束传播一致性**

6. **技术栈约束传播**
   - 以 `criterion.md` 技术栈约束段 为权威源，验证 `architecture/tech_stack.md` 描述是否完全一致（编程语言、Web 框架、数据库、消息通道、迁移工具）
   - 检查 `architecture/system_design.md`、`architecture/deployment_view.md` 中引用的技术组件是否与 `tech_stack.md` 一致

7. **API 契约一致性**
   - 验证 `architecture/api_strategy.md` 定义的入口端点、认证方式、幂等策略（§1.5）、bot_id 解析逻辑（§1.6）与 `architecture/runtime_view.md` 序列图中的处理步骤完全一致
   - 验证 `domain/user_journeys.md` 中描述的消息处理流程与 `architecture/runtime_view.md` 的场景 1（正常处理）对齐
   - 检查 HTTP 状态码语义（200/400/401/404/429/500/502/503）在 `architecture/api_strategy.md` 的状态码表与 `domain/edge_cases.md` 的异常场景描述中一致

8. **错误处理一致性**
   - 以 `architecture/cross_cutting_concepts.md` 或 `db/schema_design.md` 中的错误码枚举（TAD §9.2）为基准：
     - 验证 `domain/edge_cases.md` 中的异常场景使用了一致的错误码/类型
     - 验证 `architecture/runtime_view.md` 场景 2（Runtime 异常）的错误处理描述与 `domain/business_rules.md` 中的规则一致
   - 验证限流规则（5 msg/sec/chat_id）在 `domain/business_rules.md`、`architecture/api_strategy.md`、`architecture/runtime_view.md` 三处描述完全一致

---

**维度四：DB 规范内部一致性**

9. **Schema 与索引文档自洽**
   - 验证 `db/schema_design.md` 索引清单（`## 🔍 索引策略`）中所有索引与 `SSoT/schema/migrations/00001_init.sql` 的 Index 段完全对应（索引名、列、UNIQUE 属性），无单方面多出或遗漏
   - 特别检查 `idx_channel_bindings_lookup` 是否在索引清单和迁移文件中同时存在且一致，且 `db/migrations_and_ssot.md` 中存在 `IMPL-001` 决策记录
   - 验证 `db/performance_tuning.md` 高频查询表（`## 高频查询优化目标`）中的索引引用名称与 `db/schema_design.md` 索引清单一致

10. **观测阈值与来源标注**
    - 检查 `db/observability.md` 中所有指标/阈值是否明确区分了来源（TAD 明确定义 vs 最佳实践推导），确认不存在未标注来源的阈值主张
    - 检查 `db/performance_tuning.md` 中所有延迟目标是否同样区分了 TAD 直接约束与推导值（如 < 50ms 为推导值）

11. **数据保留期一致性**
    - 验证 `message_events` 保留 30 天、`runtime_logs` 保留 14 天 在 `db/schema_design.md`（分区策略节）、`db/security_hardening.md`、`db/observability.md` 中的描述完全一致

---

**维度五：Manifest / 索引 / README 四方同步**

12. **文件清单同步验证**
    以下四处文件索引必须完全同步（不多不少）：
    - `.context/context-manifest.json` → `generated_files` 各数组
    - `.context/openspec/integration.md` → `CONTEXT_ASSET_INDEX` 区块
    - 各模块 `README.md` 中的文件索引表（`architecture/README.md`、`domain/README.md`、`db/README.md`）
    - `.context/` 下的**实际文件系统**（`find .context/ -type f -not -path '*/source/*'`）

    对每个模块分别输出四方对照结果，标注差异。

13. **元数据时效性**
    - 检查每份文件头部的 Metadata 区块（Source, Generated At, Generator）是否存在、格式一致
    - 检查 `context-manifest.json` 的 `last_modified` 是否合理反映最近一次有效变更

---

**维度六：自洽性与逻辑完备性**

14. **风险与债务交叉验证**
    - 对比 `architecture/risks_and_debt.md` 与 `domain/risks_and_debt.md`，验证同一风险项的描述/评级/缓解措施一致，无重复或矛盾

15. **测试策略覆盖验证**
    - 检查 `domain/testing_strategy.md` 中列出的测试场景是否覆盖了 `domain/edge_cases.md` 中定义的边界情况（重点：幂等重复、DB 熔断、Runtime 超时、限流触发）
    - 验证 `domain/business_rules.md` 中标记为 MUST 的规则是否在 `domain/testing_strategy.md` 中有对应验证项

16. **criterion.md 权威性验证**
    - 以 `criterion.md` 为最高权威，逐条 MUST/MUST NOT 规则扫描所有其他资产文件，确认无矛盾
    - 特别检查：DB 熔断（503 短路、不得继续处理）、幂等约束（双键设计）、Bearer Token 无降级、PII 截断 512 字符、Rate Limit 5 msg/sec/chat_id、`runtime_logs` 仅 error 写入且脱敏

---

**执行步骤**

1. **资产清点**：遍历 `.context/` 目录（排除 `*/source/*`），列出所有实际文件，与 `context-manifest.json` 的 `generated_files` 对照，标出差异。
2. **模块内审查**：对每个模块（architecture, domain, db），检查文件间的内部一致性。
3. **跨模块审查**：按维度一至维度四的交叉维度，逐条进行跨模块比对。
4. **索引同步审查**：执行维度五的四方同步验证。
5. **自洽性审查**：执行维度六的逻辑完备性检查。
6. **意图判断**：对每处差异标注：
   - `[CONSISTENT]` 完全一致
   - `[DEVIATION]` 无意偏离（术语不统一、数值不匹配等）
   - `[CONFLICT]` 定义矛盾（两处对同一概念的描述互相冲突）
   - `[GAP]` 引用缺失（某模块使用了其他模块未定义的概念）
   - `[DESYNC]` 索引失同步（Manifest/索引/实际文件不一致）
   - `[AMBIGUOUS]` 表述模糊（无法判断正确性，需人工确认）
7. **生成报告**：按以下结构输出。

**输出报告结构**

1. **全局一致性评分**
   | 维度 | 评级 | 关键发现 |
   |------|------|---------|
   | 术语统一性 | {{高/中/低}} | {{...}} |
   | 数据模型一致性 | {{高/中/低}} | {{...}} |
   | 架构约束传播 | {{高/中/低}} | {{...}} |
   | DB 规范一致性 | {{高/中/低}} | {{...}} |
   | 索引同步 | {{高/中/低}} | {{...}} |
   | 逻辑自洽性 | {{高/中/低}} | {{...}} |

2. **文件清单四方同步矩阵**
   对每个模块输出：
   | 文件 | manifest.json | integration.md | 模块 README | 实际文件系统 |
   |------|:---:|:---:|:---:|:---:|
   | xxx.md | ✅/❌ | ✅/❌ | ✅/❌ | ✅/❌ |

3. **跨模块差异清单**
   每条包含：
   - 差异类型标签（`[DEVIATION]` / `[CONFLICT]` / `[GAP]` / `[DESYNC]` / `[AMBIGUOUS]`）
   - 涉及文件（≥2 个文件路径）
   - 差异描述（具体到章节/字段/术语）
   - 严重程度（P0 = 影响系统正确性或安全 / P1 = 影响实现一致性 / P2 = 影响文档可读性）
   - 修复建议（应修改哪一方、如何修改）

4. **术语统一性矩阵**
   | 术语 | 权威定义来源 | 其他文件使用情况 | 一致/偏离 |
   |------|-------------|----------------|----------|

5. **Criterion 合规验证表**
   | MUST/MUST NOT 规则 | criterion.md 位置 | 相关资产覆盖状态 | 矛盾？ |
   |-------------------|------------------|----------------|--------|

6. **可操作修复建议**
   按 P0 → P1 → P2 优先级排序，每条包含：修复文件、修复内容、原因。

**质量要求**
- 所有结论必须引用具体文件路径 + 章节/字段名，禁止泛泛而谈
- `source/` 目录下的文件不在本次审查范围内（仅审查生成资产间的互洽性）
- 术语对比必须逐条列出，不可合并或跳过
- Criterion MUST/MUST NOT 规则必须逐条扫描
- 输出使用中文，文件引用使用相对路径（相对于 `.context/` 根目录）
```

### 检查 DB 资产和源文档的一致性
```
/context-check review 请对以下参数指定的数据库资产执行**全量一致性审查**。

---

## 📌 审查参数（使用前填写）

| 参数 | 值 |
|------|-----|
| DB 摘要目录 | `.context/db/` |
| 权威源文档目录 | `.context/db/source/` |
| 迁移文件目录 | `SSoT/schema/migrations/` |
| 实现代码路径（可选） | _(留空则跳过代码验证)_ |
| 迁移工具链 | _(如 Goose / Flyway / Alembic / 自定义)_ |
| 已知架构决策（可选） | _(如有背景信息，逐条列出，供意图判断参考)_ |

---

## 🎯 审查目标

逐文件比对 DB 摘要与源文档的内容一致性，并与实际 SQL 迁移文件（及代码实现，如有）三角验证，
识别**缺失 / 冲突 / 过时 / 臆造**四类差异，生成可操作的一致性审查报告。

---

## 🔍 审查维度

### 1. Schema 定义一致性
- 对比 [schema_design.md](.context/db/schema_design.md:0:0-0:0) 中的表结构（表名、字段名、类型、约束、索引）与源文档逐字段核对
- 验证 ER 图、外键、级联行为与源文档及实际 DDL 是否一致
- 检查字段类型精度（如 VARCHAR 长度、NUMERIC 精度）是否匹配

### 2. 迁移策略与 SSoT 对齐
- 核对 [migrations_and_ssot.md](.context/db/migrations_and_ssot.md:0:0-0:0) 描述的工具链、SSoT 目录、版本命名规范与实际迁移文件清单
- 验证每个迁移文件包含完整的 Up / Down（或等效回滚机制）
- 确认版本序列无跳号、无重复、时序一致

### 3. 安全策略一致性
- 对比 [security_hardening.md](.context/db/security_hardening.md:0:0-0:0) 与 `architecture/security_policy.md`（如存在）中的：
  认证方式、角色权限、PII 处理规则、数据保留期
- 验证凭证管理策略（Token/密钥不入库等）在 DB 与架构文档之间无矛盾

### 4. 性能配置一致性
- 核对 [performance_tuning.md](.context/db/performance_tuning.md:0:0-0:0) 中的配置参数与源文档定义
- 如提供实现代码，验证代码中的实际参数值与文档一致

### 5. 跨资产一致性
- 检查 DB 资产中的表名/字段名与 `architecture/`、`domain/` 中的引用是否统一
- 验证 [README.md](design/README.md:0:0-0:0) 文件索引与实际目录内容一致
- 验证 [context-manifest.json](.context/context-manifest.json:0:0-0:0) 中 [db](.context/db:0:0-0:0) 数组与实际文件完全同步

### 6. 代码实现验证（仅当参数表提供实现代码路径时执行）
- 逐表对照 [schema_design.md](.context/db/schema_design.md:0:0-0:0) 描述与实际 DDL，标注字段级一致 / 差异
- 确认迁移代码引用的文件路径可解析、内容符合预期

---

## ⚙️ 执行步骤

1. **资产清点** — 列举摘要目录与源文档目录所有文件，建立配对关系
2. **文件级对比** — 逐章节/逐字段深度对比，识别差异类型
3. **三角验证** — 源文档 ↔ 摘要 ↔ SQL/代码 三方交叉
4. **意图判断** — 结合「已知架构决策」参数，对每处差异标注：
   `[INTENTIONAL]` 有意迭代 / `[DEVIATION]` 无意偏离 / `[AMBIGUOUS]` 待确认
5. **生成报告**

---

## 📊 输出结构

1. **总体一致性摘要** — 整体符合程度（高/中/低）+ 核心发现 ≤ 5 条
2. **文件配对状态表** — ✅ 完全一致 / ⚠️ 存在差异 / ❌ 仅单侧存在
3. **差异清单** — 每条格式：
   `路径 → 章节 → 差异类型 → 描述 → P0/P1/P2 → [意图标签]`
4. **代码-文档对齐矩阵**（仅当执行步骤 6 时输出）
5. **可操作修复建议** — 仅针对 `[DEVIATION]` 和 `[AMBIGUOUS]`，说明修改哪一方、如何修改、优先级

---

## 📐 严重程度定义

| 级别 | 触发条件 |
|------|---------|
| **P0** | 影响 Schema 正确性或数据安全（字段缺失、类型错误、PII 泄露风险） |
| **P1** | 影响迁移一致性或实现对齐（版本跳号、路径不可解析、约束不匹配） |
| **P2** | 影响文档可读性（描述过时、索引失效、说明缺失） |

---

## ✅ 质量要求

- 所有结论必须与文件实际内容建立明确对应关系，**禁止泛泛而谈**
- 输出使用中文，文件引用使用**完整相对路径**
- 无法从文件内容直接推断的差异，一律标注 `[AMBIGUOUS]`，不得擅自推断意图
```

### 检查 UI 资产和源文档的一致性
```
/context-check review 请对 `.context/ui` 目录下的 UI 设计资产与 `.context/ui/source/` 目录下的原始 UI 设计规范文档执行 **全量一致性审查**，同时交叉验证 UI 资产与 PRD 用户旅程、架构约束之间的对齐性。

**核心目标**
逐文件比对 `.context/ui/` 设计资产与 `.context/ui/source/` 权威 UI 规范之间的内容一致性，并将两者与 PRD（`.context/domain/source/`）和架构文档（`.context/architecture/`）进行三角验证，识别所有形式的差异（遗漏、冲突、臆造、过时），生成一份可操作的一致性审查报告。

**审查范围**
- `.context/ui/` 下的生成文件：`design_system.md`、`stitch_design_system.md`、`stitch_prompts.md`、`atomic_components.md`、`layout_grid.md`、`interaction_states.md`、`STITCH_GUIDE.md`、`README.md`
- `.context/ui/source/` 下的权威源文档：`OpenFleet-CBEC-Shopify-Web-UIUX-V0.md`、`OpenFleet-CBEC-Shopify-Landing-Page-V0.md`
- 交叉验证文档：`.context/domain/source/OpenFleet-CBEC-Shopify-PRD-V0.md`（用户旅程、功能需求、验收标准）
- 交叉验证文档：`.context/architecture/system_design.md`、`.context/architecture/security_policy.md`（技术约束）
- `.context/context-manifest.json`（`generated_files.ui` 数组）
- `.context/openspec/integration.md`（`CONTEXT_ASSET_INDEX` 区块 UI 部分）

**审查维度与重点**

1.  **设计 Token 一致性**
    *   对比 `design_system.md` 和 `stitch_design_system.md` 中定义的颜色值（Primary `#2563EB`、Success `#10B981`、Warning `#F59E0B`、Error `#EF4444` 等）是否与源文档 §1.3 色板定义完全一致。
    *   验证两份设计系统文件之间的 Token 命名是否统一（CSS 变量 `--color-primary` vs Stitch Token `color-primary`），值是否一一对应，无矛盾。
    *   核对字体策略：系统字体栈是否与源文档 §1.2 完全一致，是否有遗漏或自行添加的字体。
    *   验证间距系统：4px 乘数间距值（4/8/12/16/24/32/48px）是否与源文档 §1.2 一致。
    *   检查是否存在源文档未给出、但资产中臆造了具体数值的情况（违反"不臆造具体色值/字号/tokens"规则）。

2.  **组件规范与交互状态对齐**
    *   核对 `atomic_components.md` 中定义的组件清单是否完整覆盖源文档 §3–§4 中出现的所有 UI 组件（登录卡片、Pin Input、进度条、Store Agent 卡片、告警规则卡片、成员表格、密钥面板、凭证更新 Modal、校验清单等）。
    *   验证 `interaction_states.md` 中的组件状态矩阵是否覆盖源文档 §5.1–§5.6 定义的所有交互模式和状态：
        - Optimistic UI vs 显式提交态的场景划分（§5.2）
        - 凭证失效锁定（§5.3）、实例降级（§5.4）、实例离线（§5.5）、前端网络断开（§5.6）
    *   检查各 Banner 的 z-index 优先级、颜色、触发条件、恢复方式是否与源文档完全一致。
    *   验证动画/过渡规范是否仅限于源文档 §1.1 描述的"克制的微交互"范围，无自行扩展。

3.  **布局与响应式一致性**
    *   核对 `layout_grid.md` 中定义的三栏布局（Header 56px + Sidebar 240px/60px + Content 流式）是否与源文档 §4.1 描述一致。
    *   验证断点定义（Mobile ≤768px / Tablet 769–1024px / Desktop ≥1025px）是否与源文档 §1.4 适配策略对齐（V0 桌面端优先，移动端基本可用）。
    *   检查 Dark Mode 策略：所有 UI 资产是否明确标注"V0 仅 Light Mode"（源文档 §1.4）。

4.  **Stitch Prompt 与 PRD 用户旅程对齐**
    *   逐条核对 `stitch_prompts.md` 中的 Flow prompts（F.1–F.8）是否完整覆盖源文档和 PRD 中定义的所有页面和用户旅程：
        - 登录与 Onboarding（PRD §2.2 + 源文档 §3.1）
        - Setup Wizard 5 步引导（PRD §3.5 + 源文档 §3.2）
        - 全局大盘与 Store Agent 列表（源文档 §4.2）
        - 单店 Dashboard（源文档 §4.3）
        - 告警规则配置（源文档 §4.4）
        - 执行日志（源文档 §4.5）
        - 实例设置三模块：Telegram 群管理（§4.6）、团队成员（§4.7）、Agent 密钥（§4.8）
        - Landing Page（Landing Page 规范全文）
    *   验证每个 Flow prompt 的 SCREENS 列表、STATES TO COVER、CONSTRAINTS 是否准确反映源文档中对应章节的要求，无遗漏或臆造。
    *   检查 Prototype prompts（P.1、P.2）的屏幕连接顺序是否与源文档 §2 信息架构的 mermaid 流程图一致。

5.  **错误文案矩阵对齐**
    *   核对 `stitch_prompts.md` 和 `interaction_states.md` 中引用的错误提示文案是否与源文档 §6.1（配置阶段）和 §6.2（运行阶段）错误文案矩阵中的文案逐字一致。
    *   验证 §6.3（IM 侧固定消息）是否在 UI 资产中明确标注为"非 Web UI 交付范围"。

6.  **跨资产一致性**
    *   检查 `design_system.md` 中引用的状态指标（Running/Idle/Online/Degraded/Offline）与 `.context/domain/ubiquitous_language.md` 和 `.context/architecture/system_design.md` 中的定义是否统一。
    *   验证凭证安全约束（不展示 Secret、不回显历史凭证、Client ID 首尾摘要）是否与 `.context/architecture/security_policy.md` 和 PRD §4.1 凭证安全约束一致。
    *   核对告警状态（Active/Silenced/Disabled）的定义、触发条件、颜色与 `.context/domain/business_rules.md` 和源文档 §4.4 是否一致。
    *   验证 Setup Wizard 步骤数（5 步）与 PRD §3.5（4 步描述 + 实际 5 步实施）之间的差异是否已在资产中明确说明并与源文档对齐。

7.  **Manifest 与索引同步**
    *   验证 `.context/context-manifest.json` 的 `generated_files.ui` 数组是否与 `.context/ui/` 目录下的实际文件列表完全同步（不多不少）。
    *   验证 `.context/openspec/integration.md` 中 `CONTEXT_ASSET_INDEX` 区块的 UI 文件数量和列表是否与 Manifest 和实际目录一致。
    *   验证 `.context/ui/README.md` 中的文件索引表是否与上述两处同步。

**执行步骤**
1. **资产清点**：分别列举 `.context/ui/`（不含 `source/`）与 `.context/ui/source/` 下的所有文件，建立生成资产与源文档的映射关系。
2. **文件级对比**：对每份生成资产，逐章节/逐 Token/逐组件进行深度对比，识别差异类型：
   - 信息缺失（源文档有，资产中无）
   - 臆造内容（源文档未提供具体值，资产中凭空填写）
   - 数据冲突（两处定义同一 Token/组件/状态但值或描述矛盾）
   - 覆盖遗漏（源文档中的页面/组件/状态未在 Stitch Prompt 或组件清单中覆盖）
   - 跨资产矛盾（UI 资产 vs 架构/领域资产的描述不一致）
3. **三角验证**：将 UI 资产描述与源 UI 规范 + PRD 用户旅程 + 架构安全约束进行三方交叉验证。
4. **意图判断**：对每处差异，参照文件头部 Metadata 和生成规则判断属于：
   - `[INTENTIONAL]` 有明确依据的有意推导（如 hover 色从 Primary 加深一级）
   - `[DEVIATION]` 缺乏依据的无意偏离
   - `[FABRICATED]` 源文档未提供但资产中臆造了具体值
   - `[AMBIGUOUS]` 无法确定，需人工确认
5. **生成报告**：按以下结构输出审查结果。

**输出报告结构**
1. **总体一致性摘要**：整体符合程度（高/中/低）及核心发现。
2. **文件配对状态表**：所有生成资产文件的一致性状态（✅ 完全一致 / ⚠️ 存在差异 / ❌ 严重偏离）。
3. **差异清单**（每条包含）：
   - 涉及文件路径 → 章节/Token/组件名 → 差异类型 → 具体描述 → 严重程度（P0 阻塞 / P1 重要 / P2 建议）→ 意图判断标签（`[INTENTIONAL]` / `[DEVIATION]` / `[FABRICATED]` / `[AMBIGUOUS]`）
4. **Token 对照矩阵**：将 `design_system.md` 和 `stitch_design_system.md` 中的每个颜色/字体/间距 Token 与源文档 §1.2–§1.3 的定义逐一对照，标注一致/偏差/臆造。
5. **Flow 覆盖矩阵**：将 `stitch_prompts.md` 中的 8 个 Flow 与源文档的页面章节（§3.1–§4.8）和 PRD 用户旅程（§2.2）逐一对照，标注覆盖/部分覆盖/未覆盖。
6. **Manifest 同步状态**：`context-manifest.json` ↔ `integration.md` ↔ `README.md` ↔ 实际目录的四方同步验证结果。
7. **可操作修复建议**：针对每条 `[DEVIATION]`、`[FABRICATED]` 和 `[AMBIGUOUS]`，给出修复方案（应修改哪一方、如何修改、优先级）。

**质量要求**
- 所有结论必须与文件实际内容建立明确对应关系，禁止泛泛而谈
- 严重程度分级须有依据（P0 = 影响设计系统正确性或安全约束；P1 = 影响 Stitch 生成一致性或页面覆盖完整性；P2 = 影响文档可读性或索引同步）
- 输出使用中文，文件引用使用完整相对路径
- 对任何臆造内容（源文档未给出具体值但资产中填写了具体值的情况）必须明确标注并评估风险
- Token 对照矩阵必须逐条列出，不可跳过或合并
```

### 检查init
```
请根据 .context 资产目录，对 .context/AGENTS.md、.context/criterion.md 和 .context/README.md 三份核心文档执行以下交叉审计：

1. **文件系统一致性**：将每份文档中描述的目录结构树与实际文件系统进行逐项比对，标记幽灵引用（文档中列出但实际不存在的文件/目录）和遗漏引用（实际存在但文档未提及的关键文件/目录）。

2. **三文档交叉一致性**：对比三份文档中对同一主题（目录结构、核心文件表格、更新规则、SSoT 路径等）的描述，标记任何矛盾、冲突或表述不一致之处。

3. **职责边界清晰度**：评估三份文档是否各有明确分工（README = 人类开发者目录总览；AGENTS = AI 执行入口；criterion = 工程约束 SSoT），标记内容重叠或越界。

4. **引用有效性**：检查文档中引用的所有外部路径（SSoT/、openspec/、design/ 工具路径等）是否实际存在，标注缺失文件的当前状态（待生成/已废弃/路径错误）。

5. **元数据时效性**：检查 Metadata 块中的时间戳是否能反映文档的实际最后修改状态，检查 context-manifest.json 与文档内容是否同步。

输出格式：按优先级（P0/P1/P2）分类，每条包含：涉及文件 → 行号 → 问题描述 → 修复建议。最后附一份文件系统实际状态快照作为修正基准。
```

### 检查 domain 资产和 prd 文档的一致性
```
/context-check review 请执行以下领域资产与产品需求文档的一致性审查任务：

**核心目标**
系统性地检查 `.context/domain` 目录下的领域模型资产与 `.context/domain/source/` 目录下的产品需求文档之间的逻辑一致性、术语统一性和数据对齐性。

**审查范围与重点**
1.  **实体与概念一致性**
    *   对比领域模型中定义的核心实体、值对象、聚合根与PRD中描述的业务概念、功能模块、数据对象是否一一对应。
    *   检查属性定义：名称、数据类型、约束条件、业务含义是否完全匹配。
    *   验证关系映射：领域模型中的关联、聚合、继承关系是否在PRD的功能流程、数据流转和界面交互中得到准确体现。

2.  **业务流程与规则对齐**
    *   核对领域服务、领域事件、业务规则与PRD中描述的核心业务流程、用户操作路径、系统处理逻辑是否吻合。
    *   识别PRD中隐含的业务规则是否已在领域模型中显式定义和固化。
    *   验证关键业务流程的输入、输出、状态转换在两者中是否一致。

3.  **术语表统一性**
    *   对照领域通用语言（Ubiquitous Language）术语表，检查PRD全文使用的业务术语、功能名称、状态标签是否严格统一，无歧义、无冲突。

4.  **边界与范围确认**
    *   确认PRD定义的产品功能范围是否完全落在领域模型所界定的限界上下文（Bounded Context）之内。
    *   检查是否存在PRD中描述的需求超出了当前领域模型的设计能力，或领域模型中存在未被PRD引用的冗余设计。

**执行步骤**
1.  **资产加载**：读取并解析 `.context/domain` 目录下的所有相关领域模型文件（如实体定义、关系图、规则说明等）。
2.  **文档解析**：深度解析 `.context/domain/source/*.md`，提取所有功能需求、非功能需求、用户故事、业务规则及数据定义。
3.  **逐项对比**：按照上述“审查范围与重点”中的四个维度，进行系统性交叉比对。
4.  **生成报告**：输出一份结构化的一致性审查报告，包含：
    *   **一致性摘要**：总体一致程度评估。
    *   **差异清单**：具体列出每一处发现的不一致、矛盾或缺失，需精确到文件、章节或具体定义。
    *   **风险评估**：对不一致项可能引发的开发风险、逻辑漏洞或业务误解进行评估。
    *   **修复建议**：针对每条差异，提出明确的修正建议（应修改哪一方，如何修改）。

**输出要求**
请以清晰、可操作的报告形式呈现审查结果，便于团队直接据此进行文档修订或模型调整。
```

### 架构检查
```
/context-check review 请对 `.context/architecture` 目录下的架构资产与 `.context/architecture/source` 目录下的原始架构源材料执行 **全量一致性审查**。

**核心目标**
逐文件比对两个目录中的对应内容，识别所有形式的差异，并明确区分"有意为之的迭代更新"与"无意偏离源材料"，生成一份可操作的一致性审查报告。

**执行步骤**
1. **资产清点**：分别列举 `.context/architecture/` 与 `.context/architecture/source/` 下的所有文件，建立配对关系；标出仅存在于单侧的文件。
2. **文件级对比**：对每一对配对文件，逐章节/逐字段进行深度对比，识别以下类型的差异：
   - 信息缺失（源材料有，资产中无）
   - 数据过时（资产中的版本、数字、状态与源材料不同步）
   - 定义冲突（两处定义同一概念但表述相矛盾）
   - 结构性差异（章节顺序、层级、命名规则不一致）
   - 未授权修改（资产中出现源材料中不存在且无明显理由的新增内容）
3. **意图判断**：对每处差异，参照提案记录或文件头部 metadata（如版本号、更新日志），判断该差异属于：
   - `[INTENTIONAL]` 有明确依据的有意更新
   - `[DEVIATION]` 缺乏依据的无意偏离
   - `[AMBIGUOUS]` 无法确定，需人工确认
4. **生成报告**：按以下结构输出审查结果。

**输出报告结构**
1. **总体一致性摘要**：整体符合程度（如百分比或等级：高/中/低）及核心发现。
2. **文件配对状态表**：列出所有文件的配对结果，标注其一致性状态（✅ 完全一致 / ⚠️ 存在差异 / ❌ 仅单侧存在）。
3. **差异清单**（每条包含）：
   - 涉及文件路径 → 章节/行号 → 差异类型 → 具体描述 → 严重程度（P0 阻塞 / P1 重要 / P2 建议）→ 意图判断标签（`[INTENTIONAL]` / `[DEVIATION]` / `[AMBIGUOUS]`）
4. **孤立文件列表**：列出仅存在于 `.context/architecture/` 或仅存在于 `.context/architecture/source/` 的文件，说明可能原因。
5. **可操作修复建议**：针对每条 `[DEVIATION]` 和 `[AMBIGUOUS]` 差异，给出明确的修复方案（应修改哪一方、如何修改、优先级）。

**质量要求**
- 所有结论必须与文件实际内容建立明确对应关系，禁止泛泛而谈
- 严重程度分级须有依据（P0 = 影响架构正确性；P1 = 影响实现一致性；P2 = 影响文档可读性）
- 输出使用中文，文件引用使用完整相对路径

```

### 项目整体结构说明检查
```
/context-check review 请对 `openspec/config.yaml` 执行 **全量结构与内容一致性审查**，逐章节验证其与 `.context/` 资产目录中权威文档的对齐程度。

**核心目标**
`openspec/config.yaml` 是 OpenSpec 规划工作流的注入上下文（OPSX context），由 `/context-openspec project` 从 `.context/` 全量资产综合而成。本次审查的目标是：验证 config.yaml 中每一项描述是否准确反映了权威源文档的内容，识别遗漏、过时、矛盾或精度不足之处，确保第三方 AI 消费此文件时获取的上下文完整且正确。

**审查范围**
- 被审文件：`openspec/config.yaml`
- 格式规范来源：`design/context-dev/openspec/project/AGENTS.md` §Phase 3 格式要求
- 权威对照源（按 config.yaml 章节映射）：

| config.yaml 章节 | 权威来源文件 |
|------------------|-------------|
| 顶层结构（`schema`/`version`/`rules` 结构） | `design/context-dev/openspec/project/AGENTS.md` §格式要求 |
| `context.purpose` | `.context/criterion.md` §1–§2 + `.context/architecture/system_design.md` §系统目的 |
| `context.tech_stack` | `.context/criterion.md` §3 + `.context/architecture/tech_stack.md` |
| Code Style（是否存在独立章节） | `.context/criterion.md` §3 + `.context/architecture/tech_stack.md` §AI 引用指南 |
| `context.architecture_patterns` | `.context/architecture/system_design.md` + `.context/architecture/cross_cutting_concepts.md` + `.context/architecture/runtime_view.md` |
| `context.domain_concepts` | `.context/domain/business_rules.md` + `.context/domain/user_journeys.md` + `.context/domain/domain_model.md` |
| `context.key_constraints` | `.context/criterion.md` §3–§5 全部 MUST/MUST NOT + `.context/architecture/security_policy.md` |
| `context.data_models` | `.context/db/schema_design.md` + `.context/db/migrations_and_ssot.md` + `.context/domain/domain_model.md` |
| `context.api_contracts` | `.context/architecture/api_strategy.md` + `.context/criterion.md` §4（SSoT/api/main.tsp） |
| `context.testing` | `.context/domain/testing_strategy.md` |
| Git Workflow（是否存在独立章节） | `.context/criterion.md` §6 + `.context/db/migrations_and_ssot.md` §PR Checklist |
| External Dependencies（是否存在独立章节） | `.context/architecture/system_design.md` §外部依赖 + `.context/architecture/deployment_view.md` + `.context/domain/risks_and_debt.md` §依赖管理 |
| UI Guidelines | 本项目无 `.context/ui/` 目录，应正确缺席 |
| `rules`（结构与子键） | `design/context-dev/openspec/project/AGENTS.md` §rules 建议 |
| `rules.proposal`（若存在） | `.context/criterion.md` §2/§6 + `.context/domain/risks_and_debt.md` RISK-B005 |
| `rules.specs`（若存在） | `.context/domain/testing_strategy.md` + `.context/domain/business_rules.md` BR-020~BR-023 |
| `rules.design`（若存在） | `.context/criterion.md` §3/§5 + `.context/architecture/security_policy.md` |
| `rules.tasks`（若存在） | `.context/criterion.md` §6 + `.context/db/migrations_and_ssot.md` §Checklist |
| `rules`（平铺列表，若无子键） | 以上全部来源，验证覆盖度 |

**审查维度与重点**

---

**维度一：结构完整性**

1. **模板字段覆盖验证**
   - 对照 `design/context-dev/openspec/project/AGENTS.md` §填充要求表格，逐条验证 config.yaml 是否包含全部必须（✅）和条件必须（⚠️）章节：Purpose、Tech Stack、Code Style、Architecture Patterns、Testing Strategy、Git Workflow、Domain Context、Important Constraints、External Dependencies、Database Design
   - 检查顶层是否含 `schema: spec-driven` 键（AGENTS.md §Phase 3 必须字段，本项目当前实际使用 `version: "1.0"`，需标注偏差）
   - 检查 `context` 是否为多行字符串（`context: |`）或嵌套对象，与模板要求对照
   - 检查 `rules` 是否为子键结构（proposal/specs/design/tasks）或平铺列表，与 AGENTS.md §rules 建议对照；若为平铺列表，至少需能被映射到 `tasks` 和 `specs` 语义
   - 标注缺失的 AGENTS.md 必须章节：Code Style、Git Workflow、External Dependencies

2. **YAML 格式正确性**
   - 验证 YAML 语法合法性（缩进、多行字符串 `|`、列表 `-` 格式、嵌套层级）
---

**维度二：context 章节内容准确性**

3. **`context.purpose` 准确性**
   - 与 `criterion.md` §1–§2 及 `architecture/system_design.md` §系统目的对照，验证：
     - 项目名称（IM Agent Bridge）是否正确
     - 三层架构描述（Channel → Bridge → Core）顺序和层命名是否正确
     - 各层职责（Matterbridge 桥接、Gateway 唯一入口、RuntimeAdapter 策略模式、NanoBot）是否准确
     - MVP 边界（Telegram、文本消息、单 Runtime）是否明确
     - 系统目标（Runtime 可替换、渠道可扩展）是否体现

4. **`context.tech_stack` 准确性**
   - 逐项与 `criterion.md` §3 和 `tech_stack.md` 对照：
     - Gateway：Rust、tokio、actix-web 或 axum、sqlx 或 diesel、serde/serde_json、uuid crate 约束等级（MUST/SHOULD）是否与 `tech_stack.md` 一致
     - NanoBot：Python 3.10+、来源（HKUDS/nanobot）、配置文件（MEMORY.md、config.json → tools.mcpServers）是否准确
     - PostgreSQL：版本（15+）、Goose、SSoT 路径（SSoT/schema/migrations/）是否正确
     - 基础设施：Docker + Docker Compose（MUST）、Nginx（可选）是否准确
     - 集成组件：Matterbridge（Go）、Shopify MCP（每店铺独立容器）、Telegram Bot API 是否齐全
     - forbidden 列表（Redis、硬编码凭证、NanoBot 作主入口等）是否与 `criterion.md` MUST NOT 条目吻合
     - 标注缺失的 Code Style 独立章节（AGENTS.md §填充要求 ✅ 必须）

5. **`context.architecture_patterns` 准确性**
   - 与 `system_design.md`、`cross_cutting_concepts.md`、`runtime_view.md` 对照，验证：
     - 三层边界禁止跨越规则（BR-020, BR-021）是否准确
     - RuntimeAdapter 路由字段（`bots.runtime_type`）是否正确
     - 熔断器：触发条件（PostgreSQL 不可用）、行为（503、禁止继续处理上下文）是否与 BR-041 一致
     - 限流：算法（Token Bucket）、维度（chat_id）、阈值（5 msg/sec）、超限行为（429、不调用 Runtime、不写 message_events）是否与 BR-055 精确匹配
     - 幂等：入站键 4 个字段及 COALESCE 语义、reply_id 来源描述是否准确
     - 回写重试：次数（3 次）、退避序列（1s/2s/4s）、409 语义（视为成功）是否与 BR-062 一致
     - 可观测性：10 个 Counter 名称是否与 `cross_cutting_concepts.md` §指标监控完全匹配；trace_id 覆盖范围是否准确
     - 标注缺失的 Git Workflow 独立章节（AGENTS.md §填充要求 ✅ 必须）

6. **`context.domain_concepts` 准确性**
   - 与 `business_rules.md`、`user_journeys.md`、`domain_model.md` 对照：
     - StandardMessage 9 个必须字段是否与 domain_model.md §1 完全一致；`bot_id` 来源（查 channel_bindings，不可外部传入）是否准确
     - session_id 生成规则（私聊/群聊格式字符串）是否与 BR-010/BR-011 精确匹配
     - 群聊共享上下文（BR-013）和禁止引入 thread_id/group_id（BR-015）是否覆盖
     - bot_id 解析：查询键（3 个字段）、NULL 降级（COALESCE/退化匹配）、未找到处理是否准确
     - 6 个错误码是否完整，语义描述是否与 `cross_cutting_concepts.md` §错误处理一致
     - 用户可见提示文案是否与 `cross_cutting_concepts.md` / `edge_cases.md` §系统响应规范原文一致

7. **`context.key_constraints` 完整性**
   - 逐条提取 `criterion.md` §3–§5 全部 MUST/MUST NOT，与 config.yaml `key_constraints` 及 `rules` 进行比对：
     - 消息长度三档（入站 4096 拒绝/出站 4096 截断/DB 512 截断）是否各自对应 BR-002/BR-003/BR-070
     - 超时预算四值（Gateway→Runtime 15s / MCP 10s / P95 5s / NanoBot 内部 120s）是否与 `runtime_view.md` §超时预算一致
     - 安全约束（Bearer Token 401 无降级、凭证环境变量、日志脱敏、裸 HTTP TD-001 标注）是否均覆盖
     - 数据治理三个保留期（message_events 30d / runtime_logs 14d / sessions 30d）是否与 `schema_design.md` 数据保留规则一致
     - runtime_logs payload 写入条件（仅 error）及 PII 脱敏字段（user_id、input_text）是否准确
     - MVP 范围锁定（单渠道/文本/单 Runtime）是否覆盖
     - 标注 `criterion.md` 中任何未覆盖的 MUST/MUST NOT 条目

8. **`context.testing` / `context.data_models` / `context.api_contracts` 准确性**
   - **testing**：与 `testing_strategy.md` 对照，核查单元/集成/E2E 覆盖率目标（80%/70%/100%）、工具链（cargo test/testcontainers）、P95 ≤ 5s 等性能目标、DoD 8 条（含成功率 ≥95%/≥95%/≥90%）是否精确
   - **data_models**：与 `schema_design.md` + `migrations_and_ssot.md` 对照，核查 5 张表名、UUID PKs、入站幂等键 4 字段 + COALESCE、sessions.runtime_session_key NanoBot 特化说明、是否遗漏 `db/performance_tuning.md`/`db/observability.md`/`db/security_hardening.md` 关键规则
   - **api_contracts**：与 `api_strategy.md` 对照，核查两个端点路径/方法、/gateway/inbound 错误码列表（401/404/429/503）、NanoBotAdapter timeout（15s）、RUNTIME_SESSION_NOT_FOUND 重建逻辑是否准确
   - 标注缺失的 External Dependencies 独立章节（AGENTS.md §填充要求 ✅ 必须）

---

**维度三：rules 规则完备性与可操作性**

9. **rules 结构合规性**
   - config.yaml 的 `rules` 是平铺列表还是子键（proposal/specs/design/tasks）？与 AGENTS.md §格式要求对照，标注结构偏差（P0：影响工具消费格式）
   - 若为平铺列表：评估是否造成规则分类信息丢失，是否能被语义映射到 tasks 和 specs

10. **规则覆盖验证**
    - **架构边界**：BR-020/BR-021/BR-022/BR-023（三层边界、Gateway 唯一入口、Runtime 可替换、Runtime 不作主入口）是否均覆盖
    - **消息处理**：BR-001/BR-002/BR-003/BR-004/BR-005（文本限定、入站拒绝/出站截断/DB截断长度、bot_id 来源、输出纯文本）是否均覆盖
    - **Session**：BR-010~BR-013/BR-015（私聊/群聊 session 规则、隔离、群聊共享、无 thread_id）是否均覆盖
    - **数据库**：BR-041/BR-042（PostgreSQL 熔断、幂等去重、数据保留期、Goose 迁移约束）是否覆盖
    - **安全**：BR-030/BR-031/BR-032/BR-033（凭证环境变量、Bearer Token 401、bot_id 隔离、MCP 凭证管理）是否均覆盖
    - **性能**：BR-050/BR-051/BR-052/BR-055（P95 5s、Gateway timeout 15s、MCP timeout 10s、限流 5/s）是否均覆盖
    - **变更流程**：SSoT-first（DB 先写 Goose、API 先改 TypeSpec）、OpenSpec 提案要求是否覆盖

11. **规则可执行性评估**
    - 每条规则是否有具体数值/判断标准（vs 过于抽象）
    - 是否存在规则间矛盾或重复
    - 是否遗漏 `criterion.md` 中任何 MUST/MUST NOT 约束

---

**维度四：精度与时效性**

12. **数值精度验证**
    - 逐一核对 config.yaml 中出现的所有数值与源文档是否精确一致：
      - 超时四值（15s / 10s / 5s P95 / 120s NanoBot 内部）
      - 限流（5 msg/sec/chat_id）
      - 长度限制（入站/出站 4096、DB 512）
      - 重试（3 次、退避 1s/2s/4s）
      - 数据保留期（30d / 14d / 30d）
      - 测试覆盖率（≥80% / ≥70%）
      - 成功率（接入 ≥95%、回写 ≥95%、MCP ≥90%）
      - 表数量（5 张）、Counter 数量（10 个）
    - 标注任何与源文档不符的具体数值

13. **术语一致性**
    - 检查 config.yaml 中使用的关键术语是否与权威来源一致：
      - `bridge_message_id` 字段名是否与 `schema_design.md` / `domain_model.md` 一致
      - `runtime_session_key` 语义描述是否与 `schema_design.md` §sessions 一致
      - 表名 `sessions`（非 `session_mappings`）是否正确
      - 6 个错误码名称大小写格式是否与 `cross_cutting_concepts.md` 完全一致

---

**维度五：遗漏检测**

14. **关键信息遗漏**
    - 检查以下内容是否在 config.yaml 中有适当体现（即使是摘要形式）：
      - **Code Style 章节**（AGENTS.md ✅ 必须）：Rust 异步模式、错误处理惯例
      - **Git Workflow 章节**（AGENTS.md ✅ 必须）：SSoT-first 三层（DB→Goose、API→TypeSpec、代码最后）、OpenSpec 提案流程
      - **External Dependencies 章节**（AGENTS.md ✅ 必须）：各外部依赖及已知风险（RISK-001~RISK-007）
      - **消息状态机**：`message_events.status` 枚举（pending/processing/done/error）和 `reply_status`（success/reply_failed）
      - **DB 额外文件**：`db/performance_tuning.md`、`db/observability.md`、`db/security_hardening.md` 中的关键规则
      - **RISK-007 适配层差距**：TAD Push 模型 vs Matterbridge Pull API 的已知风险是否提及
    - 判断：该遗漏是合理精简还是关键缺失（影响 OPSX 生成正确性则 P0）

---

**维度六：与其他 OpenSpec 文件的协调性**

15. **与 integration.md 的一致性**
    - 验证 `openspec/config.yaml` 中引用的资产范围是否与 `.context/openspec/integration.md` 的 `CONTEXT_ASSET_INDEX` 所列文件一致；特别检查 `db/performance_tuning.md`、`db/observability.md`、`db/security_hardening.md`、`architecture/runtime_view.md`、`domain/edge_cases.md` 是否在 config.yaml 中有相应规则体现

16. **与 criterion.md SSoT 路径表的一致性**
    - 验证 config.yaml 中引用的 SSoT 路径（`SSoT/schema/migrations/`、`SSoT/api/main.tsp`）是否与 `criterion.md` §4 SSoT 路径表完全一致

---

**执行步骤**

1. **结构扫描**：解析 config.yaml 的 YAML 结构，提取所有顶级键和 context 子章节标题，与模板要求对照。
2. **逐章节审查**：按维度二的每个章节，打开对应权威源文件，逐条/逐值对比 config.yaml 中的描述。
3. **规则审查**：按维度三，检查每条 rule 的覆盖度和可执行性。
4. **数值交叉验证**：按维度四，提取所有数值进行精确比对。
5. **遗漏扫描**：按维度五，系统扫描 `.context/` 关键文件中的核心概念在 config.yaml 中的体现。
6. **意图判断**：对每处差异标注：
   - `[ACCURATE]` 准确反映源文档
   - `[IMPRECISE]` 表述不够精确（未错但可改进）
   - `[OUTDATED]` 与源文档当前版本不同步
   - `[CONFLICT]` 与源文档定义矛盾
   - `[MISSING]` 关键信息未覆盖
   - `[REDUNDANT]` 冗余或重复内容
7. **生成报告**。

**输出报告结构**

1. **全局一致性评分**

   | 维度 | 评级 | 关键发现 |
   |------|------|---------|
   | 结构完整性 | {{高/中/低}} | {{...}} |
   | context 准确性 | {{高/中/低}} | {{...}} |
   | rules 完备性 | {{高/中/低}} | {{...}} |
   | 数值精度 | {{高/中/低}} | {{...}} |
   | 遗漏程度 | {{高/中/低}} | {{...}} |
   | OpenSpec 协调性 | {{高/中/低}} | {{...}} |

2. **章节覆盖对照表**

   | config.yaml 章节 | 权威来源 | 覆盖状态 | 准确度 | 备注 |
   |------------------|---------|:--------:|:------:|------|
   | context.Purpose | criterion.md §1 | ✅/⚠️/❌ | {{...}} | {{...}} |
   | ... | ... | ... | ... | ... |

3. **MUST/MUST NOT 合规矩阵**
   逐条列出 `criterion.md` 中的每条 MUST/MUST NOT 规则：

   | 规则 (criterion.md 位置) | config.yaml 覆盖情况 | 表述一致？ | 标签 |
   |-------------------------|---------------------|:---------:|------|
   | {{规则描述}} (§3.1 MUST #1) | {{对应条目或"未覆盖"}} | ✅/❌ | `[ACCURATE]`/`[MISSING]`/... |

4. **差异清单**（每条包含）：
   - config.yaml 位置（章节 + 具体行/段落）
   - 权威来源（文件路径 + 章节/字段）
   - 差异类型标签（`[IMPRECISE]` / `[OUTDATED]` / `[CONFLICT]` / `[MISSING]` / `[REDUNDANT]`）
   - 具体描述（差异内容对比）
   - 严重程度（P0 = 影响 OPSX 生成的正确性或安全约束 / P1 = 影响规划上下文精度 / P2 = 影响可读性）

5. **rules 可执行性评估**

   | 规则类别 | 规则条数 | 可直接验证 | 过于抽象 | 有矛盾 | 有遗漏 |
   |---------|:-------:|:---------:|:------:|:-----:|:-----:|
   | proposal（若存在） | {{N}} | {{N}} | {{N}} | {{N}} | {{N}} |
   | specs（若存在） | ... | ... | ... | ... | ... |
   | design（若存在） | ... | ... | ... | ... | ... |
   | tasks（若存在） | ... | ... | ... | ... | ... |
   | 平铺列表（若无子键） | {{总N}} | {{N}} | {{N}} | {{N}} | {{N}} |

6. **遗漏项清单**
   需在 config.yaml 中补充的关键信息（按影响程度排序）。

7. **可操作修复建议**
   按 P0 → P1 → P2 优先级排序，每条包含：
   - 修改位置（config.yaml 章节）
   - 修改内容（应添加/修改/删除的具体文案）
   - 原因（引用权威来源文件路径 + 章节）

**质量要求**
- 所有结论必须引用 config.yaml 的具体章节和权威来源文件的具体路径+章节，禁止泛泛而谈
- MUST/MUST NOT 合规矩阵必须逐条扫描 `criterion.md` §3–§4 的每一条规则，不可跳过或合并
- 数值对比必须精确到具体值（如"config.yaml 写 retry 次数为 2，business_rules.md BR-062 中为 3 次"）
- 对 config.yaml 中使用精简/摘要表述的地方，需判断是合理精简还是丢失关键信息
- 输出使用中文，文件引用使用相对路径（相对于项目根目录）
```

### 整体检查大纲计划

```
/context-check review 请对 `openspec/proposal-roadmap.md` 执行**全量提案路线图审查**，以验证路线图是否与 `.context/` 资产目录中的权威规范完整对齐。

**核心目标**
对路线图的结构合理性、语境合规性、提案原子性、依赖完整性和风险覆盖进行系统性评审，输出一份可操作的审查报告。

**审查范围**

| 文件 | 路径 | 角色 |
|------|------|------|
| 路线图（单文件） | `openspec/proposal-roadmap.md` | 阶段总览、依赖图、16 个提案详情 |

> 本项目为绿地项目（无子 Phase 文件），所有提案集中在单一路线图文件中。

**权威对照源**

| 权威文件 | 对照维度 |
|---------|----------|
| `.context/criterion.md` | MUST/MUST NOT 规则、技术约束、SSoT 规范 |
| `.context/domain/business_rules.md` | 业务规则覆盖（BR-001～BR-072 逐条核对） |
| `.context/domain/user_journeys.md` | 用户旅程覆盖（主旅程 + 异常流 → 提案映射） |
| `.context/domain/domain_model.md` | 核心实体（StandardMessage / ChannelBinding / Session / MessageEvent / RuntimeLog）生命周期与提案对齐 |
| `.context/domain/testing_strategy.md` | BDD Gherkin 场景覆盖（模块 1–8） |
| `.context/domain/risks_and_debt.md` | 业务风险（RISK-B00X）缓解映射 |
| `.context/architecture/risks_and_debt.md` | 技术风险（RISK-T00X）缓解映射 |
| `.context/architecture/system_design.md` | 三层架构组件职责、NFR/SLO 指标 |
| `.context/architecture/api_strategy.md` | 接口契约（inbound / bridge/reply / runtime/process） |
| `.context/architecture/runtime_view.md` | 5 个运行时场景、超时预算、状态机 |
| `.context/architecture/security_policy.md` | 认证边界、凭证管理、数据截断、PII 脱敏 |
| `.context/architecture/deployment_view.md` | Docker Compose 拓扑、网络分区、容器角色 |
| `.context/architecture/tech_stack.md` | MUST/SHOULD/MUST NOT 技术选型 |
| `.context/db/schema_design.md` | 5 张表 + 索引与提案对齐 |
| `.context/db/migrations_and_ssot.md` | SSoT-first 变更流程、Goose 约束 |
| `openspec/config.yaml` | 项目级约束传播 |

**审查维度与重点**

---

**维度一：语境合规性（Context Compliance）**

1. **Criterion MUST/MUST NOT 映射**
   - 逐条提取 `criterion.md` §2–§6 中的全部 MUST 和 MUST NOT 规则，验证每条规则在路线图中是否有至少一个提案负责覆盖
   - 特别检查：
     - Gateway 唯一入口（Bridge 不可直连 Runtime/DB）
     - Runtime 可替换性（Strategy Pattern per runtime_type，trait 抽象）
     - session_id 格式（`telegram:private:{chat_id}` / `telegram:group:{chat_id}`）
     - Bearer Token 环境变量注入、禁止硬编码
     - NanoBot 协议约束：session_id 必传、messages 严格 1 条、不传 stream
     - DB 不可用时短路熔断 503，禁止继续处理
     - MCP 配置禁止存入 PostgreSQL（MUST NOT）
     - SSoT-first：DB 变更先建 Goose 迁移，API 变更先改 main.tsp
   - 标注：已覆盖 / 部分覆盖 / 未覆盖

2. **业务规则覆盖矩阵**
   - 从 `business_rules.md` 提取所有 BR-001～BR-072 编号规则，逐条核对是否有对应提案负责实现
   - 重点检查：
     - BR-001～BR-005（消息接入：文本限定、4096 上限、StandardMessage 结构、回复格式）
     - BR-010～BR-015（session_id 生成、上下文隔离、退化策略）
     - BR-020～BR-023（架构边界：Gateway 唯一入口、Runtime 可替换、Runtime 不直接访问 DB）
     - BR-030～BR-033（Bearer Token、安全通信、配置隔离、MCP 凭证管理）
     - BR-040～BR-042（PostgreSQL 持久化范围、DB 不可用短路、数据清理）
     - BR-050～BR-053（端到端 P95、15s 超时、10s MCP 超时、故障隔离）
     - BR-060～BR-063（Runtime/MCP/回写失败处理、错误可见性）
     - BR-070～BR-072（消息最小化、PII 脱敏、512 字符截断）

3. **用户旅程覆盖矩阵**
   - 从 `user_journeys.md` 提取所有旅程步骤（主旅程 + 系统开发者旅程 + 异常流），验证每个步骤映射到至少一个提案
   - 特别核查：私聊 vs 群聊 session 路由、Bot 配置初始化（bots + channel_bindings 数据录入）

---

**维度二：整体编排合理性（Sequencing & Phasing）**

4. **Phase 划分合理性**
   - 评估 5 个 Phase（0–4）的分组逻辑是否符合「基础→核心→集成→部署→验证」的渐进原则
   - 验证 Phase 0（基础设施）是否为后续所有 Phase 提供了必要的脚手架
   - 检查 Phase 1 入口（feat-gateway-db-layer）是否在 DB 熔断就绪后才允许业务逻辑提案启动
   - 验证 Phase 4 E2E 提案（feat-e2e-integration-test）是否确实位于所有功能提案之后

5. **时间估算可行性**
   - 评估各提案预计时间是否与其范围边界（In/Out 项数）和关键任务数量匹配
   - 计算总估时（串行关键路径），验证是否在合理的 MVP 交付周期内（参考 criterion.md 约束）
   - 识别估时最长的提案（> 2 天），评估是否可进一步拆解

---

**维度三：依赖关系完整性（Dependency Integrity）**

6. **依赖图一致性**
   - 将路线图中的 Mermaid 依赖图与每个提案声明的「前置依赖」字段逐条交叉核对
   - 标注三类问题：图中有但提案未声明 / 提案声明了但图中缺失 / 两者矛盾
   - 重点核查提案索引表中的「前置依赖」列与 Mermaid 图的一致性

7. **隐含依赖识别**
   - 检查是否存在提案 A 的实现实际需要提案 B 的产出，但两者依赖关系未显式声明
   - 重点检查：feat-gateway-channel-session 是否依赖 feat-gateway-db-layer 的 PgPool、feat-runtime-reply-bridge 是否依赖 feat-infra-matterbridge-deploy 的 Bridge URL 配置

8. **孤立与循环检测**
   - 检查是否存在无上游依赖且无下游被依赖的孤岛提案
   - 检查是否存在循环依赖（提案 A 依赖提案 B，提案 B 依赖提案 A）

---

**维度四：提案原子性（Atomicity）**

9. **单一职责验证**
   - 对每个提案的范围边界（In / Out）审查，确保每个提案仅解决一个单一、明确的议题
   - 标记将多个不相关议题混杂的提案（如同时涉及 Schema 变更 + Rust 业务逻辑 + Matterbridge 配置）
   - 标记范围过大可拆解的提案（判断标准：预计时间 > 3 天 且 关键任务 > 4 项）

10. **范围重叠检测**
    - 检查是否有两个提案的 In 范围存在重叠（如 session_id 生成在两处都声称实现）
    - 检查是否有两个提案的验收标准存在矛盾（如对同一字段的写入行为声明不同）

---

**维度五：安全与合规覆盖（Security & Compliance）**

11. **安全约束闭环**
    与 `security_policy.md` 对照，验证以下安全链路是否有完整的提案覆盖：
    - **凭证管理链路**：GATEWAY_BEARER_TOKEN / BRIDGE_BEARER_TOKEN / TELEGRAM_BOT_TOKEN / LLM_API_KEY / Shopify 凭证 — 全部通过环境变量注入，无任何凭证入库或硬编码
    - **Bearer Token 校验链路**：Bridge→Gateway（必须校验）、Gateway→Runtime（MVP 无认证，已知技术债 TD-001）
    - **数据截断链路**：input_text / output_text 截断至 512 字符写入 DB（BR-071）
    - **runtime_logs PII 保护**：仅 error 时写入 payload，且脱敏后才写（BR-072）
    - **DB 熔断链路**：PostgreSQL 不可用 → 503 短路，不得继续处理（BR-041）
    - **MCP 凭证隔离**：Shopify MCP 凭证通过 .env 注入子进程，MUST NOT 存入 PostgreSQL

    标注任何缺失的提案覆盖环节

---

**维度六：风险覆盖（Risk Coverage）**

12. **技术风险缓解映射**
    - 提取 `architecture/risks_and_debt.md` 中全部 RISK-T00X 风险条目（Runtime SPOF、NanoBot 本地状态丢失、Shopify MCP 可用性、PostgreSQL 不可用、Matterbridge 稳定性、Bearer Token 泄露、TAD 与三方工具能力差距）
    - 验证每个高/极高风险是否有至少一个提案显式覆盖了缓解措施
    - 重点：NanoBot 协议偏差风险（tech_stack.md 标注）是否在 feat-runtime-nanobot-adapter 中得到充分处理

13. **业务/项目风险缓解映射**
    - 提取 `domain/risks_and_debt.md` 中全部 RISK-B00X 风险条目（Runtime 能力误判、边界混淆、MCP 工具暴露过度、Session 设计导致 MVP 蔓延、架构完整性被扩展破坏、群聊上下文歧义）
    - 验证每个中/高风险是否在路线图某个提案的风险缓解列表中被显式提及

14. **技术债标注**
    - 提取 `architecture/risks_and_debt.md` 中全部 TD-00X 技术债（无 Gateway-Runtime 认证、无独立管理后台、单 IM/Runtime 限制、无下游请求取消、手动数据保留清理、群聊 context 共享）
    - 验证路线图是否在相关提案的 Out 范围或备注中明确标注了这些技术债（而非静默忽略）

---

**执行步骤**

1. **文件加载**：读取 `openspec/proposal-roadmap.md` + 上述全部权威对照源。
2. **Criterion 逐条扫描**：提取 `criterion.md` §2–§6 的每条 MUST/MUST NOT，在 15 个提案中搜索覆盖证据。
3. **BR 规则映射**：提取 `business_rules.md` 的全部 BR 编号（BR-001～BR-072），映射到对应提案。
4. **用户旅程核查**：提取 `user_journeys.md` 全部旅程步骤，逐步验证提案覆盖。
5. **依赖图校验**：解析 Mermaid 图 + 各提案依赖声明，逐条交叉比对（不得抽检）。
6. **原子性审查**：对 15 个提案的范围、任务数、估时逐一进行单一职责判断。
7. **安全链路追踪**：沿 6 条安全链路逐条验证提案覆盖完整性。
8. **风险映射**：RISK-T00X / RISK-B00X / TD-00X → 提案 → 缓解措施闭环。
9. **生成报告**：按以下结构输出。

**输出报告结构**

1. **全局质量评分**

   | 维度 | 评级 | 关键发现 |
   |------|------|---------|
   | 语境合规性 | {{高/中/低}} | {{...}} |
   | 编排合理性 | {{高/中/低}} | {{...}} |
   | 依赖完整性 | {{高/中/低}} | {{...}} |
   | 提案原子性 | {{高/中/低}} | {{...}} |
   | 安全覆盖 | {{高/中/低}} | {{...}} |
   | 风险覆盖 | {{高/中/低}} | {{...}} |

2. **Criterion MUST/MUST NOT 合规矩阵**

   | 规则（criterion.md 位置） | 关联提案 change-id | 覆盖状态 | 备注 |
   |--------------------------|-------------------|:--------:|------|
   | {{规则描述}} (§N.x MUST/MUST NOT) | {{change-id 或「未覆盖」}} | ✅/⚠️/❌ | {{...}} |

3. **BR 规则覆盖矩阵**

   | BR 编号 | 规则摘要 | 关联提案 change-id | 覆盖状态 |
   |---------|---------|-------------------|:--------:|
   | BR-001 | 仅处理文本消息 | feat-gateway-inbound-gate | ✅/⚠️/❌ |

4. **用户旅程覆盖矩阵**

   | 旅程（user_journeys.md） | 步骤 | 关联提案 change-id | 覆盖状态 |
   |-------------------------|------|-------------------|:--------:|

5. **依赖一致性核对表**

   | 提案 change-id | 提案声明的前置依赖 | Mermaid 图中的上游 | 一致？ | 备注 |
   |----------------|------------------|------------------|:-----:|------|

6. **原子性审查清单**

   | 提案 change-id | In 范围项数 | 关键任务数 | 估时 | 原子性判定 | 拆解建议 |
   |----------------|:---------:|:--------:|:----:|:--------:|---------|
   | {{...}} | {{N}} | {{N}} | {{N}}d | ✅ 原子 / ⚠️ 可拆 / ❌ 需拆 | {{...}} |

7. **安全链路闭环检查**

   | 安全链路 | 涉及提案 change-id | 覆盖完整？ | 缺失环节 |
   |---------|------------------|:--------:|---------|
   | 凭证环境变量注入（6 类凭证） | {{...}} | ✅/❌ | {{...}} |
   | Bearer Token 校验（Bridge→Gateway） | {{...}} | ✅/❌ | {{...}} |
   | input_text/output_text 512 字符截断 | {{...}} | ✅/❌ | {{...}} |
   | runtime_logs PII 脱敏写入 | {{...}} | ✅/❌ | {{...}} |
   | PostgreSQL 不可用短路熔断 | {{...}} | ✅/❌ | {{...}} |
   | MCP 凭证隔离（禁止入库） | {{...}} | ✅/❌ | {{...}} |

8. **风险缓解映射表**

   | 风险 ID | 风险描述 | 等级 | 缓解提案 change-id | 缓解充分？ |
   |---------|---------|:----:|-------------------|:--------:|
   | RISK-T001 | Runtime 单点故障 | 高 | {{...}} | ✅/⚠️/❌ |
   | RISK-T004 | PostgreSQL 不可用 | 高 | feat-gateway-db-layer | ✅/⚠️/❌ |
   | RISK-T006 | TAD 与三方工具能力差距 | 高 | feat-runtime-nanobot-adapter | ✅/⚠️/❌ |
   | RISK-B001 | Runtime 能力误判 | 中 | {{...}} | ✅/⚠️/❌ |

9. **技术债标注核查**

   | TD 编号 | 债务描述 | 相关提案 change-id | 在 Out 范围中标注？ |
   |---------|---------|-------------------|:-----------------:|
   | TD-001 | 无 Gateway-Runtime 认证 | feat-runtime-nanobot-adapter | ✅/❌ |
   | TD-004 | 手动数据保留清理 | feat-gateway-message-pipeline | ✅/❌ |

10. **问题清单**
    每条包含：
    - 维度标签（`[COMPLIANCE]` / `[SEQUENCING]` / `[DEPENDENCY]` / `[ATOMICITY]` / `[SECURITY]` / `[RISK]`）
    - 涉及文件路径 + 提案 change-id
    - 问题描述（精确到字段值或规则编号）
    - 严重程度（P0 = 影响系统正确性或安全 / P1 = 影响交付一致性 / P2 = 影响文档质量）
    - 修复建议

11. **可操作修复建议**
    按 P0 → P1 → P2 优先级排序，每条包含：
    - 修改文件：`openspec/proposal-roadmap.md`
    - 修改位置（提案 change-id + 章节名）
    - 修改内容（应新增/修改/删除的具体内容）
    - 原因（引用权威来源文件相对路径 + 章节名）

**质量要求**
- 所有结论必须引用具体的提案 change-id 和权威来源文件相对路径 + 章节，禁止泛泛而谈
- Criterion MUST/MUST NOT 必须逐条扫描（criterion.md §2–§6），不可跳过或合并
- BR 规则必须逐编号核对（BR-001～BR-072），不可选择性忽略
- 依赖图校验必须逐条比对 Mermaid 图与提案声明，不可仅做抽检
- 数值对比必须精确（如"提案中写 retry 2 次，runtime_view.md §场景4 中为 3 次"）
- 输出使用中文，文件引用使用相对路径（相对于项目根目录）
```


### 其他
```
请对docs/openfleet/cbec/Shopify/v0文件夹下的所有文档进行全面审核，检查每个文档的内部逻辑是否自洽、信息是否一致、术语是否统一，并在检查完毕后生成一份报告，列出每个文档的名称、检查结果（通过/不通过）、存在的逻辑冲突或不一致之处，并给出改进建议。note:1.如果架构文档和产品需求文档有冲突，按照架构文档进行调整
```

## outline&plan&context

### 检查提案大纲，提案具体内容和提案资产的实际情况
```
/context-check review 对 change-id 为 `feat-legacy-command-surface-lockdown` 的提案进行三层联合评审，评审对象包括：
- 大纲（Outline）：`openspec/proposal-roadmap-Phase-0.md` 中该提案的大纲条目
- 提案内容（Proposal）：`openspec/changes/feat-legacy-command-surface-lockdown/`（包含 proposal.md / spec.md / tasks.md,可能有design.md）
- 业务资产（Context）：`.context/` 目录下的权威文档

---

## 阶段一：大纲与资产一致性评审（Outline ↔ .context）

验证大纲中描述的目标、范围、约束与验收口径是否与 `.context/` 权威资产对齐，识别大纲存在的遗漏、冲突或表述不准确之处。

**权威对照源**：
- `criterion.md`（工程约束 SSoT）
- `domain/business_rules.md` + `domain/domain_model.md` + `domain/user_journeys.md`（业务规则、领域模型与用户旅程）
- `domain/edge_cases.md`（边界场景，校验 In/Out 是否遗漏已知边缘情况）
- `architecture/risks_and_debt.md` + `domain/risks_and_debt.md`（风险对照）
- `architecture/security_policy.md`（安全约束）
- `legacy/legacy_system_analysis.md`（遗留系统约束，Phase-0 提案必须参考）

**评审项**：
1. **业务目标对齐** — 大纲"业务目标"中的每条目标是否能在 `business_rules.md` 或 `user_journeys.md` 中找到对应的业务需求依据？是否有与 .context 冲突的目标？
2. **关联资产覆盖度** — 大纲"关联 Context 资产"表格是否引用了该提案所涉及的全部 .context 文档？对照 `.context/` 实际目录，列出缺失引用和多余引用。
3. **In/Out 边界与 .context 对齐** — 大纲的 In 范围是否与 .context 所定义的功能边界和业务规则一致？Out 范围是否遗漏了 `edge_cases.md` 中已知的需要明确排除的边缘场景？
4. **验收标准与约束对齐** — 大纲的验收标准是否覆盖了 `criterion.md` 中相关的 MUST/MUST NOT 要求？是否有可操作的验证方式（日志/指标值/API 响应）？标注主观模糊或缺失量化指标的条目。
5. **风险覆盖完整度** — 对照 `risks_and_debt.md` 中的 RISK-xxx 列表，大纲风险表是否覆盖了所有与该提案相关的风险？标注遗漏的 RISK-xxx 及其严重程度。

**输出**：
- 大纲与 .context 一致性评级（高 / 中 / 低）及核心发现
- 业务目标对照表（| 大纲目标 | .context 依据 | 对齐状态 ✅/⚠️/❌ |）
- 缺失 / 多余的资产引用列表
- In/Out 边界问题清单（遗漏的边缘场景、与 .context 冲突的范围定义）
- 验收标准问题列表（触发 MUST/MUST NOT 遗漏或缺失量化指标的条目）
- 遗漏 RISK-xxx 列表（含风险等级）

---

## 阶段二：大纲与提案内容一致性校验（Outline ↔ Proposal）

逐项核对提案内容是否忠实还原并完整展开了大纲的意图，识别边界漂移、缺口与偏差。

**评审项**：
1. **目标映射** — 大纲"业务目标"中的每条目标是否在 proposal.md 中均有对应说明？标注未覆盖项。
2. **范围边界落地** — 大纲 In/Out 范围是否完整体现在 proposal.md 的范围章节中？是否有边界漂移（提案比大纲多做或少做了什么）？
3. **关键任务还原** — 大纲"关键任务"列表是否在 tasks.md 中被拆解为可执行的原子任务？检查是否有大纲任务未在 tasks.md 出现。
4. **验收标准传递** — 大纲的验收条目是否在 spec.md 或 tasks.md 的验收部分中被完整继承？是否有弱化或丢失。
5. **依赖关系对齐** — 大纲声明的前置依赖和被依赖提案是否在 proposal.md 的依赖章节中完整体现？

**输出**：
- 一致性总评（完全一致 / 部分一致 / 不一致）
- 目标覆盖对照表（| 大纲目标 | 提案对应章节 | 覆盖状态 |）
- 边界漂移清单（多做/少做的具体内容）
- 任务映射缺口列表
- 依赖声明差异列表

---

## 阶段三：提案内容合规评审（Proposal → .context）

判断提案内容是否满足 `.context/` 定义的系统业务要求与约束，并验证其在路线图中的定位合理性。

**权威对照源**（按提案涉及模块选择性加载，全量列举以供对照）：
| 评审维度 | 权威来源文件 |
|---------|------------|
| 业务规则 / 用户旅程 | `.context/domain/business_rules.md` + `.context/domain/user_journeys.md` + `.context/domain/domain_model.md` + `.context/domain/ubiquitous_language.md` |
| 边界与边缘场景 | `.context/domain/edge_cases.md` |
| 非功能要求（安全/合规/性能） | `.context/criterion.md` §3-§4 + `.context/architecture/security_policy.md` + `.context/db/security_hardening.md` |
| 可观测性 / 运维 / 部署 | `.context/architecture/cross_cutting_concepts.md` + `.context/architecture/runtime_view.md` + `.context/architecture/deployment_view.md` |
| 数据层（如涉及 DB 变更） | `.context/db/schema_design.md` + `.context/db/migrations_and_ssot.md` + `.context/db/performance_tuning.md` |
| 遗留系统兼容 / 迁移路径 | `.context/legacy/legacy_system_analysis.md` + `.context/architecture/migration_architecture.md` |
| 测试与验收 | `.context/domain/testing_strategy.md` + `.context/domain/data_strategy.md` |
| UI / 交互（如涉及前端） | `.context/ui/design_system.md` + `.context/ui/interaction_states.md` + `.context/ui/stitch_prompts.md` |
| API 契约 | `.context/architecture/api_strategy.md` |
| 路线图定位与依赖 | `openspec/proposal-roadmap.md` + `openspec/proposal-roadmap-Phase-<N>.md` |

**审查维度**：
1. MUST/MUST NOT 合规矩阵（逐条扫描 criterion.md §3-§4，不可跳过）
2. 业务规则核对（BR-xxx 逐条映射，标注满足/不满足/缺失证据）
3. 路线图定位（依赖完整性、冲突点、重叠建设风险）
4. 接口契约分析（上下游耦合、破坏性变更影响范围）
5. 技术与交付风险（触发条件 + 影响 + 缓解 + 责任归属）
6. 待澄清问题（P0 阻塞 / P1 重要 / P2 建议）
7. 可执行修改建议（M-n 编号 + 修改位置 + 验收口径 + 监控指标）

**输出报告**：
1. **综合结论**（PASS / PASS (Conditional) / MODIFY / FAIL）及各阶段小结
2. MUST/MUST NOT 合规矩阵（| 规则（criterion.md 位置）| 提案证据 | 状态 ✅/⚠️/❌ |）
3. 业务契合度表（| BR-xxx | 满足/不满足/缺失 | 提案引用位置 |）
4. 大纲↔提案一致性对照表（| 大纲章节 | 提案对应位置 | 一致性 | 差异说明 |）
5. 路线图关联分析（依赖图核对、冲突点、整合建议）
6. 风险清单（| 风险ID | 触发条件 | 影响 | 缓解方案 | 责任归属 |）
7. 待澄清问题列表（按 P0/P1/P2 排序）
8. 修改建议（M-n 编号 + 涉及层（大纲/提案/两者）+ 修改位置 + 具体内容 + 验收口径）

**质量要求**：
- 所有结论必须引用具体文件路径 + 章节/行号，禁止泛泛而谈
- MUST/MUST NOT 必须逐条扫描，BR-xxx 必须逐条映射，不可合并或跳过
- 修改建议须明确指出应改"大纲"还是"提案内容"或"两者均需同步"
- 输出使用中文，文件引用使用相对路径（相对项目根目录）
```

已经按照要求进行了修复，请重新进行全面复核

### 同步提案实现的内容到资产目录中

```
请将本次 openspec/changes/archive/2026-03-18-fix-session-idle-reaper 提案的全部内容与仓库中的 .context 文件进行逐段对齐与同步：先完整读取两者，按主题/章节建立对应关系，精确找出不一致之处（新增、缺失、表述冲突、命名/路径/接口/时间线差异、范围边界不一致、术语不统一、状态与结论不一致等），在不改变提案意图的前提下给出最小且可审计的同步修改方案；最终输出同步后的结果文本（同时给出更新后的 .context 内容与必要的提案修订片段），并在末尾用简洁文字列出所有发生变更的点及其原因与依据，确保同步后两者在目标、范围、约束、假设、里程碑、验收标准与风险/未决项上完全一致。
```

### 用 `context-update sync` 同步 roadmap / proposal 到资产目录

#### 先看同步计划
```bash
/context-update sync roadmap --mode review
```

#### 只同步某个阶段的 roadmap 到 `.context/**`
```bash
/context-update sync roadmap phase3 --mode review
/context-update sync roadmap phase3 --mode apply
```

#### 只同步某个 roadmap 文件
```bash
/context-update sync roadmap openspec/proposal-roadmap-Phase3.md --mode review
```

#### 同步单个 proposal 到 `.context/**`
```bash
# 若该 change 已归档：仍使用 <change-id>，但读取来源为 `openspec/changes/archive/YYYY-MM-DD-<change-id>/`
/context-update sync proposal feat-worker-heartbeat --mode review
/context-update sync proposal feat-observability-logging --mode apply
```

#### 推荐做法
```text
1. 先用 `--mode review` 看映射关系和差异分类
2. 确认没有 `conflict` 后，再执行 `--mode apply`
3. `apply` 后应同步检查 `.context/**`、`.context/**/source/**`、`.context/context-manifest.json`
```

### 检查提案大纲
```
/context-check review 请以系统分析师与产品架构评审者的身份，对 openspec/proposal-roadmap-Phase3.5.md 进行逐条对照审查，判断其整体提案计划是否满足 .context 资产目录中定义的系统业务要求与约束，并给出可追溯的证据引用（需标明来源文件与相关段落/标题）。审查时重点聚焦“对原有系统的改造是否充分”，至少覆盖：现有能力与目标能力的差距是否被明确识别；遗留模块/数据/接口/权限/流程的改造范围是否完整；数据迁移与一致性策略是否可执行；与上下游系统的集成与兼容性计划是否明确；运行与运维（监控、日志、告警、容量、备份、灾备、回滚）是否纳入；非功能需求（性能、可用性、安全、合规、可审计性）是否被映射到路线图交付物；阶段划分、里程碑、依赖与风险控制是否能支撑平滑演进；对现网影响、灰度发布、回退方案与变更窗口是否充分。输出需包含：总体结论（符合/部分符合/不符合及理由）、需求覆盖与缺口说明、对原系统改造充分性评估（指出遗漏改造点或描述不清之处）、关键风险与影响分析（含优先级与触发条件）、以及对 proposal-roadmap-Phase3.5.md 的具体修订建议（直接给出应补充/改写的内容要点与放置位置）。禁止泛泛而谈，所有结论必须与 .context 要求或文档内容建立明确对应关系。
```

### 检查提案大纲（UI）
```
/context-check review 请以系统分析师与产品架构评审者的身份，通读并交叉核对以下资产：.context 资产目录中的系统业务要求（含任何需求条目、约束、非功能要求、范围边界、里程碑与验收口径）、以及 openspec/proposal-roadmap-Phase3.5.md。你的任务是判断 proposal-roadmap-Phase3.5.md 的提案大纲与计划是否完整覆盖并严格符合 .context 业务要求与 /context-check.md 的检查口径，同时确认最终原型实现必须对齐并参考 .context/ui/sme/stitch（组件、交互、信息架构、视觉与状态规范），避免出现与 stitch 不一致的 UI/交互设计。请输出一份评审结论，内容需包含：总体符合度结论与关键理由；需求覆盖对照表（每条业务要求对应到 proposal-roadmap-Phase3.5.md 的具体章节/条目/里程碑，标注覆盖状态：已覆盖/部分覆盖/未覆盖，并说明证据）；发现的问题清单（缺口、矛盾、模糊点、范围漂移、依赖与风险、与 /context-check.md 不一致之处）；对 proposal-roadmap-Phase3.5.md 的最小必要修改建议（精确到应新增/调整的章节标题、条目内容与里程碑/交付物描述）；原型实现对齐 stitch 的核对结果（哪些页面/组件/流程需要按 stitch 修正或补齐，指出具体不一致点与建议做法）；最后给出可执行的验收标准与待确认问题（仅保留阻塞性问题）。输出使用中文，引用文件时使用准确路径与可定位的章节/条目名称，不要泛泛而谈。
```

### 检查整体大纲和资产目录一致性
```
 /context-check review 检查 `openspec/proposal-roadmap.md` 及所有相关的详细提案大纲（Proposal Outlines）是否严格遵循 `@/.context/` （包括源文件`source/`）中的规范与指南。请重点验证提案大纲文档与资产目录文档（Asset Inventory）之间的一致性，确保所有引用的资产、路径及元数据在两个文档中完全同步。发现任何结构偏差、缺失链接或逻辑冲突请详细列出。
```

## plan&plancontext

### 提案大纲和提案内容的一致性检查
```
/context-check review 严格核对 `openspec/proposal-roadmap-Phase3.md` 中的 `feat-worker-heartbeat` 大纲计划与 `openspec/changes/archive/2026-03-28-feat-worker-heartbeat` 提案内容的一致性。请逐项检查提案是否完整覆盖大纲的所有里程碑、任务和预期交付物，并确保两者在目标、范围、约束、假设、时间线、验收标准与风险/未决项上完全一致。输出需包含：
- 总体符合度结论（完全一致/部分一致/不一致及理由）
- 需求覆盖对照表（大纲每项对应提案的章节/条目，标注覆盖状态与证据）
- 发现的问题清单（缺口、矛盾、模糊点、范围漂移、依赖与风险）
- 对提案的最小必要修改建议（精确到应新增/调整的章节标题、条目内容与交付物描述）
- 结论性判断与下一步推荐动作

引用文件时使用准确路径与可定位的章节/条目名称，不要泛泛而谈。
```

## plan&code

### 检查代码是否实现
```
/context-check review 对照 `openspec/changes/archive/2026-03-18-fix-session-idle-reaper` 提案的要求，根据最新的代码提交记录验证该提案是否已全部完成。逐项核对提案中列出的修改点，对比代码实现与提案意图，确认每项功能需求是否已在代码中实现，标记任何未完成或存在差异的部分，并提供详细的检查结果说明。
```

### 检查提案和已有系统的实际情况
```
/context-check review  请基于当前仓库中`/Users/chenaifeiyang/Desktop/wecloud/injoysai/openfleet/crates`目录下的真实代码实现（以现有接口、数据结构、路由/页面装配方式、状态管理、权限与特性开关、API 调用与返回结构、UI 组件与布局约束为准），对 openspec/changes/feat-ui-home-page 提案进行逐条复核，判断提案描述与现状实现是否一致、是否可直接落地、以及是否存在与实现相冲突或遗漏的部分。要求以代码为依据，不做臆测；在指出任何不一致时，明确对应的 crate/模块/文件路径（若可）与相关类型/函数/配置项名称，并解释冲突原因（例如现有字段不存在、命名不同、生命周期/所有权限制、异步边界、权限模型不支持、路由机制不同、UI 组件能力不足、缓存/分页/排序策略差异等）。同时给出最小修改建议：提案应如何改写以匹配现状实现，或实现侧需要补哪些变更才能满足提案（含影响范围、兼容性风险、潜在回归点、需要补充的测试与文档）。最后给出结论性判断：提案与当前实现是完全一致、部分一致（列出需要调整的关键点）、还是不一致（列出阻塞点与替代方案），并说明下一步推荐动作。
```

## UI原型

### stitch原型对齐
```
根据原型细节与提案约束的偏差，并将其整合至文件 `openspec/changes/feat-ui-shell-layout/stitch_video_production_workbench/DESIGN.md` 中。请确保所有的增补与修改均严格遵循该文件中既有的文档结构、命名规范以及 Markdown 格式风格。切勿遗漏任何与工作台布局变更相关的技术细节，并确保更新后的设计与 `feat-ui-shell-layout` 分支中所定义的整体功能范畴保持一致。
```

## 检查
```
## 审查修复完成

### RBAC 整改完成清单

| 文档 | 修改项 | 状态 |
|------|-------|:----:|
| **PRD** §2.1 | "不区分权限" → "命令级最小 RBAC（P1）" | ✅ |
| **PRD** §2.4 | ❌ 角色权限 → ⚠️ 命令级最小 RBAC | ✅ |
| **PRD** §3.7 | 新增 RBAC 失败路径（forbidden / not_member / owner_lost） | ✅ |
| **PRD** §4.3 | IM 群 → IM 群 + 命令级 RBAC，角色定义表 | ✅ |
| **PRD** §7 | "引入角色权限" → "V0 已有命令级 RBAC" | ✅ |
| **架构** §4.1 | 消息路由增加 AuthManager 鉴权前置检查 | ✅ |
| **架构** §7.2 | 新增独立章节：身份绑定策略、权限矩阵、鉴权链路、数据同步、Owner 接管 | ✅ |
| **架构** §9.1 | `chat_bindings` 增加 owner_user_id；新增 `chat_members`、`auth_audit_log` 表 | ✅ |
| **异常处理** §3.4 | 新增 AG-004~007（forbidden / not_bound / not_member / owner_lost） | ✅ |
| **异常处理** §6 | 级联故障增加 RBAC 规则（AG-004 立即拒绝、AG-005 引导绑定） | ✅ |
| **异常处理** 映射表 | 新增 EH-V0-017 | ✅ |
## 下一步
已经完成修复，请重新开始复核

```