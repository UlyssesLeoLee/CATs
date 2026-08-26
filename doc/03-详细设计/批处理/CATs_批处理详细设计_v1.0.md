# CATs バッチ詳細設計 v1.1

> **文档编号**：CATs-DD-049（CATs バッチ詳細設計）  
> **フェーズ**：49 バッチ詳細設計  
> **关联任务**：150 任务 #49、#32（バッチ設計）、#10-11（迁移）  
> **版本**：v1.1（基线升级：Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）  
> **创建日**：2026-08-20  
> **更新**：v1.0 → v1.1（2026-08-26 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6，见 [`CATs_技术基线_v1.0`](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md)）  
> **作者**：架构师 + Rust Lead + DBA（worker 代签 per DEC-008）

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| 架构师 | ☐ | — |
| SRE | ☐ | — |
| DBA | ☐ | — |
| QA | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-20 | 架构师 | 评审前草稿：8 类批处理 + 失败恢复 |
| **v1.1** | **2026-08-26** | **架构师 + Rust Lead + DBA（worker 代签 per DEC-008）** | **基线升级：统一引用 `CATs_技术基线_v1.0`（Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）** |

---

## 1. 目的

为 CATs 的**批处理任务**（worker-svc）提供详细设计，作为：

- 详细设计评审（DD Review）的输入
- 实施的参考
- 性能基线的依据
- 失败恢复的 SOP

---

## 2. 范围

### 2.1 包含

| 类型 | 任务 | 触发 |
|------|------|------|
| 数据 ETL | TM 导入、术语导入、用户导入 | 手动 / 调度 |
| 数据导出 | 文档导出、报告生成、备份 | 手动 / 调度 |
| 索引重建 | TM HNSW 重建、术语 trgm 重建 | 调度 / 阈值 |
| 数据聚合 | 统计、报表、监控指标 | 调度 |
| 清理任务 | 临时文件、过期会话、孤儿数据 | 调度 |
| 通知任务 | 邮件、IM 推送、提醒 | 事件驱动 |
| 数据迁移 | 历史数据、跨库同步 | 一次性 |
| 媒体处理 | 文件转换、压缩、缩略图 | 事件驱动 |

### 2.2 不包含

- 实时同步（Outbox）
- 实时通知（Kafka consumer）
- 流处理（Flink 类）

---

## 3. 总体架构

```
┌────────────────────────────────────────────────────┐
│              Task Producer                          │
│  (API / Scheduler / Event / Manual)                │
└─────────────────────┬──────────────────────────────┘
                      │ enqueue
                      ▼
        ┌──────────────────────────┐
        │   TaskQueue (Kafka)      │
        │  - task.*  topics         │
        │  - DLQ 死信队列            │
        └──────┬───────────────────┘
               │ consume
               ▼
┌──────────────────────────────────────┐
│         Worker (Stateless)            │
│  - 拉取 task                         │
│  - 执行 (根据 kind)                  │
│  - 上报状态 (心跳)                    │
│  - 写结果                            │
└──────────┬───────────────────────────┘
           │ write
           ▼
┌──────────────────────────────────────┐
│       Result Store (PG + MinIO)      │
│  - 任务状态                          │
│  - 中间产物                          │
│  - 最终产物                          │
└──────────────────────────────────────┘
```

---

## 4. 任务模型

```rust
pub struct Task {
    pub id: TaskId,
    pub kind: TaskKind,                 // EtlImport / EtlExport / IndexRebuild / ...
    pub args: serde_json::Value,        // 参数
    pub status: TaskStatus,             // Pending / Running / Completed / Failed / Cancelled
    pub priority: Priority,             // Low / Normal / High / Urgent
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub attempt: u32,                   // 重试次数
    pub max_attempts: u32,              // 默认 3
    pub timeout_secs: u32,              // 默认 3600
    pub result: Option<TaskResult>,
    pub error: Option<TaskError>,
    pub created_by: UserId,
    pub tenant_id: TenantId,
}

pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed { error: String, will_retry: bool },
    Cancelled,
}

pub enum TaskKind {
    EtlImport,           // TMX/XLIFF/CSV 导入
    EtlExport,           // 导出 TMX/XLIFF
    IndexRebuild,         // HNSW 重建
    Aggregation,         // 统计聚合
    Cleanup,              // 清理
    Notification,         // 通知发送
    DataMigration,        // 数据迁移
    MediaProcess,         // 媒体处理
}
```

---

## 5. 调度与队列

### 5.1 Kafka Topics

