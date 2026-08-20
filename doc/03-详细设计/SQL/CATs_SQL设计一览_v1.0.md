# CATs SQL 設計一覧 v1.0

> **文档编号**：CATs-DD-048（CATs SQL 设计）  
> **フェーズ**：48 SQL 設計  
> **关联任务**：150 任务 #48、#47（DB 詳細設計）  
> **版本**：v1.0（评审会前草稿）  
> **创建日**：2026-08-20  
> **作者**：DBA + 架构师

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| DBA | ☐ | — |
| 架构师 | ☐ | — |
| 开发者 | ☐ | — |
| QA | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-20** | **DBA** | **评审前草稿：基于 DB v2.0 §4 + DDL** |

---

## 1. 目的

汇总 CATs 系统的**关键 SQL 语句**设计，作为：

- 详细评审（DD Review）的输入
- 性能调优的基线
- 性能测试（PT, QA-041）的依据
- 实施人员的参考

---

## 2. 范围

### 2.1 包含

| 类别 | 数量级 | 优先级 |
|------|--------|--------|
| 关键查询 | 50+ | P0 |
| 关键写入 | 30+ | P0 |
| 关键更新 | 20+ | P0 |
| 分析查询 | 30+ | P1 |
| 维护脚本 | 20+ | P1 |

### 2.2 不包含

- 自动生成的 CRUD（ORM 处理）
- 工具/脚本类 SQL（migration 工具）
- 一次性 ETL

---

## 3. 命名与编码规范

### 3.1 命名

| 元素 | 规范 | 示例 |
|------|------|------|
| 表名 | snake_case + 复数 | `tm_segments` |
| 字段 | snake_case | `created_at` |
| 主键 | `id` (BIGSERIAL/UUID) | `id` |
| 外键 | `<单数表名>_id` | `project_id` |
| 索引 | `idx_<表>_<字段>` | `idx_tm_segments_project_id` |
| 唯一约束 | `uk_<表>_<字段>` | `uk_users_email` |
| 检查约束 | `ck_<表>_<字段>` | `ck_users_age` |
| 触发器 | `trg_<表>_<动作>` | `trg_audit_log_insert` |

### 3.2 风格

- 关键字大写：`SELECT`, `FROM`, `WHERE`
- 表/字段小写 + snake_case
- 缩进 2 空格
- 显式 JOIN（不用 `,` 连接）
- 注释：表与字段必加

### 3.3 必备

- 所有表：`created_at`, `updated_at`, `created_by`, `updated_by`
- 软删除：`deleted_at`（TIMESTAMPTZ NULL）
- 主键策略：UUID v7（高并发分布式友好）

---

## 4. 关键查询 SQL

### 4.1 TM 模糊匹配（pgvector）

```sql
-- 任务：给定源文，匹配 TM 中前 10 个候选（>85% 相似度）
-- 表：tm_segments
-- 索引：HNSW (embedding vector_cosine_ops)
-- 期望：P95 < 200ms

SELECT
  ts.id,
  ts.source_text,
  ts.target_text,
  ts.tu_id,
  ts.project_id,
  1 - (ts.embedding <=> $1::vector) AS similarity
FROM tm_segments ts
WHERE
  ts.project_id = $2
  AND ts.deleted_at IS NULL
  AND 1 - (ts.embedding <=> $1::vector) >= 0.85
ORDER BY ts.embedding <=> $1::vector
LIMIT 10;
```

参数：
- `$1`：源文 embedding (vector(1024))
- `$2`：项目 ID (uuid)

**索引**：
```sql
CREATE INDEX idx_tm_segments_embedding
  ON tm_segments
  USING hnsw (embedding vector_cosine_ops)
  WHERE deleted_at IS NULL;
```

### 4.2 术语查询

