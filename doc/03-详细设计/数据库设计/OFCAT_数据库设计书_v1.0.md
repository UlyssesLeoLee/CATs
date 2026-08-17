# OFCAT 数据库设计书

**系统名称:** OFCAT — AI 增强型 CAT 浏览器工作台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | OFCAT-DD-D-001 |
| 文档名 | 数据库设计书（详细设计 / DDL·索引·迁移） |
| 版本 | 第 1.0 版（草稿） |
| 创建日 | 2026-06-25 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [基础设计书 v1.0](../../02-基础设计/架构设计/OFCAT_基础设计书_v1.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | 初版。物理模型、DDL、索引、向量表、迁移与合并策略 |

---

## 1. 总则
- DBMS：SQLite 3（本地权威副本，C4）；向量经 `sqlite-vec` 扩展。
- PRAGMA：`journal_mode=WAL`、`foreign_keys=ON`、`busy_timeout=5000`。
- 字符：UTF-8；时间存 ISO-8601 UTC 文本。
- 版本管理：`schema_migrations` 表 + 顺序迁移脚本（§6）。

---

## 2. 表一览

| 表名 | 说明 | 主键 |
|---|---|---|
| `projects` | 项目/语言对/领域/敏感策略 | id |
| `terms` | 术语库条目 | id |
| `translation_memory` | TM 句段对 | id |
| `tm_vectors` | TM 语义向量（vec0 虚拟表） | tm_id |
| `history` | 变更/操作审计 | id |
| `import_jobs` | 导入任务与报告 | id(uuid) |
| `sync_state` | 同步游标 | entity |
| `settings` | 键值设置 | key |
| `schema_migrations` | 迁移版本 | version |

---

## 3. DDL

```sql
-- 3.1 projects
CREATE TABLE projects (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  name         TEXT    NOT NULL,
  source_lang  TEXT,
  target_lang  TEXT,
  domain       TEXT    DEFAULT '',
  sensitivity  TEXT    NOT NULL DEFAULT 'normal',   -- normal | sensitive
  created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  updated_at   TEXT
);

-- 3.2 terms
CREATE TABLE terms (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id      INTEGER REFERENCES projects(id) ON DELETE CASCADE,  -- NULL=全局
  source_lang     TEXT    NOT NULL,
  target_lang     TEXT    NOT NULL,
  domain          TEXT    NOT NULL DEFAULT '',
  source_term     TEXT    NOT NULL,
  source_term_norm TEXT   NOT NULL,                 -- 规范化匹配键
  target_term     TEXT    NOT NULL,
  forbidden       TEXT    NOT NULL DEFAULT '[]',     -- JSON array
  case_sensitive  INTEGER NOT NULL DEFAULT 0,
  match_mode      TEXT    NOT NULL DEFAULT 'word',   -- word | substring | regex
  note            TEXT,
  status          TEXT    NOT NULL DEFAULT 'active', -- active | disabled
  created_at      TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  updated_at      TEXT
);

-- 3.3 translation_memory
CREATE TABLE translation_memory (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id   INTEGER REFERENCES projects(id) ON DELETE SET NULL,
  source_lang  TEXT    NOT NULL,
  target_lang  TEXT    NOT NULL,
  domain       TEXT    NOT NULL DEFAULT '',
  source_text  TEXT    NOT NULL,
  source_norm  TEXT    NOT NULL,                     -- 规范化文本（模糊基准）
  source_hash  TEXT    NOT NULL,                     -- sha1(source_norm)（L0）
  source_len   INTEGER NOT NULL,                     -- 长度预过滤
  target_text  TEXT    NOT NULL,
  context      TEXT,
  origin       TEXT    NOT NULL DEFAULT 'human',     -- human | import | mt
  quality      INTEGER NOT NULL DEFAULT 0,
  usage_count  INTEGER NOT NULL DEFAULT 0,
  created_at   TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  updated_at   TEXT
);

-- 3.4 tm_vectors（sqlite-vec 虚拟表，bge-m3 → 1024 维）
CREATE VIRTUAL TABLE tm_vectors USING vec0(
  tm_id     INTEGER PRIMARY KEY,
  embedding FLOAT[1024]
);

-- 3.5 history
CREATE TABLE history (
  id           INTEGER PRIMARY KEY AUTOINCREMENT,
  entity_type  TEXT    NOT NULL,        -- tm | term
  entity_id    INTEGER,
  action       TEXT    NOT NULL,        -- create | update | delete | import | sync
  before       TEXT,                    -- JSON
  after        TEXT,                    -- JSON
  actor        TEXT,
  ts           TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);

-- 3.6 import_jobs
CREATE TABLE import_jobs (
  id          TEXT    PRIMARY KEY,      -- uuid
  status      TEXT    NOT NULL DEFAULT 'pending',  -- pending|running|done|failed
  total       INTEGER DEFAULT 0,
  succeeded   INTEGER DEFAULT 0,
  duplicated  INTEGER DEFAULT 0,
  failed      INTEGER DEFAULT 0,
  mapping     TEXT,                     -- JSON
  report      TEXT,                     -- JSON（错误行明细）
  created_at  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now')),
  finished_at TEXT
);

-- 3.7 sync_state
CREATE TABLE sync_state (
  entity       TEXT PRIMARY KEY,        -- terms | tm
  last_pull_at TEXT,
  last_push_at TEXT,
  cursor       TEXT
);

-- 3.8 settings
CREATE TABLE settings (
  key        TEXT PRIMARY KEY,
  value      TEXT,                      -- JSON
  updated_at TEXT
);

-- 3.9 schema_migrations
CREATE TABLE schema_migrations (
  version    INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ','now'))
);
```

---

## 4. 索引设计

```sql
-- L0 精确匹配：O(1)
CREATE INDEX idx_tm_hash   ON translation_memory(source_lang, target_lang, source_hash);
-- L1 候选预过滤：语言对+领域+长度
CREATE INDEX idx_tm_filter ON translation_memory(source_lang, target_lang, domain, source_len);
-- TM 去重唯一键
CREATE UNIQUE INDEX uq_tm  ON translation_memory(source_lang, target_lang, domain, source_hash, IFNULL(project_id,0));
-- 术语命中
CREATE INDEX idx_term_lookup ON terms(source_lang, target_lang, domain, status);
CREATE UNIQUE INDEX uq_term  ON terms(IFNULL(project_id,0), source_lang, target_lang, domain, source_term_norm);
-- 历史检索
CREATE INDEX idx_hist ON history(entity_type, entity_id, ts);
```

| 索引 | 服务于 | 说明 |
|---|---|---|
| `idx_tm_hash` | M-02 L0 | 精确命中 |
| `idx_tm_filter` | M-02 L1 | 候选集缩小到百量级 |
| `uq_tm` | F9 回存 | UPSERT 去重 |
| `idx_term_lookup` / `uq_term` | M-03 | 术语载入与唯一性 |
| `idx_hist` | 审计 | 追溯 |

---

## 5. 规范化与键生成规则
- `source_norm = collapse_ws(strip(NFKC(text)))`；拉丁语种且 `case_insensitive=1` 时另用小写键参与匹配（CJK 不折叠大小写）。
- `source_hash = lower(hex(sha1(source_norm)))`，用于 L0 与去重。
- `source_len = char_length(source_norm)`，用于 L1 长度窗口预过滤。
- 术语 `source_term_norm` 同上规则；`match_mode=regex` 时不规范化、原样存储。

---

## 6. 迁移策略
- 每个变更对应一支 `NNN_description.sql`，事务内执行，成功后 `INSERT INTO schema_migrations(version)`。
- 启动时比对 `MAX(version)` 与代码内嵌迁移集，按序补齐。
- 向量维度变更（更换嵌入模型）视为破坏性迁移：重建 `tm_vectors` 并后台重嵌入。

```
migrations/
  001_init.sql          -- 本文档 §3/§4 全部 DDL
  002_xxx.sql           -- 后续变更
```

---

## 7. 同步合并策略（C4，承接接口 API-10）

| 实体 | 拉取冲突 | 推送冲突 |
|---|---|---|
| `terms` | **以中心为准**（管理者维护权威术语），本地差异记 history | 仅管理者可推送，普通客户端只读拉取 |
| `translation_memory` | **后写优先**（`updated_at` 新者胜），但保留被覆盖者到 history；同 `source_hash` 不同译文 → 双方都留，标记 `conflict` 待人工 | 同上 |

- 同步为异步旁路（NFR-01/02），断网不影响本地主流程。

---

## 8. 容量与维护
- 假设（O1）：TM ≤ 50 万、术语 ≤ 10 万；该量级 SQLite + 上述索引可满足分层延迟。
- 备份：定期复制 `ofcat.db`（WAL checkpoint 后）；导出支持 TMX/CSV（对接 F11 反向）。
- 清理：`history` 可按保留期归档；`import_jobs` 完成 N 天后清理。

---

## 9. 典型查询（参考）
```sql
-- L0 精确
SELECT target_text FROM translation_memory
WHERE source_lang=? AND target_lang=? AND source_hash=?
ORDER BY quality DESC, usage_count DESC LIMIT 1;

-- L1 候选预过滤（再交 RapidFuzz 计分）
SELECT id, source_norm, target_text, origin FROM translation_memory
WHERE source_lang=? AND target_lang=? AND (domain=? OR ?='')
  AND source_len BETWEEN ? AND ?;

-- 术语载入
SELECT source_term, source_term_norm, target_term, forbidden, match_mode
FROM terms
WHERE source_lang=? AND target_lang=? AND status='active'
  AND (domain=? OR domain='') AND (project_id=? OR project_id IS NULL);
```