| Topic | 消费者 | 用途 |
|-------|--------|------|
| `task.etl.import` | EtlImportHandler | TM/term 导入 |
| `task.etl.export` | EtlExportHandler | 导出 |
| `task.index.rebuild` | IndexRebuildHandler | 索引重建 |
| `task.aggregation` | AggregationHandler | 统计聚合 |
| `task.cleanup` | CleanupHandler | 清理 |
| `task.notification` | NotificationHandler | 通知 |
| `task.data-migration` | DataMigrationHandler | 迁移 |
| `task.media` | MediaHandler | 媒体 |
| `task.dlq` | 人工/监控 | 死信队列 |

### 5.2 队列属性

| 队列 | 分区 | 副本 | 保留 |
|------|------|------|------|
| 全部 task | 12 | 3 | 7d |
| DLQ | 3 | 3 | 30d |

### 5.3 优先级

| Priority | Topic | 延迟 |
|----------|-------|------|
| Urgent | 立即入队 + 高 partition | < 1min |
| High | 标准 | < 5min |
| Normal | 标准 | < 30min |
| Low | 批量 | < 1h |

---

## 6. 8 类批处理详细逻辑

### 6.1 EtlImport（数据导入）

**用途**：从 TMX / XLIFF / CSV / Trados 导出 等格式导入 TM、术语、用户

**输入**：

```rust
pub struct EtlImportArgs {
    pub source_type: SourceType,        // Tmx / Xliff / Csv / Trados / Memoq
    pub source_url: String,             // MinIO URL 或 本地路径
    pub target: ImportTarget,           // Tm / Term / User
    pub project_id: Option<ProjectId>,
    pub field_mapping: Option<FieldMapping>,
    pub filters: ImportFilters,
    pub batch_size: usize,              // 默认 1000
    pub generate_embedding: bool,       // 默认 true
    pub skip_duplicates: bool,          // 默认 true
}
```

**算法**：

```
1. DOWNLOAD source from URL
2. STREAM PARSE
   - TMX: <tu><tuv><seg></seg></tuv></tu>
   - XLIFF: <file><unit><segment><source/target>
   - CSV: 按 mapping
3. VALIDATE
   - 语言对匹配
   - 必填字段
4. BATCH (默认 1000)
5. PER BATCH:
   a. DEDUPE (内存 + DB)
   b. EMBED (异步, 并发 10)
   c. BULK INSERT (COPY)
6. UPDATE INDEX (HNSW)
7. ANALYZE
8. EMIT Event (import.completed)
9. UPLOAD summary report
10. RETURN
```

**性能**：

- 吞吐：≥ 10 万 TU/分钟（8 核 + GPU embed）
- 失败恢复：checkpoint 每 10K 行
- 幂等：基于 (source_hash, project_id) 去重

**监控**：

- 进度：每 10K 行上报
- 失败：写 DLQ
- 内存：Bloom Filter 10M bit

### 6.2 EtlExport（数据导出）

**用途**：导出 TMX / XLIFF / TBX / CSV / Excel

**输入**：

```rust
pub struct EtlExportArgs {
    pub export_type: ExportType,        // Tmx / Xliff / Tbx / Csv / Xlsx
    pub source: ExportSource,           // project / tenant / global
    pub filter: ExportFilter,
    pub target_url: String,             // MinIO
    pub include_metadata: bool,
}
```

**算法**：

```
1. QUERY (按 filter)
2. STREAM TO MEMORY (避免 OOM)
3. FORMAT:
   a. TMX: <tu> <tuv xml:lang="src"> <seg>src</seg> </tuv> ...
   b. XLIFF 1.2: <file original="x"><body><trans-unit>...
   c. XLIFF 2.1: <file id="x"><unit id="u1"><segment>...
4. UPLOAD to MinIO
5. SIGN URL (24h)
6. NOTIFY user
7. RETURN URL
```

**性能**：

- 100 万 TU：< 5 分钟

### 6.3 IndexRebuild（索引重建）

**用途**：HNSW 重建 / trgm 重建 / VACUUM ANALYZE

**输入**：

```rust
pub struct IndexRebuildArgs {
    pub target: IndexTarget,            // TmVector / TmTrgm / TermTrgm / ...
    pub strategy: RebuildStrategy,      // Full / Incremental / Partition
    pub hnsw_params: Option<HnswParams>,// m, ef_construction
    pub parallel: usize,                // 默认 4
}
```

**算法 (Full)**：

```
1. CREATE INDEX CONCURRENTLY (新索引)
2. SET session_replication_role = replica
3. COPY existing data → new index (并行)
4. SWAP indexes (atomic)
5. DROP old index
6. UPDATE stats
```

