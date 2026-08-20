# CATs ロジック設計書 v1.0

> **文档编号**：CATs-DD-045（CATs ロジック設計）  
> **フェーズ**：45 ロジック設計  
> **关联任务**：150 任务 #45、#5-#11（F1-F11 核心算法）  
> **版本**：v1.0（评审会前草稿）  
> **创建日**：2026-08-20  
> **作者**：架构师 + 核心开发者

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| 架构师 | ☐ | — |
| Rust Lead | ☐ | — |
| 算法工程师 | ☐ | — |
| QA | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-20** | **架构师** | **评审前草稿：8 大核心算法详细逻辑** |

---

## 1. 目的

为 CATs 的**核心业务逻辑**提供详细算法设计，作为：

- 详细设计评审（DD Review）的核心内容
- 性能优化（PT）的依据
- 实现的"参考实现"基线
- 单元测试用例的输入

---

## 2. 范围

### 2.1 包含

| # | 算法 | 服务 | 关联 F |
|---|------|------|-------|
| L-1 | TM 混合召回 | tm-service | F3 |
| L-2 | 术语模糊匹配 | term-service | F4 |
| L-3 | LLM 翻译 + 后编辑 | llm-gateway | F5 |
| L-4 | QA 规则引擎 | qa-engine | F7 |
| L-5 | 翻译记忆 ETL | worker-svc | F3/F11 |
| L-6 | 实时协作（CRDT） | 前端 | F1/F2 |
| L-7 | 增量同步（Outbox） | 所有服务 | 跨 F |
| L-8 | 角色权限检查（RBAC + ABAC） | auth-svc | F10 |

---

## 3. L-1 TM 混合召回

### 3.1 输入

```rust
pub struct MatchInput {
    pub source_text: String,
    pub source_lang: Lang,
    pub target_lang: Lang,
    pub project_id: ProjectId,
    pub limit: usize,                  // 默认 10
    pub min_similarity: f32,           // 默认 0.85
    pub filters: MatchFilters,         // 域、租户、时间
}
```

### 3.2 输出

```rust
pub struct MatchResult {
    pub matches: Vec<Match>,
    pub total_candidates: usize,       // 候选数
    pub latency_ms: u64,
    pub engine_breakdown: EngineStats, // 各引擎贡献
}

pub struct Match {
    pub segment_id: SegmentId,
    pub tu_id: TuId,
    pub source_text: String,
    pub target_text: String,
    pub similarity: f32,
    pub match_type: MatchType,         // Exact/Fuzzy95/Fuzzy85/Semantic
    pub metadata: SegmentMetadata,
}
```

### 3.3 算法

```
INPUT: source_text, project_id, filters
OUTPUT: matches[] (top-K, similarity >= threshold)

1. EMBEDDING
   - 调用 embed service (或 LLM gateway) 获取 1024-dim 向量
   - 缓存：同 (text, lang) 1h 缓存
   - 失败：降级到纯 fuzzy 检索

2. PARALLEL SEARCH (3 路并发)
   a. Vector Search (HNSW)
      - pgvector: cosine distance
      - filter: project_id IN [project, tenant_shared]
      - filter: source_lang = input.source_lang
      - filter: target_lang = input.target_lang
      - top-K * 2
   b. Trigram Search (B-tree GIN)
      - pg_trgm: similarity(text, source_text) > 0.5
      - filter: same as above
      - top-K * 2
   c. Exact Match (B-tree)
      - WHERE source_text = input.source_text
      - LIMIT K

3. MERGE & DEDUPE
   - 按 segment_id 去重
   - 保留最高分
   - 取 top-K * 3 进入排序

4. RANK
   - 综合分 = 0.6 * vector_sim + 0.3 * trgm_sim + 0.1 * exact_boost
   - 二次精排：基于项目域 / 客户偏好 / 复用次数
   - 截断到 top-K

5. POST-FILTER
   - min_similarity 过滤
   - 领域白名单
   - 时间新鲜度（默认不限）

6. RETURN matches
```

### 3.4 性能

| 数据量 | P95 |
|--------|-----|
| 10 万 | < 200ms |
| 100 万 | < 500ms |
| 1000 万 | < 1s (分桶) |

### 3.5 QA-011 决策点

TM 索引策略选型影响 L-1 性能：

| 方案 | 适用 | 决策 |
|------|------|------|
| 全表 HNSW | < 1000 万 | 默认 |
| 分桶（按客户/项目） | 1000 万 ~ 1 亿 | ⏳ QA-011 决议 |
| Scale-out（多 PG） | > 1 亿 | 远期 |

---

## 4. L-2 术语模糊匹配

### 4.1 输入

