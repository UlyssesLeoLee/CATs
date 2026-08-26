# CATs ロジック設計書 v1.0

> **文档编号**：CATs-DD-045（CATs ロジック設計）  
> **フェーズ**：45 ロジック設計  
> **关联任务**：150 任务 #45（DD 评审 v1.1 §4.2 评审对象 #2）  
> **版本**：v1.0（评审会前定稿）  
> **创建日**：2026-08-26  
> **作者**：算法工程师 + 架构师（worker 代签 per DEC-008）  
> **配套**：P2M3 待触发索引 v1.0 §2 #24 / DD 评审纪要 v1.1 §4.2

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 | 备注 |
|------|------|------|------|
| 起草 算法工程师 | ☑ | 2026-08-26 | v1.0 8 大算法骨架 |
| 起草 架构师 | ☑ | 2026-08-26 | 接口/数据/可观测对齐 |
| 评审 架构师（DD 主持） | ☐ | — | 评审会 D-Day |
| 评审 DBA / QA / SRE / 安全 | ☐ | — | 评审会 D-Day |
| 批准 PMO | ☐ | — | 评审会 D+2 |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| v0.x | 2026-08-20 | 架构师 + 核心开发者 | 评审前草稿（13 章 + 子节格式，已被 v1.0 取代） |
| **v1.0** | **2026-08-26** | **算法工程师 + 架构师（worker 代签 per DEC-008）** | **重写为 8 大算法（对齐 DD 评审 §4.2）+ 6 段模板（目标/输入输出/算法步骤/库模式/复杂度/测试）+ 上下游引用；ADR 引用 ≥ 5** |

---

## 1. 目的与范围

### 1.1 目的

为 CATs 8 大核心算法提供**可直接落库 / 写单测的算法级设计**，作为：

- **实现基线**：每个 L-X 对应一个共享库 trait + 单测夹具
- **评审依据**：DD 评审会 D-Day 唯一讨论对象（DD 评审 v1.1 §4.2）
- **性能基线**：对应 ADR-003 §3 目标（TM P95 < 500ms、LLM 流式 P95 < 5s、ABAC P95 < 5ms）
- **降级契约**：每个 L-X 必须定义"主路径失败 → 降级路径"的明确开关

### 1.2 8 大算法总览

| # | 算法 | 所属服务 | 主要模式 |
|---|------|----------|----------|
| L-1 | TM 召回（3 路并行 + 融合） | tm-service | 并行召回 + 加权融合 |
| L-2 | 术语匹配（Exact / Prefix / Fuzzy 三级） | term-service | 多级 Fallback 链 |
| L-3 | LLM 翻译（缓存 + 提示词 + 降级） | llm-gateway | 多级缓存 + 4 级降级 |
| L-4 | QA 规则（可扩展 trait 链） | qa-engine | trait 链 + 异步并行 |
| L-5 | ETL（流式 + 批量） | worker-svc | 流式读 + COPY 批量写 |
| L-6 | CRDT 协同（Yjs + WebSocket 冲突合并） | collab-ws / -persistence | CRDT + 周期快照 |
| L-7 | Outbox（事务性事件投递） | 15 服务（共享库） | Transactional Outbox + CDC |
| L-8 | RBAC + ABAC（矩阵 + 条件求值） | auth-svc + 共享库 | RBAC 矩阵 + 条件 DSL |

不包含：UI 交互算法（ADR-004）、批处理调度（批处理详细设计 v1.0）、LLM 训练 / 微调（M3 末评估）。

---

## 2. L-1 TM 召回（3 路并行 + 融合）

### 2.1 目标

对一条源文片段，并行执行 **精确匹配 / 编辑距离 / 向量检索** 三路召回，加权融合返回 top-K。

### 2.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `source_text` (String ≤ 4KB) / `source_lang` / `target_lang` (BCP-47) / `project_id` / `limit` (默认 10) / `min_similarity` (默认 0.85) |
| 出 | `Vec<Match>`（含 `similarity`、`match_type`=Exact/Fuzzy95/Fuzzy85/Semantic） + `engine_breakdown` (3 路各自命中数 / 耗时) |

### 2.3 算法步骤