**算法 (Incremental)**：

```
1. 读取 last_rebuild_at
2. WHERE created_at > last_rebuild_at
3. ADD to index
4. UPDATE last_rebuild_at
```

**调度**：每周日凌晨 02:00

### 6.4 Aggregation（数据聚合）

**用途**：统计 + 报表预计算 + 监控指标

**输入**：

```rust
pub struct AggregationArgs {
    pub period: AggregationPeriod,      // Hourly / Daily / Weekly / Monthly
    pub metrics: Vec<MetricKind>,       // TmHitRate / TranslationProgress / ...
    pub target: AggregationTarget,      // project / tenant / global
}
```

**算法**：

```
1. COMPUTE WINDOW (按 period)
2. PARALLEL FOR each metric:
   a. QUERY base data
   b. AGGREGATE in memory / material view
   c. UPSERT to metrics table
3. TRIGGER refresh for dashboards
```

**调度**：
- Hourly：每整点 +5min
- Daily：每日 01:00
- Weekly：每周一 02:00
- Monthly：每月 1 日 03:00

### 6.5 Cleanup（清理）

**用途**：临时文件、过期会话、孤儿数据、审计压缩

**输入**：

```rust
pub struct CleanupArgs {
    pub target: CleanupTarget,          // TempFile / ExpiredSession / Orphan / AuditOld
    pub retention_days: u32,
    pub dry_run: bool,
}
```

**算法**：

```
1. LIST candidates (按 retention)
2. IF dry_run: COUNT + log; EXIT
3. BATCH DELETE (避免长事务)
4. LOG deleted_count
5. RETURN
```

**调度**：
- TempFile：每 6h
- ExpiredSession：每 1h
- Orphan：每日 03:00
- AuditOld：每月 1 次（保留 7 年）

### 6.6 Notification（通知）

**用途**：邮件 / IM 推送 / 站内通知

**输入**：

```rust
pub struct NotificationArgs {
    pub channel: NotificationChannel,   // Email / Im / InApp
    pub template: String,
    pub recipient: Recipient,
    pub context: serde_json::Value,
    pub priority: Priority,
}
```

**算法**：

```
1. RENDER template
2. ROUTE:
   - Email: SES / SMTP
   - IM: 企业 IM webhook
   - InApp: write to notify table + push via WebSocket
3. RETRY: 3 次指数退避
4. LOG result
```

**性能**：
- 吞吐：≥ 1000 / 秒
- 失败：写 DLQ + 告警

### 6.7 DataMigration（数据迁移）

**用途**：跨库迁移、历史数据导入、跨集群同步

**输入**：

```rust
pub struct DataMigrationArgs {
    pub source: MigrationSource,        // PG / MySQL / CSV / API
    pub target: MigrationTarget,        // PG table / Kafka topic
    pub mapping: FieldMapping,
    pub batch_size: usize,
    pub validate: bool,
    pub rollback_on_error: bool,
}
```

**算法**：

```
1. PRECHECK: source/target 可达
2. SNAPSHOT: BEGIN; SAVEPOINT sp1
3. BATCH READ → TRANSFORM → WRITE
4. VALIDATE (counts, checksums)
5. IF ok: COMMIT
   ELSE: ROLLBACK TO sp1
6. UPDATE migration_log
7. EMIT event
```

**调度**：一次性 / 客户指定

### 6.8 MediaProcess（媒体处理）

**用途**：图片压缩、缩略图、PDF 转换、视频转码

**输入**：

```rust
pub struct MediaProcessArgs {
    pub source_url: String,
    pub target_format: MediaFormat,     // Jpeg / Png / Webp / Pdf / Mp4
    pub options: ProcessOptions,        // quality, size, watermark...
}
```

**算法**：

```
1. DOWNLOAD source
2. PROCESS (FFmpeg / ImageMagick / pandoc)
3. UPLOAD result
4. UPDATE media metadata
5. EMIT event
```

**性能**：
- 图片：< 5s / 张
- 视频：实时（GPU 加速）
- PDF：< 30s / 100 页

---

## 7. 失败处理

### 7.1 重试策略

| 错误类型 | 重试 | 退避 |
|---------|------|------|
| 临时网络 | 是 | 指数 1s/2s/4s |
| 数据库死锁 | 是 | 指数 |
| 资源不足 | 是 | 指数 |
| 超时 | 是 | 指数 |
| 数据错误 | 否 | — |
| 配置错误 | 否 | — |
| 权限错误 | 否 | — |

### 7.2 死信队列（DLQ）