```rust
pub struct TermLookupInput {
    pub query: String,
    pub source_lang: Lang,
    pub target_lang: Lang,
    pub domain: Option<String>,
    pub project_id: ProjectId,
    pub limit: usize,                  // 默认 20
}
```

### 4.2 输出

```rust
pub struct TermLookupResult {
    pub terms: Vec<TermHit>,
}

pub struct TermHit {
    pub term_id: TermId,
    pub source_text: String,
    pub target_text: String,
    pub score: f32,                    // 0-1
    pub match_type: TermMatchType,     // Exact/Prefix/Fuzzy/Synonym
    pub definition: Option<String>,
    pub context: Option<String>,
    pub usage_count: u32,
}
```

### 4.3 算法

```
INPUT: query, source_lang, target_lang, project_id, domain
OUTPUT: terms[] (top-K)

1. NORMALIZE
   - lowercase
   - 去除标点
   - 词形还原（lemmatization）

2. EXACT MATCH (B-tree)
   - WHERE LOWER(source_text) = LOWER(query)
   - LIMIT K
   - 立即返回（如果命中且不需要 fuzzy）

3. PREFIX MATCH
   - WHERE LOWER(source_text) LIKE LOWER(query) || '%'
   - LIMIT K/2

4. FUZZY MATCH (trigram)
   - WHERE similarity(LOWER(source_text), LOWER(query)) > 0.4
   - ORDER BY similarity DESC
   - LIMIT K

5. SYNONYM MATCH
   - 查询同义词词典
   - 展开为多个 query 重试 step 2-4

6. RANK
   - 优先级: Exact (1.0) > Prefix (0.8) > Fuzzy (similarity) > Synonym (0.5)
   - 项目级术语优先于全局
   - usage_count 加权

7. RETURN
```

### 4.4 性能

- 单次查询 P95 < 50ms
- 支持 1M 术语

---

## 5. L-3 LLM 翻译 + 后编辑

### 5.1 输入

```rust
pub struct TranslateInput {
    pub source_text: String,
    pub source_lang: Lang,
    pub target_lang: Lang,
    pub domain: Option<String>,
    pub style_guide: Option<String>,   // 风格指南
    pub tm_hints: Vec<Match>,          // TM 候选
    pub term_hints: Vec<TermHit>,      // 术语候选
    pub max_tokens: u32,               // 默认 1024
    pub temperature: f32,              // 默认 0.3
}
```

### 5.2 输出

```rust
pub struct TranslateOutput {
    pub translated_text: String,
    pub confidence: f32,               // 模型自评
    pub tokens_used: TokenUsage,
    pub latency_ms: u64,
    pub model: String,
    pub cache_hit: bool,
}
```

### 5.3 算法

```
INPUT: source_text + context (TM/term hints)
OUTPUT: translated_text

1. CACHE CHECK
   - Key: hash(source_text + langs + tm_hints ids + term_hints ids)
   - Hit: 直接返回（cache_hit = true）
   - TTL: 24h

2. PROMPT ASSEMBLY (3 段式)
   - SYSTEM: 你是 CAT 系统的本地化专家
     + 风格指南（如有）
     + 必须遵守的术语（来自 term_hints）
     + 参考译文（来自 tm_hints top-3）
   - USER: 翻译以下文本：{source_text}
   - CONSTRAINTS: 输出格式（保留占位符、标签、数字不变）

3. INFERENCE
   - 调用 llama.cpp / vLLM
   - 设置 temperature / top_p
   - 限制 max_tokens
   - 监控 latency（timeout 5s）

4. POST-PROCESS
   - 恢复占位符（如有）
   - 术语合规检查（确保 term_hints 中的术语被使用）
   - 长度检查（不超过原文 1.5x）
   - 失败 → 重试（最多 1 次）→ 降级（返回原文 + 警告）

5. CACHE STORE
   - 写缓存（异步）

6. RETURN
```

### 5.4 关键参数

| 参数 | 值 | 理由 |
|------|-----|------|
| temperature | 0.3 | 翻译需要稳定 |
| top_p | 0.9 | 平衡多样性 |
| max_tokens | 1024 | 段落级 |
| context window | 8K | 模型支持 |
| timeout | 5s | 用户体验 |
| retry | 1 | 失败不重试 |
| cache TTL | 24h | 复用 |

### 5.5 降级策略

```
LLM 不可用 → TM/术语补全 → 原文 + "需人工翻译"标记
```

---

## 6. L-4 QA 规则引擎

### 6.1 输入

```rust
pub struct ScanInput {
    pub document: Document,
    pub rule_set: RuleSet,             // 启用规则集
    pub lang: Lang,
    pub context: ScanContext,          // TM/term 等
}
```