1. **Embedding 缓存查询**：key=`hash(text+lang)`，Redis 7 TTL 1h；未命中 → 异步触发 embed（不阻塞）。
2. **3 路并行**（`tokio::join!`）：a. 精确 `WHERE source_text=$1`（B-tree, LIMIT K）；b. 编辑距离 `pg_trgm.similarity>0.5`（GIN, LIMIT K×2）；c. 向量 `pgvector.cosine_distance<0.2`（HNSW, LIMIT K×2）。
3. **去重 + 融合**：`HashMap<segment_id, ScoredMatch>` 去重；综合分 = `0.6·vector_sim + 0.3·trgm_sim + 0.1·exact_boost`。
4. **二次精排**：项目域白名单 + 客户偏好 + 历史复用（`tm_usage_stats`）。
5. **Post-filter**：`min_similarity` 截断 + 租户 RLS（per ADR-005 §2.3）。
6. **降级**：embed 故障 → 跳 c 路 `weights=(0,0.7,0.3)`；trigram 未建 → 跳 b 路仅 a+c。

### 2.4 关键 Rust 库 / 模式

- `sqlx` 0.8（async PG + RLS session var）/ `pgvector` 0.4（`cosine_distance`）/ `tokio` 1.40（`join!`）/ `reqwest` 0.12（embed HTTP）
- 模式：**Read-Through 缓存** + **多路召回 + 加权融合**（RAG 范式）

### 2.5 复杂度

- 时间：单段 P95 < 500ms（10万 TU 基线；1000万 TU 走分桶，per ADR-003 §3 + QA-011 已决）
- 空间：embedding 1024-d float32 × N；单段缓存 4KB
- DB N+1 风险：⚠ 中（精排查 `tm_usage_stats` 必须 `IN (...)` 批量，禁止段级循环）

### 2.6 测试要点

- **单测**（`mockall` 0.13）：3 路分别命中 / 全未命中 / embed 降级 / 租户 RLS 拒绝
- **集成**（`testcontainers-rs` 0.20 + 真 PG + pgvector）：10万 TU P95 采样
- **属性**（`proptest` 1.5）：任意 source_text 1s 内必有结果（即使空）
- **性能**（`criterion` 0.5）：10万 / 100万 / 1000万 三档基准

### 2.7 上下游引用

- 上游：需求 F3 / 接口书 v2.0 §3.5 `MatchSegment` gRPC
- 下游：term-service（L-2 hint）/ llm-gateway（L-3 hint）/ PG `tm_segments` + `tm_embeddings`
- 关联 ADR：**ADR-001 §3**（tm-service 边界）/ **ADR-002 §3**（gRPC）/ **ADR-003 §3**（pgvector HNSW + 分桶）/ **ADR-007 §3**（gRPC 客户端重试）

---

## 3. L-2 术语匹配（Exact / Prefix / Fuzzy 三级）

### 3.1 目标

按 **Exact → Prefix → Fuzzy** 三级 Fallback 链返回 top-K 术语命中，首屏 < 50ms。

### 3.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `query` (String ≤ 256B) / `source_lang` / `target_lang` / `domain` (Option) / `project_id` / `limit` (默认 20) |
| 出 | `Vec<TermHit>`（`score` 0-1、`match_type`=Exact/Prefix/Fuzzy/Synonym、`usage_count`） |

### 3.3 算法步骤

1. **Normalize**：`lowercase` + 去标点 + Unicode NFC + 英文词形还原（`rust-stemmers` 0.2，按需）。
2. **Exact 级**：`LOWER(source_text)=LOWER($1)`，B-tree 索引，**命中即返回**。
3. **Prefix 级**：`LOWER(source_text) LIKE LOWER($1)||'%'`，B-tree，LIMIT K/2。
4. **Fuzzy 级**（`pg_trgm`）：`similarity>0.4`，GIN，LIMIT K。
5. **Synonym 展开**：查 `term_synonyms`，对每个同义词重跑 step 2-4（上限 3 个避免爆炸）。
6. **排序**：Exact (1.0) > Prefix (0.8) > Fuzzy (similarity) > Synonym (0.5)；同分按 `usage_count DESC` + 项目级优先。
7. **降级**：trigram 缺失 → 跳 Fuzzy，仅 Prefix+Exact；项目表空 → 回退租户级。