```sql
-- 任务：给定源词，查询术语
-- 表：terms
-- 索引：B-tree (source_text, source_lang)

SELECT
  t.id,
  t.source_text,
  t.target_text,
  t.definition,
  t.context,
  t.domain,
  t.score
FROM terms t
WHERE
  t.source_text ILIKE $1
  AND t.source_lang = $2
  AND t.deleted_at IS NULL
ORDER BY t.score DESC, t.updated_at DESC
LIMIT 50;
```

### 4.3 项目列表（分页）

```sql
-- 任务：项目列表分页
-- 表：projects
-- 索引：B-tree (tenant_id, status, updated_at DESC)

SELECT
  p.id, p.name, p.client_id, p.status,
  p.start_date, p.end_date,
  p.total_words, p.translated_words, p.reviewed_words,
  p.pm_id, p.updated_at
FROM projects p
WHERE
  p.tenant_id = $1
  AND p.status = ANY($2)
  AND p.deleted_at IS NULL
  AND ($3::text IS NULL OR p.name ILIKE '%' || $3 || '%')
ORDER BY p.updated_at DESC
LIMIT $4 OFFSET $5;
```

### 4.4 翻译进度统计

```sql
-- 任务：项目翻译进度
-- 表：segments（按 project_id 聚合）

SELECT
  s.project_id,
  COUNT(*) AS total_segments,
  COUNT(*) FILTER (WHERE s.status = 'translated') AS translated,
  COUNT(*) FILTER (WHERE s.status = 'reviewed') AS reviewed,
  COUNT(*) FILTER (WHERE s.status = 'final') AS final,
  COUNT(*) FILTER (WHERE s.status = 'pending') AS pending
FROM segments s
WHERE s.deleted_at IS NULL
GROUP BY s.project_id;
```

### 4.5 用户活跃度

```sql
-- 任务：用户日活统计
-- 表：user_activity

SELECT
  ua.user_id,
  u.email,
  u.display_name,
  COUNT(*) AS action_count,
  SUM(ua.duration_ms) AS total_duration_ms,
  MAX(ua.created_at) AS last_active
FROM user_activity ua
JOIN users u ON u.id = ua.user_id
WHERE
  ua.created_at >= $1
  AND ua.created_at < $2
  AND u.tenant_id = $3
GROUP BY ua.user_id, u.email, u.display_name
ORDER BY action_count DESC
LIMIT 100;
```

---

## 5. 关键写入 SQL

### 5.1 翻译单元创建

```sql
-- 任务：创建 TU（Translation Unit）+ 段
-- 表：tu（翻译单元）、segments（段）

BEGIN;

INSERT INTO tu (id, project_id, source_lang, target_lang, domain, created_at, updated_at, created_by)
VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), $6)
RETURNING id;

INSERT INTO segments (id, tu_id, project_id, segment_index, source_text, status, created_at, updated_at, created_by)
VALUES
  ($7, $1, $2, 1, $8, 'pending', NOW(), NOW(), $6),
  ($9, $1, $2, 2, $10, 'pending', NOW(), NOW(), $6),
  ...
;

COMMIT;
```

### 5.2 翻译保存（Outbox 模式）

```sql
-- 任务：保存翻译，事务性发布事件
-- 表：segments、outbox

BEGIN;

UPDATE segments
SET
  target_text = $1,
  status = 'translated',
  translated_by = $2,
  translated_at = NOW(),
  updated_at = NOW(),
  updated_by = $2
WHERE id = $3 AND deleted_at IS NULL
RETURNING id;

INSERT INTO outbox (id, aggregate_type, aggregate_id, event_type, payload, created_at)
VALUES (
  gen_random_uuid(),
  'segment',
  $3,
  'segment.translated',
  jsonb_build_object(
    'segment_id', $3,
    'translator_id', $2,
    'translated_at', NOW()
  ),
  NOW()
);

COMMIT;
```

### 5.3 批量导入（COPY）