### 6.2 输出

```rust
pub struct ScanResult {
    pub issues: Vec<Issue>,
    pub stats: ScanStats,
}

pub struct Issue {
    pub rule_id: RuleId,
    pub severity: Severity,            // Error/Warning/Info
    pub segment_id: SegmentId,
    pub position: Position,
    pub message: String,
    pub suggestion: Option<String>,
}
```

### 6.3 算法

```
INPUT: document, rule_set
OUTPUT: issues[]

1. LOAD RULES
   - 从 RuleSet 中加载启用的规则
   - 每条规则: id / name / severity / check_fn

2. PARALLEL EXECUTE
   - 对每条规则异步执行
   - 超时：单规则 1s
   - 失败：记录 + 跳过

3. EACH RULE: check(segment, context) -> Vec<Issue>
   - TermConsistency: 术语使用是否一致
   - NumberConsistency: 数字/日期/单位
   - TagConsistency: 标签是否平衡
   - LengthConsistency: 长度合理性
   - PlaceholderIntegrity: 占位符
   - PunctuationConsistency: 标点
   - StyleGuide: 自定义
   - TmReuseCheck: TM 复用建议
   - ...

4. AGGREGATE
   - 合并所有 issue
   - 按 segment_id 分组
   - 按 severity 排序

5. RETURN
```

### 6.4 规则扩展

```rust
pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn severity(&self) -> Severity;
    fn check(&self, segment: &Segment, ctx: &ScanContext) -> AppResult<Vec<Issue>>;
}
```

注册：

```rust
let engine = RuleEngine::new()
    .register(TermConsistencyRule::new())
    .register(NumberConsistencyRule::new())
    .register(TagConsistencyRule::new());
```

---

## 7. L-5 翻译记忆 ETL

### 7.1 输入

```rust
pub struct EtlInput {
    pub source: EtlSource,             // TMX/XLIFF/CSV/Trados/MemoQ
    pub target_project: ProjectId,
    pub mapping: FieldMapping,
    pub filters: EtlFilters,
    pub batch_size: usize,             // 默认 1000
}
```

### 7.2 输出

```rust
pub struct EtlResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<EtlError>,
    pub duration_ms: u64,
    pub embedding_generated: usize,
}
```

### 7.3 算法

```
INPUT: source file/stream
OUTPUT: ETL result

1. STREAM READ
   - 解析 TMX/XLIFF/CSV
   - 验证 schema

2. TRANSFORM
   - 字段映射
   - 过滤（长度、域、语言对）
   - 标准化（标点、空格、Unicode）

3. DEDUPE
   - 内存 Bloom Filter (10M bit)
   - DB query: 已有 (source_text, lang, project)?

4. EMBED (异步 + 批量)
   - 调用 embed service
   - 批量大小 100
   - 失败重试 3 次

5. BULK INSERT (COPY)
   - pg COPY BINARY
   - 每批 1000
   - 事务提交

6. UPDATE INDEX
   - HNSW 异步更新
   - 触发 ANALYZE

7. RETURN
```

### 7.4 性能

- 吞吐：≥ 10 万 TU/分钟（普通 8 核）
- 内存：Bloom Filter 固定 10M bit
- 失败恢复：checkpoint 每 10K 行

---

## 8. L-6 实时协作（CRDT）

### 8.1 数据结构

```typescript
// Yjs Doc 结构
interface YDoc {
  meta: Y.Map;                        // 文档元信息
  segments: Y.Array<Y.Map>;           // 段数组
  // 每段：
  //   { id, source, target, status, version, comments, ... }
  comments: Y.Array<Y.Map>;           // 评论
}
```

### 8.2 协作算法

```
INPUT: 客户端 A 改动 segment[5].target
OUTPUT: 全员同步

1. LOCAL EDIT (A)
   - Y.Map.set("target", newValue)
   - Yjs 生成 operation: { segmentId, field, op }

2. BROADCAST (A -> Server)
   - 通过 WebSocket 发送 update
   - 编码为 binary update

3. SERVER (中心化)
   - 接收 update
   - 应用到 server-side Doc
   - 广播给其他客户端（B/C/D）

4. APPLY (B/C/D)
   - 收到 update
   - Yjs merge (无冲突)
   - 更新 UI

5. PERSIST
   - 每 5s / 100 ops 写入 Postgres
   - snapshot + delta
```

### 8.3 一致性

- Yjs CRDT 保证最终一致
- 离线支持：客户端保存本地 update，重连后 sync

### 8.4 性能

- 100 并发用户编辑：< 100ms 同步
- 单文档 1000 段：< 50ms 应用