### 3.4 关键 Rust 库 / 模式

- `sqlx` 0.8 / `pg_trgm`（DB 侧扩展）/ `rust-stemmers` 0.2 / `unicode-normalization` 0.1
- 模式：**多级 Fallback 链**（每级独立可降级）

### 3.5 复杂度

- 时间：单次 P95 < 50ms（1M 术语，per 接口书 §3.6 NFR）
- 空间：每术语 ~200B + GIN 索引 ~3× 表大小
- DB N+1 风险：⚠ 低（Synonym 展开 `IN (...)` 批量，禁止循环）

### 3.6 测试要点

- **单测**：Exact 命中短路 / Prefix CJK 边界 / Fuzzy 阈值 0.4 上下 / Synonym 上限
- **集成**（testcontainers-rs）：1M 术语 P95 + `EXPLAIN` 必含 Index Scan
- **i18n**：CJK / 阿拉伯 RTL / 希腊字符 Normalize 一致性

### 3.7 上下游引用

- 上游：需求 F4 / tm-service（L-1 term hint）
- 下游：llm-gateway（L-3 prompt 组装）/ 前端术语高亮
- 关联 ADR：**ADR-002 §3**（gRPC）/ **ADR-003 §2**（PG tsvector + GIN）/ **ADR-005 §2.3**（多租户 schema）

---

## 4. L-3 LLM 翻译（缓存 + 提示词 + 降级）

### 4.1 目标

调用 LLM 生成译文，**4 级降级**保证任何 LLM 故障下用户拿到结果（最差兜底为源文 + "需人工"）。

### 4.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `source_text` (≤ 4KB) / `source_lang` / `target_lang` / `tm_hints` (Vec<Match>, 来自 L-1 top-3) / `term_hints` (Vec<TermHit>, 强制术语) / `style_guide` (Option) / `compliance_mode` (Local/Cloud, per ADR-010) |
| 出 | `translated_text` / `confidence` (f32) / `tokens_used` / `latency_ms` / `cache_hit` / `fallback_level` (1-4) |

### 4.3 算法步骤

1. **L1 缓存**：key=`hash(text+langs+tm_hint_ids+term_hint_ids)`，Redis 7 Cluster TTL 24h；命中即返回。
2. **Prompt 3 段组装**：System（角色 + 风格指南 + **强制术语清单**）/ User（源文）/ Constraints（保留占位符 + 数字不变 + 长度 ≤ 1.5× 原文）。
3. **推理路由**（per ADR-010 §3）：Local → vLLM（Qwen2.5-7B-Instruct-AWQ 单卡 RTX 4090）；Cloud → LiteLLM Proxy → `gpt-4o-mini` / `claude-3-5-sonnet`。
4. **流式推理**：`tokio::time::timeout(5s)` + `tokio_stream` 按 token 流式；前端 SSE 推首 token。
5. **后处理**：恢复占位符（正则）+ 术语合规校验（强制清单全部出现）+ 长度检查（>1.5× 截断 + 警告）。
6. **L1 缓存写**（异步，不阻塞响应）。
7. **降级链**：L2 Cloud→Local（项目允许时）/ L3 TM top-1 直出（similarity>0.95）/ L4 原文+警告。

### 4.4 关键 Rust 库 / 模式

- `reqwest` 0.12（SSE stream，含 `eventsource` 适配）/ `redis` 0.27（`deadpool-redis`）/ `tokio` 1.40（`timeout` + `Stream`）/ `async-openai` 0.25 / `handlebars` 6
- 模式：**Read-Through 缓存** + **Circuit Breaker**（`failsafe` 1.2）+ **多级降级**

### 4.5 复杂度

- 时间：P95 < 5s，首 token < 800ms（per 接口书 §3.7 NFR）
- 空间：缓存 ~1KB/条，100万条 ≈ 1GB Redis
- DB N+1 风险：⚠ 无（纯外部 API）

### 4.6 测试要点

- **单测**（`wiremock` 0.6）：4 级降级路径 / 占位符保真 / 超时降级 / 强制术语清单
- **集成**（真 vLLM docker）：首 token 延迟 / 5s 超时边界
- **A/B**（M1-S2 后）：`temperature=0.2 vs 0.3` 翻译质量盲评（OI-2 跟踪）
- **成本基线**：`metrics-exporter-prometheus` 暴露 `llm_token_total{deployment,model}` → billing 面板