```sql
-- 任务：从 ETL 批量导入 TM
-- 表：tm_segments
-- 工具：COPY FROM STDIN BINARY

COPY tm_segments (
  id, tu_id, project_id, source_text, target_text,
  source_lang, target_lang, domain, embedding, created_at, created_by
)
FROM STDIN BINARY;
```

### 5.4 审计日志写入

```sql
-- 任务：审计事件写入
-- 表：audit_log（append-only）
-- 索引：B-tree (created_at DESC), GIN (payload jsonb_path_ops)

INSERT INTO audit_log (
  id, actor_id, actor_role, action, resource_type, resource_id,
  result, ip, user_agent, payload, trace_id, created_at
)
VALUES (
  $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW()
);
```

---

## 6. 关键更新 SQL

### 6.1 软删除

```sql
-- 任务：软删除（不物理删除）
-- 表：通用

UPDATE tm_segments
SET deleted_at = NOW(), updated_at = NOW(), updated_by = $2
WHERE id = $1 AND deleted_at IS NULL
RETURNING id;
```

### 6.2 版本快照

```sql
-- 任务：保存版本快照
-- 表：versions

INSERT INTO versions (
  id, resource_type, resource_id, version_no, snapshot, created_by, created_at
)
VALUES (
  $1, $2, $3,
  COALESCE(
    (SELECT MAX(version_no) + 1 FROM versions
     WHERE resource_type = $2 AND resource_id = $3),
    1
  ),
  $4, $5, NOW()
);
```

### 6.3 角色授予

```sql
-- 任务：授予用户角色
-- 表：user_roles（多对多）

INSERT INTO user_roles (id, user_id, role_id, project_id, granted_by, granted_at, expires_at)
VALUES (gen_random_uuid(), $1, $2, $3, $4, NOW(), $5)
ON CONFLICT (user_id, role_id, project_id)
DO UPDATE SET
  granted_by = EXCLUDED.granted_by,
  granted_at = EXCLUDED.granted_at,
  expires_at = EXCLUDED.expires_at;
```

---

## 7. 分析查询 SQL

### 7.1 TM 命中率（按项目）

```sql
-- 任务：TM 命中率
-- 表：tm_queries（查询日志）

SELECT
  tq.project_id,
  COUNT(*) AS total_queries,
  COUNT(*) FILTER (WHERE tq.match_type = 'exact') AS exact_matches,
  COUNT(*) FILTER (WHERE tq.match_type = 'fuzzy_95_99') AS fuzzy_95_99,
  COUNT(*) FILTER (WHERE tq.match_type = 'fuzzy_85_94') AS fuzzy_85_94,
  COUNT(*) FILTER (WHERE tq.match_type = 'no_match') AS no_match,
  1.0 * COUNT(*) FILTER (WHERE tq.match_type IN ('exact', 'fuzzy_95_99')) / COUNT(*) AS hit_rate
FROM tm_queries tq
WHERE tq.created_at >= $1 AND tq.created_at < $2
GROUP BY tq.project_id;
```

### 7.2 LLM 使用统计

```sql
-- 任务：LLM 用量
-- 表：llm_usage

SELECT
  lu.project_id,
  lu.model,
  COUNT(*) AS call_count,
  SUM(lu.input_tokens) AS total_input_tokens,
  SUM(lu.output_tokens) AS total_output_tokens,
  AVG(lu.latency_ms) AS avg_latency_ms
FROM llm_usage lu
WHERE lu.created_at >= $1 AND lu.created_at < $2
GROUP BY lu.project_id, lu.model;
```

### 7.3 译者效率

```sql
-- 任务：译者效率（每日字数）
-- 表：segments

SELECT
  s.translated_by,
  u.display_name,
  DATE_TRUNC('day', s.translated_at) AS day,
  COUNT(*) FILTER (WHERE s.status IN ('translated', 'reviewed', 'final')) AS segments_done,
  SUM(LENGTH(s.source_text)) AS chars_done
FROM segments s
JOIN users u ON u.id = s.translated_by
WHERE s.translated_at >= $1 AND s.translated_at < $2
GROUP BY s.translated_by, u.display_name, DATE_TRUNC('day', s.translated_at)
ORDER BY day DESC, segments_done DESC;
```