---

## 9. L-7 增量同步（Outbox）

### 9.1 算法

```
生产者：
  transaction {
    UPDATE segments SET ...;
    INSERT INTO outbox (event) VALUES (...);
  }

后台 Publisher：
  LOOP every 1s {
    rows = SELECT * FROM outbox WHERE published_at IS NULL LIMIT 100;
    FOR each row {
      TRY publish to Kafka;
      ON SUCCESS: UPDATE outbox SET published_at = NOW() WHERE id = row.id;
      ON FAILURE: log + retry later;
    }
  }
```

### 9.2 关键保证

- **原子性**：业务操作 + 事件写入 同事务
- **至少一次**：Kafka producer 幂等 + consumer 幂等
- **顺序**：单 aggregate 顺序保证（按 aggregate_id + created_at）
- **回溯**：未发布事件可重放

### 9.3 监控

- outbox 表堆积量（> 1000 告警）
- publish 失败率（> 1% 告警）
- publish 延迟（> 5s 告警）

---

## 10. L-8 角色权限检查（RBAC + ABAC）

### 10.1 输入

```rust
pub struct AuthCheck {
    pub user: User,
    pub resource: Resource,            // type + id
    pub action: Action,                // read/write/admin
    pub context: AccessContext,        // IP/time/项目等
}
```

### 10.2 输出

```rust
pub enum AuthResult {
    Allow,
    Deny { reason: String },
    RequireElevation { reason: String },
}
```

### 10.3 算法

```
INPUT: user, resource, action, context
OUTPUT: Allow / Deny / RequireElevation

1. ROLE LOOKUP
   - 用户角色：role[]
   - 资源类型：resource.type

2. ROLE PERMISSION
   - 对每个 role 查 Permission Matrix (role × resource_type × action)
   - 命中 → 进入 ABAC

3. ABAC CHECKS
   - 时间窗口：if (action in [sensitive]) check business_hours
   - IP 白名单：if (role == admin) check ip_whitelist
   - 数据标签：if (resource.tag in [restricted]) check extra_approval
   - 设备指纹：if (user.device != registered) deny

4. FINAL DECISION
   - 任一 ABAC fail → Deny
   - 全部通过 → Allow
   - 需要额外审批 → RequireElevation

5. LOG
   - 写 audit-svc
   - 包含: user, resource, action, decision, reason, context
```

### 10.4 权限矩阵（RBAC 基础）

| Role | project | document | term | tm | user | config |
|------|---------|----------|------|----|----- |--------|
| 译者 | R/W* | R/W* | R | R | — | — |
| 审校 | R/W* | R/W* | R | R | — | — |
| PM | R/W/A | R/W/A | R | R | R* | — |
| 术语专家 | R | R | R/W/A | R | — | — |
| QA | R | R | R | R | — | — |
| 管理员 | R/W/A | R/W/A | R/W/A | R/W/A | R/W/A | R/W/A |

*仅限被分配的项目

### 10.5 ABAC 增强（条件维度）

- 时间窗口
- IP 白名单
- 设备信任
- 数据敏感标签
- 紧急模式（incident 期间允许）

---

## 11. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 45 ロジック設計 | 本文 |
| F1-F11 需求 | 输入 |
| 接口设计书 | 调用契约 |
| 错误处理设计 | 降级路径 |
| 性能测试 | 基准 |
| 类图 | 实现结构 |
| 批处理详细 | L-5 ETL |

---

## 12. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_类图 v1.0 | `03-详细设计\类图\` |
| CATs_批处理详细设计 v1.0 | `03-详细设计\批处理\` |
| CATs_错误处理设计 v1.0 | 即将建（任务 50 独立化） |
| CATs_接口设计书 v2.0 | `03-详细设计\接口设计\` |
| CATs_数据库设计书 v2.0 | `03-详细设计\数据库设计\` |
| CATs_SQL 设计一览 v1.0 | `03-详细设计\SQL\` |
| CATs_需求规格说明书 v2.0 F1-F11 | `01-需求\需求规格说明\` |
| CATs_测试设计书 v1.0 §10 性能 | `04-测试\测试设计书\` |

---

## 13. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | TM 召回融合权重调优 | 算法 + QA | M1-S2 |
| OI-2 | LLM 提示词 A/B 测试 | 算法 + BA | M1-S2 |
| OI-3 | QA 规则扩展（行业特定） | QA + 客户 | M2-ST |
| OI-4 | ETL 性能基准（QA-011 影响） | 架构 + SRE | M1-S1 |
| OI-5 | ABAC 策略可视化配置 | 架构 + 前端 | M1-S1 |

---

**文档结束**