### 4.7 上下游引用

- 上游：需求 F5 / tm-service（L-1 hints）/ term-service（L-2 hints）
- 下游：前端 SSE 流式 / billing 域（云端成本按租户聚合）
- 关联 ADR：**ADR-002 §3**（`TranslateSegment` gRPC）/ **ADR-005 §3**（多租户配额）/ **ADR-010 §3**（混合部署 + LiteLLM）

---

## 5. L-4 QA 规则（可扩展 trait 链）

### 5.1 目标

对翻译后文档执行**可插拔 QA 规则**，输出问题清单（Error/Warning/Info），支持第三方热加载。

### 5.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `document` (Document) / `rule_set` (RuleSet 启用规则 ID 集合) / `lang` / `context` (ScanContext: TM/term) |
| 出 | `Vec<Issue>`（`rule_id`+`severity`+`segment_id`+`position`+`message`+`suggestion?`） + `ScanStats` |

### 5.3 算法步骤

1. **加载规则集**：从 `RuleSet` 取启用规则 ID；从 `RuleRegistry`（共享库 `common-grpc`）取 `Box<dyn Rule>`。
2. **并行执行**（`tokio::task::JoinSet`）：每条规则独立 task，`timeout(1s)`；失败 → 记录 + 跳过该规则（不影响其他）。
3. **每条规则 `check(segment, ctx) -> Vec<Issue>`**：内置 `TermConsistency` / `NumberConsistency` / `TagConsistency` / `LengthConsistency` / `PlaceholderIntegrity` / `PunctuationConsistency` / `TmReuseCheck`；自定义实现 `Rule` trait 注册。
4. **聚合**：按 `segment_id` 分组 → severity 排序（Error>Warning>Info）→ 同 `rule_id+position` 去重。
5. **降级**：单条失败仅记日志，扫描继续；规则加载失败 → 走默认集（仅内置）。

### 5.4 关键 Rust 库 / 模式

- `async-trait` 0.1（`trait Rule: Send + Sync`）/ `tokio` 1.40（`JoinSet` + `timeout`）/ `inventory` 0.3（插件注册）/ `dashmap` 6.1（注册表并发安全）
- 模式：**Strategy / Plugin**（trait + 运行时注册）+ **Bulkhead**（单规则超时隔离）

### 5.5 复杂度

- 时间：1000 段 × 10 规则 P95 < 3s（单规则 < 1s + 并行）
- 空间：每 Issue ~200B；10K 段 ~2MB
- DB N+1 风险：⚠ 中（`TermConsistency` 查术语库必须 `IN (...)`，禁止段级循环）

### 5.6 测试要点

- **单测**：trait 抽象 / 规则注册-反注册 / 单规则超时隔离 / severity 排序
- **集成**（testcontainers-rs）：1000 段 + 10 规则端到端 P95
- **属性**（proptest）：任意 `Document` 10s 内必有结果
- **示例 crate**：`cats-rule-template` 提供 trait 骨架 + 1 个示例规则（OI-3）

### 5.7 上下游引用

- 上游：需求 F7 / llm-gateway（L-3 输出后触发扫描）
- 下游：前端问题面板 / audit 域（Issue 落库）
- 关联 ADR：**ADR-001 §3**（qa-engine 边界）/ **ADR-002 §3**（gRPC）

---

## 6. L-5 ETL（流式 + 批量）

### 6.1 目标

将外部 TMX / XLIFF / CSV / Trados / MemoQ **流式读入 + 批量写入** PG，同时生成 embedding，**吞吐 ≥ 10 万 TU/分钟**。

### 6.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `source` (EtlSource: 文件/S3 流) / `target_project` / `mapping` (FieldMapping) / `filters` / `batch_size` (默认 1000) |
| 出 | `imported` / `skipped` / `errors` / `embedding_generated` / `duration_ms` |

### 6.3 算法步骤