---

## 8. 性能关键 SQL（QA-041）

### 8.1 PG + pgvector 性能基线

| 查询 | 期望 P95 | 当前实现 | 优化方向 |
|------|----------|----------|----------|
| TM 模糊匹配（10万） | < 200ms | ⏳ 测试中 | HNSW 参数 + 分桶 |
| TM 模糊匹配（100万） | < 500ms | ⏳ | 分桶 + 分区 |
| 术语查询 | < 50ms | ⏳ | B-tree + 缓存 |
| 项目列表 | < 100ms | ⏳ | 索引 + 分页 |
| 翻译进度 | < 500ms | ⏳ | 物化视图 |
| 审计查询 | < 1s | ⏳ | 分区 + BRIN |

### 8.2 慢查询阈值

- 任何 P95 > 1s 的查询必须优化
- 任何全表扫描必须优化
- 任何 > 100ms 的写入必须排查

### 8.3 EXPLAIN 检查清单

- [ ] 是否使用索引
- [ ] 是否全表扫描
- [ ] 是否 Nested Loop 异常
- [ ] 是否 Hash Join 合理
- [ ] 是否 Sort 在内存
- [ ] 是否 Buffer Hit > 99%

---

## 9. 安全关键 SQL

### 9.1 RLS（行级安全）

```sql
-- 任务：多租户隔离
-- 表：projects, segments, tm_segments, terms

ALTER TABLE projects ENABLE ROW LEVEL SECURITY;

CREATE POLICY projects_tenant_isolation ON projects
  USING (tenant_id = current_setting('app.current_tenant')::uuid);

ALTER TABLE projects FORCE ROW LEVEL SECURITY;
```

### 9.2 SQL 注入防护

- 全部使用参数化查询（prepared statement）
- 禁止字符串拼接
- ORM 强制使用
- SAST 扫描

---

## 10. SQL 性能基线（QA-041）

### 10.1 测试用例

| 场景 | 数据量 | 期望 |
|------|--------|------|
| 100 用户并发查询 | 100 万 TM | P95 < 500ms |
| 1000 段/秒写入 | — | P95 < 100ms |
| 100 并发 ETL 导入 | 1000 万/批 | 吞吐 > 10万/分钟 |

### 10.2 测试方法

- 工具：pgbench / sysbench / k6
- 数据：生产脱敏副本
- 监控：pg_stat_statements + EXPLAIN
- 报告：性能测试报告

---

## 11. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 48 SQL 設計 | 本文 |
| 47 DB 詳細設計 | 表结构 |
| 30/31 DB 基本設計 | 库/ER |
| 50 エラー処理設計 | SQL 错误处理 |
| QA-041 | PG + pgvector 性能基线 |
| QA-011 | TM 索引策略 |

---

## 12. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_数据库设计书 v2.0 §4 | `03-详细设计\数据库设计\` |
| CATs_接口设计书 v2.0 | `03-详细设计\接口设计\` |
| CATs_模块设计书 v2.0 | `03-详细设计\模块设计\` |
| CATs_测试设计书 v1.0 §10 性能 | `04-测试\测试设计书\` |
| CATs_安全要件定义书 v1.0 §9 SQL 注入 | `05-其他\安全\` |
| CATs_实施前 QA 登记册 v1.0 QA-041 | `05-其他\` |

---

## 13. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | QA-041 性能基线测试 | DBA + QA | M1-S1 末 |
| OI-2 | QA-011 TM 索引选型 | 架构 + DBA | 评审会 D+2 |
| OI-3 | RLS 策略完善 | DBA | M1-S0 |
| OI-4 | 慢查询自动告警 | SRE + DBA | M1-S1 |
| OI-5 | SQL 代码审查清单 | DBA | M1-S0 |

---

**文档结束**