- 触发条件：attempt >= max_attempts
- 处理：写 `task.dlq` + 告警
- 恢复：人工介入 / 修复后重投

### 7.3 取消

- 用户主动取消：标记 Cancelled + 通知 worker
- Worker 检查取消：每个 batch 头
- 优雅停止：完成当前 batch 后退出

---

## 8. 心跳与监控

### 8.1 心跳

- Worker 每 30s 上报心跳
- 包含：worker_id, current_task_id, progress, memory_cpu
- 写入：`worker_health` 表

### 8.2 超时

- 任务 > timeout_secs：标记 Failed
- Worker > 5min 无心跳：标记 Lost，重新派发

### 8.3 指标

| 指标 | 告警阈值 |
|------|----------|
| 队列堆积 | > 1000 |
| 任务失败率 | > 5% |
| DLQ 数量 | > 10/h |
| Worker 离线 | > 2 |
| 任务 P95 时长 | > 1h |

---

## 9. 扩容与调度

### 9.1 Worker 扩容

- 静态：固定 4 副本
- 动态（HPA）：CPU > 70% 扩容到 8
- 最大：12 副本
- 缩容：CPU < 30% 持续 5min

### 9.2 调度

- 紧急任务：立即入队
- 批量任务：凌晨低峰
- 资源密集：单独队列 + 资源限制

### 9.3 资源限制

| Worker 类型 | CPU | Memory | 并发任务 |
|-------------|-----|--------|----------|
| EtlImport | 2 | 4G | 1 |
| IndexRebuild | 4 | 8G | 1 |
| Aggregation | 1 | 2G | 2 |
| Notification | 0.5 | 1G | 4 |
| Media | 2 | 4G | 1 |

---

## 10. 实施示例（EtlImport）

```rust
pub struct EtlImportHandler;

#[async_trait]
impl TaskHandler for EtlImportHandler {
    async fn handle(&self, task: Task, ctx: TaskContext) -> AppResult<TaskResult> {
        let args: EtlImportArgs = serde_json::from_value(task.args.clone())?;
        
        // 1. 下载源
        let source = download(&args.source_url).await?;
        
        // 2. 解析
        let parser = ParserFactory::create(args.source_type);
        let mut stream = parser.parse(source);
        
        // 3. 批量处理
        let mut batch = Vec::with_capacity(args.batch_size);
        let mut total = 0;
        let mut imported = 0;
        let mut skipped = 0;
        
        while let Some(item) = stream.next().await? {
            batch.push(item);
            total += 1;
            
            if batch.len() >= args.batch_size {
                let r = process_batch(&batch, &args, &ctx).await?;
                imported += r.imported;
                skipped += r.skipped;
                batch.clear();
                ctx.report_progress(imported, total).await?;
            }
        }
        // 最后一 batch
        if !batch.is_empty() {
            let r = process_batch(&batch, &args, &ctx).await?;
            imported += r.imported;
            skipped += r.skipped;
        }
        
        // 4. 报告
        Ok(TaskResult::EtlImport(EtlImportResult {
            imported, skipped, total, duration_ms: ctx.elapsed().as_millis() as u64,
        }))
    }
}
```

---

## 11. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 49 バッチ詳細設計 | 本文 |
| 32 バッチ設計 | 总体（接口§3.9） |
| 96-101 移行 | §6.7 DataMigration |
| F11 数据迁移 | §6.7 |
| 9 業務シナリオ試験 | E2E 测试 |
| CATs_可热插拔部署与运维设计 | §6 HPA |

---

## 12. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_类图 v1.0 | `03-详细设计\类图\` |
| CATs_ロジック設計書 v1.0 | `03-详细设计\逻辑\` |
| CATs_接口设计书 v2.0 §3.9 | `03-详细设计\接口设计\` |
| CATs_数据库设计书 v2.0 | `03-详细设计\数据库设计\` |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` |
| CATs_可热插拔部署与运维设计 v1.0 | `02-基础设计\架构设计\` |
| CATs_迁移要件定义书 v1.0 | `05-其他\迁移\` |

---

## 13. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | Worker 资源规模（基线 + 高峰） | SRE | M1-S0 |
| OI-2 | Kafka 分区与副本策略 | SRE | M1-S0 |
| OI-3 | DLQ 监控与告警 | SRE | M1-S0 |
| OI-4 | HPA 调优 | SRE | M2-S5 |
| OI-5 | 各 Handler 单元测试 | 开发者 | M1-S1 |
| OI-6 | 死信恢复 SOP 文档化 | SRE | M1-S0 |

---

**文档结束**