1. **流式解析**（`tokio::io::BufReader` + `quick-xml` 0.36 / `csv` 1.3）：按 chunk 读，**不一次性加载整个文件**。
2. **Schema 校验**：每行用 `validator` 0.18 derive 校验；失败计入 `errors`。
3. **Transform**：字段映射 + 过滤（长度/域/语言对）+ 标准化（Unicode NFC + 去 BOM）。
4. **内存去重**（`bloomfilter` 2.0）：10M bit Bloom（~1% 误判）；DB 二次校验 `(source_text, lang, project_id)`。
5. **Embedding 异步**：攒批 100 → 调 embed gRPC（`tonic` 0.12 client）；失败重试 3 次（per ADR-007 §3 指数退避）。
6. **Bulk Insert**：`sqlx` `PgCopyIn` COPY BINARY（per ADR-003 §3），每批 1000，事务提交。
7. **Checkpoint**：每 10K 行写 `etl_checkpoint`（file_id, line_no, status）；失败重启续跑。
8. **索引异步更新**：HNSW `ANALYZE` + `REINDEX CONCURRENTLY`（per 批处理详细设计 v1.0 §6）。
9. **降级**：embed 持续故障 → 跳过 embedding，写 TU 表 + 标 `pending_embed=true`（后台补齐）。

### 6.4 关键 Rust 库 / 模式

- `sqlx` 0.8（`PgCopyIn`）/ `tokio` 1.40（流式 IO + `spawn_blocking`）/ `quick-xml` 0.36（SAX 流式）/ `csv` 1.3 / `bloomfilter` 2.0 / `tonic` 0.12
- 模式：**流式管道**（Producer-Consumer + 背压）+ **Checkpoint 续跑** + **Bulkhead**

### 6.5 复杂度

- 时间：10万 TU/分钟（普通 8 核，per 接口书 §3.9 NFR）
- 空间：Bloom 10Mbit（~1.25MB）+ 每批 ~500KB
- DB N+1 风险：⚠ **高**（必须 COPY；逐行 INSERT 视为 P1 缺陷，code review 红线）

### 6.6 测试要点

- **单测**（mockall）：5 种格式解析器 / Checkpoint 续跑 / embed 降级
- **集成**（testcontainers-rs + 真 PG）：100万 TU 灌库耗时 + COPY 字节吞吐
- **混沌**：embed 注入 50% 失败率 → 降级 + 重试不超时
- **回归**：5 种格式样例文件（存 `tests/fixtures/etl/`）

### 6.7 上下游引用

- 上游：需求 F3/F11 / worker-svc 批处理入口
- 下游：tm-service（L-1 数据源）/ HNSW 索引异步重建
- 关联 ADR：**ADR-002 §3**（Kafka 任务队列 + Debezium）/ **ADR-003 §3**（PG COPY + HNSW）/ **ADR-007 §3**（重试 + 指数退避）

---

## 7. L-6 CRDT 协同（Yjs + WebSocket 冲突合并）

### 7.1 目标

多人实时协同编辑同一段，**无需中心锁**；离线编辑后重连自动合并（CRDT 数学保证）。

### 7.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | 客户端 `local_update` (Vec<u8> Yjs binary) / `client_id` (Uuid) / `doc_id` |
| 出 | 服务端 `broadcast` (合并 update) / 持久化 `snapshot` (YDoc state) |

### 7.3 算法步骤

1. **客户端本地编辑**：Tiptap 触发 → Yjs `YEvent` → binary update（`y-protocols` 编码）。
2. **WebSocket 发送**：浏览器 → `collab-ws`（`actix-web-actors` 4.3 + `tokio-tungstenite` 0.24）。
3. **服务端 apply**（`yrs` 0.18 Rust 端 Yjs）：`doc.transact()` 应用 update；冲突由 Yjs CRDT 自动合并。
4. **广播**：`tokio::broadcast::Sender` 推给同 `doc_id` 其他客户端（排除发送者）。
5. **持久化**（per ADR-003 §2）：`collab-persistence` 消费 Kafka topic `collab.updates`，攒批写 PG：snapshot → `y_doc_snapshots`（JSONB + LZ4 压缩）；delta → `y_doc_deltas`（5 分钟级 delta 链）。
6. **离线重连**：客户端发本地 update 序号；服务端从该序号补发 missed；最终 `YDoc = merge(snapshot, deltas, client_update)`。
7. **降级**：WS 断开 → 自动 fallback HTTP 轮询（per 批处理详细设计 v1.0 §9）；yrs 不可用 → 503，前端切只读。

### 7.4 关键 Rust 库 / 模式

- `yrs` 0.18（Rust 端 Yjs 实现）/ `actix-web-actors` 4.3（WS actor）/ `tokio-tungstenite` 0.24（WS 帧）/ `sqlx` 0.8（JSONB snapshot）/ `rdkafka` 0.36
- 模式：**CRDT（Yjs）** + **简化 Event Sourcing**（snapshot + delta 链）+ **Pub/Sub**

### 7.5 复杂度

- 时间：100 并发用户单次同步 < 100ms（per 接口书 §3.8 NFR）
- 空间：snapshot ~段数×200B + delta 压缩比 ~5×
- DB N+1 风险：⚠ 低（snapshot 单条 UPSERT + delta append）

### 7.6 测试要点

- **单测**（yrs API）：100 客户端 × 1000 ops 随机合并正确性
- **集成**（testcontainers-rs + 真 WS）：3 客户端同时编辑 + 1 客户端断网 30s 后重连，最终一致
- **官方套件**：`yrs` 自带 200+ CRDT 一致性测试，必须 0 失败
- **性能**（criterion）：1000 段文档 apply update P95

### 7.7 上下游引用

- 上游：需求 F1/F2 / 前端 Tiptap
- 下游：collab-persistence（snapshot/delta 落库）/ audit 域（协同日志）
- 关联 ADR：**ADR-002 §3**（Yjs over WebSocket）/ **ADR-004**（React + Tiptap + Yjs）/ **ADR-005 §3**（多租户 namespace 隔离 doc room）

---

## 8. L-7 Outbox（事务性事件投递）

### 8.1 目标

保证"写业务表 + 发 Kafka"**原子性**：业务事务成功 → 事件必投递；回滚 → 不投递；事件可回放。

### 8.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `tx: &mut PgConnection`（业务事务）/ `aggregate_id` (Uuid, Kafka partition key) / `event_type` (&str) / `payload` (JSON) |
| 出 | `event_id` (Uuid) / Kafka message (topic `outbox.{aggregate_type}`) |

### 8.3 算法步骤

1. **业务事务内**（共享库 `common-grpc` `OutboxWriter` trait）：
   ```rust
   tx.begin().await?;
   UPDATE segments SET target_text=$1, version=version+1 WHERE id=$2;
   INSERT INTO outbox (id, aggregate_id, event_type, payload, created_at)
   VALUES ($1, $2, $3, $4, NOW());
   tx.commit().await?;
   ```
2. **Debezium CDC**（per ADR-006 §3 决策）：监听 outbox `INSERT` → 解析 WAL → 投递 Kafka topic `outbox.{aggregate_type}`。
3. **后台 Publisher**（Debezium 故障兜底）：每 1s 轮询 `SELECT * FROM outbox WHERE published_at IS NULL LIMIT 100`；`rdkafka::FutureProducer.send()`；成功后 `UPDATE published_at=NOW()`。
4. **Consumer 幂等**（per ADR-006 §4 实施）：`event_id` SETNX dedup（Redis 7，TTL 7 天）。
5. **顺序保证**：单 `aggregate_id` 按 `created_at` 顺序（Kafka partition key = aggregate_id）。
6. **降级 / 回放**：
   - Debezium 故障 → Publisher 兜底（`published_at IS NULL` 过滤避免重发）
   - 重放 → 重置 `published_at=NULL` + 时间范围，Publisher 重消费
   - outbox 膨胀 → 周期清理（已发布 + 保留 30 天，per 批处理详细设计 v1.0 §7）

### 8.4 关键 Rust 库 / 模式

- `sqlx` 0.8（事务 + outbox 写）/ `rdkafka` 0.36（producer/consumer）/ Debezium（DB 侧独立组件）
- 模式：**Transactional Outbox**（LinkedIn / Uber / Shopify 验证）+ **CDC** + **At-Least-Once + Consumer 幂等**

### 8.5 复杂度

- 时间：业务事务多 1 次 INSERT（同表同事务，~1ms）
- 空间：outbox 表 ~业务表 10% 大小
- DB N+1 风险：⚠ 无（单事务多 INSERT）

### 8.6 测试要点

- **单测**：事务回滚 → outbox 无写入；事务提交 → outbox 有写入
- **集成**（testcontainers-rs + 真 Kafka + Debezium）：1 万事件投递成功率 ≥ 99.99%
- **混沌**：Kafka 故障 30s → Publisher 兜底；Debezium 故障 → Publisher 接管
- **幂等测试**：同 event_id 重复 100 次 → 仅处理 1 次

### 8.7 上下游引用

- 上游：全部 15 服务（凡"写业务 + 发事件"场景）
- 下游：audit / notify / analytics 域
- 关联 ADR：**ADR-002 §3**（Kafka + Debezium）/ **ADR-006 §3**（Outbox 决策与实施）/ **ADR-007 §3**（Kafka consumer 重试）

---

## 9. L-8 RBAC + ABAC（矩阵 + 条件求值）

### 9.1 目标

对每个 API/gRPC 调用执行 **RBAC 矩阵 + ABAC 条件求值**，返回 Allow / Deny / RequireElevation，**P95 < 5ms**。

### 9.2 输入 / 输出

| 项 | 类型 / 备注 |
|----|------|
| 入 | `user`（含 `roles`/`tenant_id`/`device_id`）/ `resource`（`type`+`id`）/ `action`（read/write/admin/delete）/ `context`（IP/时间/请求头） |
| 出 | `decision`：`enum {Allow, Deny{reason}, RequireElevation{reason}}` / `audit_log`（自动） |

### 9.3 算法步骤

1. **Token 校验**（per ADR-008 §3）：JWT RS256 验签 + 检查 `exp`/`nbf`/`iss`/`aud`；Keycloak JWKS 缓存 1h + 自动刷新。
2. **租户隔离前置**：`user.tenant_id == resource.tenant_id`（per ADR-005 §2.3 schema 隔离），否则直接 Deny。
3. **RBAC 矩阵**（共享库 `common-auth` 适配 `casbin` 5.x）：`role × resource_type × action → permission`；未命中 → Deny。
4. **ABAC 条件求值**（自实现轻量 DSL `cats-abac`，借鉴 OPA Rego 思路）：
   - 时间窗口：`action ∈ [delete, admin] && now() ∉ business_hours → RequireElevation`
   - IP 白名单：`role=="admin" && ip ∉ whitelist → Deny`
   - 敏感标签：`resource.security_level=="restricted" && user.clearance<required → RequireElevation`
   - 设备信任：`user.device_id ∉ registered_devices → Deny`
5. **决策合成**：任一 ABAC fail → Deny；任一 RequireElevation → 返回；全过 → Allow。
6. **审计日志**：每次决策（含 Allow）写 `audit-svc`（per ADR-006 §3 消费 outbox 事件）。
7. **降级**：JWKS 拉取失败 → 本地缓存（最长 24h 容忍 Keycloak 不可达）；casbin 加载失败 → fail-closed（默认 deny-all，安全优先）。

### 9.4 关键 Rust 库 / 模式

- `jsonwebtoken` 9（JWT 验签）/ `casbin` 5.x（RBAC 矩阵）/ `regex` 1（ABAC DSL 解析）/ `redis` 0.27（JWKS + 设备注册表）
- 模式：**RBAC 矩阵**（预计算）+ **ABAC 条件 DSL**（运行时求值）+ **Cache-Aside** + **Fail-Closed**

### 9.5 复杂度

- 时间：P95 < 5ms（per 接口书 §3.10 NFR；RBAC 表 < 1ms + ABAC < 3ms + 审计异步）
- 空间：casbin 策略模型 ~1MB（in-memory 加载）
- DB N+1 风险：⚠ 低（策略全 in-memory，DB 仅租户隔离 1 次）

### 9.6 测试要点

- **单测**：7 角色 × 6 资源 × 4 动作 = 168 组合全覆盖
- **集成**（testcontainers-rs + 真 Keycloak）：JWT 过期 / 撤销 / 租户错配 / IP 拒绝
- **属性**（proptest）：随机三元组 → 决策单调（提权不导致降权）
- **性能**（criterion）：10 万次决策 P95 < 5ms
- **安全**：伪造 JWT / 重放过期 token / 跨租户 → 必须 100% 拒绝

### 9.7 上下游引用

- 上游：全部 15 服务（每个 gRPC handler 第一步鉴权）
- 下游：audit 域（决策日志）/ admin 域（策略可视化配置）
- 关联 ADR：**ADR-002 §3**（mTLS 强 + JWT 双保险）/ **ADR-005 §2.2**（RBAC+ABAC + Keycloak）/ **ADR-008 §3**（JWT 短 Access + Refresh 轮换）/ **ADR-009 §3**（mTLS 服务间认证）

---

## 10. 上下游引用汇总

| 算法 | 上游（需求/输入） | 下游（服务/库） | 关联 ADR |
|------|---------------------|-------------------|----------|
| L-1 | F3 / MatchSegment gRPC | tm-service, llm-gateway, embed svc | 001 / 002 / 003 / 007 |
| L-2 | F4 / LookupTerm gRPC | term-service, llm-gateway, 前端 | 002 / 003 / 005 |
| L-3 | F5 / TranslateSegment gRPC + L-1/L-2 hints | llm-gateway, billing 域 | 002 / 005 / 010 |
| L-4 | F7 / ScanDocument gRPC + L-3 输出 | qa-engine, 前端面板, audit | 001 / 002 |
| L-5 | F3/F11 / EtlImport task | worker-svc, tm-service, HNSW 索引 | 002 / 003 / 007 |
| L-6 | F1/F2 / Tiptap + Yjs | collab-ws, collab-persistence, audit | 002 / 004 / 005 |
| L-7 | 15 服务"写业务+发事件" | audit / notify / analytics 域 | 002 / 006 / 007 |
| L-8 | 15 服务 gRPC handler | audit 域, admin 域 | 002 / 005 / 008 / 009 |

**ADR 引用总数**：ADR-001 / 002 / 003 / 004 / 005 / 006 / 007 / 008 / 009 / 010 = **10 个**（覆盖所有落定 ADR，满足 ≥ 5）。

---

## 11. 关联文档

| 文档 | 路径 | 关联算法 |
|------|------|----------|
| CATs_类图 v1.0 | `03-详细设计\类图\` | L-1 ~ L-8 类型骨架 |
| CATs_批处理详细设计 v1.0 | `03-详细设计\批处理\` | L-5 §6 + L-7 §8 |
| CATs_模块设计书 v2.0 | `03-详细设计\模块设计\` | L-1 ~ L-8 服务职责 |
| CATs_接口设计书 v2.0 | `03-详细设计\接口设计\` | L-1 ~ L-8 gRPC/REST 契约 |
| CATs_数据库设计书 v2.0 | `03-详细设计\数据库设计\` | L-1/L-2/L-5/L-7/L-8 表 |
| CATs_SQL 设计一览 v1.0 | `03-详细设计\SQL\` | L-1 pgvector / L-2 trigram / L-5 COPY |
| CATs_DD 评审纪要 v1.1 §4.2 | `05-其他\评审记录\` | 评审项 + 决议 |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` | L-1 ~ L-8 服务边界 |
| CATs_Rust 技术选型书 v1.0 | `02-基础设计\技术选型\` | L-1 ~ L-8 crate 版本 |
| CATs_测试设计书 v1.0 §10 性能 | `04-测试\测试设计书\` | L-1/L-3/L-8 性能基线 |

---

## 12. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | L-1 TM 融合权重（0.6/0.3/0.1）A/B 调优 | 算法 + QA | M1-S2 |
| OI-2 | L-3 LLM 提示词 A/B（temperature 0.2 vs 0.3） | 算法 + BA | M1-S2 |
| OI-3 | L-4 `cats-rule-template` 示例 crate 发布 | QA + 架构 | M1-S1 |
| OI-4 | L-5 ETL 100万 TU 性能基准 | SRE + DBA | M1-S1 |
| OI-5 | L-8 ABAC 策略可视化配置（admin 域） | 架构 + 前端 | M1-S2 |
| OI-6 | L-6 yrs 0.18 升级路径跟踪 | 前端 Lead | M1-S0 |

---

**文档结束**
