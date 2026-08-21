# CATs 数据库设计书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-DD-DB-002 |
| 文档名 | 数据库设计书（PostgreSQL 逻辑库 DDL / 索引 / 分区 / CDC 对应关系） |
| 版本 | 第 2.0 版（草稿） |
| 创建日 | 2026-08-18 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [CATs 微服务架构设计书 v1.0](../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)（§5 数据库划分、§7 Outbox+CDC）、[CATs 技术选型书 v2.0](../../02-基础设计/技术选型/CATs_技术选型书_v2.0.md)（ADR-18 主存储、ADR-21 CDC、ADR-30 向量检索）、[OFCAT 数据库设计书 v1.0（历史/旧架构参考，格式沿用）](./OFCAT_数据库设计书_v1.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | （OFCAT）SQLite 单机库表结构，见历史文档 |
| 2.0 | 2026-08-18 | 架构师 | 全面重做：8 个 PostgreSQL 逻辑库完整 DDL、索引、分区策略、Debezium 对应关系，承接《CATs 微服务架构设计书 v1.0》§5、§7 |

### 审批栏

| 角色 | 姓名 | 审批日 | 签字 |
|---|---|---|---|
| 起草 | | | |
| 评审 | | | |
| 批准 | | | |

---

## 0. 阅读指南

本书是《CATs 微服务架构设计书 v1.0》§21 承诺的数据库详细设计文档。所有 DDL 均以 **PostgreSQL 最新 stable** 语法编写（2026-08-20 决议 QA-026），遵循架构设计书 §5.2「每逻辑库独立账号、迁移与运行时账号分离」原则。gRPC/REST 接口契约见《CATs 接口设计书 v2.0》，各服务内部模块与仓储层设计见《CATs 模块设计书 v2.0》。

---

## 1. 逻辑库清单与与基础设计书的一致性说明

| 逻辑库 | 归属服务 | 是否为基础设计书 §5.1 已列库 | 说明 |
|---|---|---|---|
| `auth_db` | auth-service | 是 | 与架构设计书一致 |
| `user_db` | user-service | 是 | 与架构设计书一致 |
| `project_db` | project-service, translation-core | 是 | 与架构设计书一致，含 TM/术语/pgvector |
| `task_db` | task-service（媒体处理服务经其 API 间接读写） | 是 | 与架构设计书一致 |
| `report_db` | report-service | 是 | 与架构设计书一致 |
| `audit_db` | audit-service | 是 | 与架构设计书一致 |
| `notification_db` | notification-service | 是（架构设计书 §4.1 表中已列，§5.1 表未单列，本书补齐 DDL） | 与架构设计书一致，无需新增理由 |
| `file_db` | file-service | 是（架构设计书 §4.1 已列，任务要求中提及"需新增需说明理由"——**经核对，`file_db` 已在架构设计书 §4.1 服务清单中明确列出，非本书新增**） | 与架构设计书一致 |

> **关于 `media_job_db`**：架构设计书 §4.1 明确规定 asr/ocr/subtitle/office-converter/render-writer 五个媒体处理服务"**统一约定：无独立数据库**，仅作为无状态处理器…避免碎片化"（原文见架构设计书 §4.1 末尾说明）。因此本书**不新增 `media_job_db`**，ASR 转写结果（`asr_transcripts`）、字幕分段（`subtitle_segments`）等媒体处理派生数据统一归口到 `task_db`（作为 `task_media_items` 的关联明细表），由 task-service 提供内部 API 供媒体处理服务写入/查询，不直连数据库，符合架构设计书 §5.1「task_db」行的归属说明（"task-service, ingestion/asr/ocr/subtitle/office/render-service（经 task-service API，不直连）"）。这样任务要求中提到的 `asr_transcripts`/`subtitle_segments`/`media_assets` 表均落在 `task_db` 内，详见 §4。

---

## 2. 账号与权限矩阵

| 逻辑库 | 运行时角色 | 迁移角色 | 只读角色（report-service 等跨库只读场景） |
|---|---|---|---|
| `auth_db` | `svc_auth`（CONNECT+CRUD） | `migrator_auth`（DDL） | — |
| `user_db` | `svc_user` | `migrator_user` | — |
| `project_db` | `svc_project`（project-service）、`svc_translation`（translation-core，仅 TM/术语相关表） | `migrator_project` | — |
| `task_db` | `svc_task` | `migrator_task` | `svc_report_ro`（仅 `tasks`/`task_media_items` 只读，供 report-service 用量统计） |
| `file_db` | `svc_file` | `migrator_file` | — |
| `notification_db` | `svc_notify` | `migrator_notify` | — |
| `report_db` | `svc_report` | `migrator_report` | — |
| `audit_db` | `svc_audit` | `migrator_audit` | — |

运行时角色一律不具备 DDL 权限（架构设计书 §5.2），密码经 K8s Secret + SealedSecrets 注入（架构设计书 §14）。

---

## 3. 数据库迁移版本管理工具选型

| 服务实现语言 | 迁移工具 | 理由 |
|---|---|---|
| Rust 服务（render-writer-service 高性能路径、部分核心服务如选用 Rust 实现） | `sqlx-migrate`（sqlx 自带迁移子命令） | 与 Rust 生态原生集成，迁移文件为纯 SQL，无需额外 DSL 学习成本，CI 中 `sqlx migrate run --dry-run` 校验 |
| Python 服务（translation-core、asr/ocr-service 若涉及少量本地状态、worker-service） | Alembic | SQLAlchemy 生态标准迁移工具，支持自动生成迁移脚本 diff |
| Node.js/TypeScript 服务（Next.js BFF 相关的少量后台表，若有） | Prisma Migrate | 与 TypeScript 类型生成天然集成 |
| 统一原则 | 与技术选型书/架构设计书 §15.3 保持一致 | 架构设计书 §15.3 已明确"Flyway 或 Alembic（视各服务实现语言选择，Python 服务用 Alembic，其余可选 Flyway 统一）"；本书在此基础上**补充**：Rust 服务优先 `sqlx-migrate`（比引入 Java 生态的 Flyway 更贴合 Rust 技术栈，减少 JVM 依赖），其余非 Python 服务仍以 Flyway 为兜底统一选项，未偏离架构设计书结论，仅做语言维度的落地细化 |
| 迁移策略 | Expand-Contract 模式，作为 Argo CD `PreSync` Hook Job 执行 | 与架构设计书 §15.3 完全一致 |

所有迁移脚本纳入对应服务代码仓库的 `migrations/` 目录，版本号单调递增并与 Git commit 绑定可追溯。

---

## 4. 各逻辑库核心表 DDL

### 4.1 `auth_db`

```sql
CREATE TABLE users_credential (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email           CITEXT NOT NULL UNIQUE,
    password_hash   TEXT,                         -- Argon2id，OIDC-only 用户可为 NULL
    org_id          UUID NOT NULL,                 -- 逻辑外键，指向 user_db.orgs，跨库不建物理 FK
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','locked','disabled')),
    mfa_enabled     BOOLEAN NOT NULL DEFAULT false,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_users_credential_org_id ON users_credential (org_id);

CREATE TABLE sessions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL REFERENCES users_credential(id) ON DELETE CASCADE,
    refresh_token_hash TEXT NOT NULL UNIQUE,
    issued_at       TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at      TIMESTAMPTZ NOT NULL,
    revoked_at      TIMESTAMPTZ,
    client_kind     TEXT NOT NULL CHECK (client_kind IN ('tauri','web'))
);
CREATE INDEX idx_sessions_user_id ON sessions (user_id);
CREATE INDEX idx_sessions_expires_at ON sessions (expires_at) WHERE revoked_at IS NULL;

CREATE TABLE roles (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code        TEXT NOT NULL UNIQUE,              -- 'org_admin' / 'platform_admin' / 'translator' ...
    description TEXT
);

CREATE TABLE role_bindings (
    id          BIGSERIAL PRIMARY KEY,
    user_id     UUID NOT NULL REFERENCES users_credential(id) ON DELETE CASCADE,
    role_id     UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
    org_id      UUID NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (user_id, role_id, org_id)
);
CREATE INDEX idx_role_bindings_user_id ON role_bindings (user_id);
```

### 4.2 `user_db`

```sql
CREATE TABLE orgs (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name        TEXT NOT NULL,
    plan        TEXT NOT NULL DEFAULT 'free' CHECK (plan IN ('free','team','enterprise')),
    seats_limit INT NOT NULL DEFAULT 5,
    monthly_media_minutes_quota INT NOT NULL DEFAULT 60,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE users_profile (
    id          UUID PRIMARY KEY,                  -- 与 auth_db.users_credential.id 相同值（应用层保证一致，非物理 FK）
    org_id      UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    avatar_url  TEXT,
    locale      TEXT NOT NULL DEFAULT 'zh-CN',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_users_profile_org_id ON users_profile (org_id);

CREATE TABLE org_members (
    org_id      UUID NOT NULL REFERENCES orgs(id) ON DELETE CASCADE,
    user_id     UUID NOT NULL,
    role        TEXT NOT NULL DEFAULT 'member',
    invited_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    joined_at   TIMESTAMPTZ,
    PRIMARY KEY (org_id, user_id)
);

CREATE TABLE subscriptions (
    org_id      UUID PRIMARY KEY REFERENCES orgs(id) ON DELETE CASCADE,
    plan        TEXT NOT NULL,
    seats_used  INT NOT NULL DEFAULT 0,
    monthly_media_minutes_used NUMERIC(10,2) NOT NULL DEFAULT 0,
    renews_at   TIMESTAMPTZ NOT NULL,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- user_db 的 Outbox 表（架构设计书 §7.2 模式，各库均需一份）
CREATE TABLE outbox_event (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_user_db_outbox_created_at ON outbox_event (created_at);
```

### 4.3 `project_db`（含 TM / 术语 / pgvector，与 translation-core 共享）

```sql
CREATE EXTENSION IF NOT EXISTS vector;   -- pgvector，技术选型 ADR-30

CREATE TABLE projects (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL,
    name            TEXT NOT NULL,
    source_lang     TEXT NOT NULL,       -- BCP-47
    target_lang     TEXT NOT NULL,
    domain          TEXT NOT NULL DEFAULT '',
    force_local_model BOOLEAN NOT NULL DEFAULT false,   -- 敏感策略：强制路由本地模型
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','archived')),
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_projects_org_id ON projects (org_id);

CREATE TABLE terms (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_term     TEXT NOT NULL,
    target_term     TEXT NOT NULL,
    domain_tag      TEXT NOT NULL DEFAULT '',
    forbidden       BOOLEAN NOT NULL DEFAULT false,    -- 禁用词标记，QA 阶段命中直接阻断
    version         INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE UNIQUE INDEX uq_terms_project_source ON terms (project_id, source_term);

CREATE TABLE glossary_versions (
    id              BIGSERIAL PRIMARY KEY,
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    version         INT NOT NULL,
    changed_term_id UUID NOT NULL,
    change_kind     TEXT NOT NULL CHECK (change_kind IN ('created','updated','deleted')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_glossary_versions_project ON glossary_versions (project_id, version DESC);

-- 翻译记忆库主表（沿用 OFCAT TM 概念，适配多租户/多项目 SaaS 架构）
CREATE TABLE translation_memory (
    id              BIGSERIAL,
    project_id      UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_lang     TEXT NOT NULL,
    target_lang     TEXT NOT NULL,
    source_text     TEXT NOT NULL,
    target_text     TEXT NOT NULL,
    source_hash     TEXT NOT NULL,        -- sha256(normalize(source_text))，精确匹配用
    origin          TEXT NOT NULL DEFAULT 'human' CHECK (origin IN ('human','mt_confirmed','import')),
    quality_score   SMALLINT,             -- 0-100，人工评分/QA 自动打分
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (project_id, id)
) PARTITION BY HASH (project_id);

-- 按 project_id 哈希分区，16 个分区（见 §5 分区策略）
CREATE TABLE translation_memory_p00 PARTITION OF translation_memory FOR VALUES WITH (MODULUS 16, REMAINDER 0);
CREATE TABLE translation_memory_p01 PARTITION OF translation_memory FOR VALUES WITH (MODULUS 16, REMAINDER 1);
-- ... p02 ~ p15 依此类推，CI 迁移脚本中生成全部 16 张

CREATE INDEX idx_tm_exact_match ON translation_memory (project_id, source_lang, target_lang, source_hash);

-- 语义向量索引表，与 TM 主表通过 (project_id, id) 关联，同库同事务保证一致性（技术选型 ADR-30）
CREATE TABLE tm_vectors (
    project_id      UUID NOT NULL,
    tm_id           BIGINT NOT NULL,
    embedding       vector(1024) NOT NULL,     -- bge-m3 嵌入维度
    PRIMARY KEY (project_id, tm_id),
    FOREIGN KEY (project_id, tm_id) REFERENCES translation_memory (project_id, id) ON DELETE CASCADE
);
CREATE INDEX idx_tm_vectors_hnsw ON tm_vectors USING hnsw (embedding vector_cosine_ops);

CREATE TABLE outbox_event (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_project_db_outbox_created_at ON outbox_event (created_at);
```

### 4.4 `task_db`（含任务生命周期、媒体资产、ASR/字幕明细）

```sql
CREATE TABLE tasks (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    project_id      UUID NOT NULL,
    org_id          UUID NOT NULL,
    media_type      TEXT NOT NULL CHECK (media_type IN
                       ('text','audio','video','pdf','docx','xlsx','pptx','odt','ods','odp','gif','webp')),
    source_lang     TEXT NOT NULL,
    target_lang     TEXT NOT NULL,
    source_file_id  UUID NOT NULL,          -- 逻辑外键，指向 file_db.files.id
    status          TEXT NOT NULL DEFAULT 'queued' CHECK (status IN
                       ('queued','ingesting','processing','rendering','completed','failed','canceled','partially_failed')),
    priority        TEXT NOT NULL DEFAULT 'normal' CHECK (priority IN ('low','normal','high')),
    output_formats  TEXT[] NOT NULL DEFAULT '{}',
    created_by      UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    completed_at    TIMESTAMPTZ
);
CREATE INDEX idx_tasks_project_status_created ON tasks (project_id, status, created_at);
CREATE INDEX idx_tasks_org_status ON tasks (org_id, status);
-- 对账任务（worker-service）扫描"长期未推进"任务的专用索引
CREATE INDEX idx_tasks_stalled ON tasks (status, updated_at) WHERE status IN ('ingesting','processing','rendering');

-- 每个任务的媒体处理子阶段明细（对应 asr/ocr/subtitle/office/render 各阶段状态）
CREATE TABLE task_media_items (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    stage           TEXT NOT NULL CHECK (stage IN ('ingest','asr','ocr','subtitle','office','render')),
    status          TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','running','completed','failed','skipped')),
    result_file_id  UUID,                   -- 逻辑外键，指向 file_db.files.id
    error_code      TEXT,
    error_message   TEXT,
    metrics         JSONB,
    started_at      TIMESTAMPTZ,
    finished_at     TIMESTAMPTZ,
    UNIQUE (task_id, stage)
);
CREATE INDEX idx_task_media_items_task_id ON task_media_items (task_id);

-- 原始文件/派生产物的媒体资产登记（与 file_db.files 是同一份文件在任务视角的引用+媒体元数据）
CREATE TABLE media_assets (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    file_id         UUID NOT NULL,          -- 逻辑外键，指向 file_db.files.id
    asset_kind      TEXT NOT NULL CHECK (asset_kind IN
                       ('source','asr_transcript','ocr_result','subtitle','office_translated','render_output')),
    media_type      TEXT NOT NULL,
    duration_seconds NUMERIC(10,2),         -- 音视频专用，图片/文档为 NULL
    page_count      INT,                    -- PDF/Office 专用
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_media_assets_task_id ON media_assets (task_id);
CREATE INDEX idx_media_assets_kind ON media_assets (task_id, asset_kind);

-- ASR 转写结果明细（词级/句级时间戳）
CREATE TABLE asr_transcripts (
    id              BIGSERIAL PRIMARY KEY,
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    media_asset_id  UUID NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    seq             INT NOT NULL,           -- 转写片段序号，保证顺序
    start_ms        INT NOT NULL,
    end_ms          INT NOT NULL,
    text            TEXT NOT NULL,
    confidence      REAL,
    UNIQUE (media_asset_id, seq)
);
CREATE INDEX idx_asr_transcripts_task_id ON asr_transcripts (task_id);

-- 字幕分段（翻译单元），源文/译文并列，供 subtitle-service 生成 srt/vtt/ass
CREATE TABLE subtitle_segments (
    id              BIGSERIAL PRIMARY KEY,
    task_id         UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    media_asset_id  UUID NOT NULL REFERENCES media_assets(id) ON DELETE CASCADE,
    seq             INT NOT NULL,
    start_ms        INT NOT NULL,
    end_ms          INT NOT NULL,
    source_text     TEXT NOT NULL,
    target_text     TEXT,
    tm_level        TEXT CHECK (tm_level IN ('L0','L1','MT','MISS')),
    qa_pass         BOOLEAN,
    UNIQUE (media_asset_id, seq)
);
CREATE INDEX idx_subtitle_segments_task_id ON subtitle_segments (task_id);

-- Outbox 表（架构设计书 §7.2 原始示例即出自本库，此处完整保留）
CREATE TABLE task_events_outbox (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,          -- 'task'
    aggregate_id    TEXT NOT NULL,          -- task_id，作为后续 Kafka Partition Key
    event_type      TEXT NOT NULL,          -- 'task.created' / 'task.completed' 等
    payload         JSONB NOT NULL,
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_task_events_outbox_created_at ON task_events_outbox (created_at);
```

### 4.5 `file_db`

```sql
CREATE TABLE files (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    org_id          UUID NOT NULL,
    project_id      UUID,
    filename        TEXT NOT NULL,
    content_type    TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL,
    storage_backend TEXT NOT NULL DEFAULT 'minio' CHECK (storage_backend IN ('minio','nfs')),
    storage_key     TEXT NOT NULL,           -- 对象存储 bucket 内 key
    purpose         TEXT NOT NULL DEFAULT 'task_source' CHECK (purpose IN ('task_source','task_derived','other')),
    status          TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active','deleted')),
    uploaded_by     UUID NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at      TIMESTAMPTZ
);
CREATE INDEX idx_files_org_id ON files (org_id);
CREATE INDEX idx_files_project_id ON files (project_id);

CREATE TABLE file_versions (
    id              BIGSERIAL PRIMARY KEY,
    file_id         UUID NOT NULL REFERENCES files(id) ON DELETE CASCADE,
    version         INT NOT NULL,
    storage_key     TEXT NOT NULL,
    size_bytes      BIGINT NOT NULL,
    created_by_service TEXT NOT NULL,        -- 'render-writer-service' 等
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE (file_id, version)
);

CREATE TABLE outbox_event (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,
    aggregate_id    TEXT NOT NULL,
    event_type      TEXT NOT NULL,           -- 'file.uploaded' / 'file.version_added'
    payload         JSONB NOT NULL,
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_file_db_outbox_created_at ON outbox_event (created_at);
```

### 4.6 `notification_db`

```sql
CREATE TABLE notifications (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL,
    type            TEXT NOT NULL,           -- 'task.completed' 等，与事件 template 一致
    title           TEXT NOT NULL,
    body            TEXT,
    read_at         TIMESTAMPTZ,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_notifications_user_unread ON notifications (user_id, created_at DESC) WHERE read_at IS NULL;

CREATE TABLE notification_prefs (
    user_id         UUID PRIMARY KEY,
    email_enabled   BOOLEAN NOT NULL DEFAULT true,
    ws_enabled      BOOLEAN NOT NULL DEFAULT true,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 4.7 `report_db`

```sql
CREATE TABLE usage_daily (
    org_id          UUID NOT NULL,
    project_id      UUID NOT NULL,
    usage_date      DATE NOT NULL,
    media_minutes   NUMERIC(10,2) NOT NULL DEFAULT 0,
    task_count      INT NOT NULL DEFAULT 0,
    PRIMARY KEY (org_id, project_id, usage_date)
) PARTITION BY RANGE (usage_date);

-- 按月分区，示例（迁移脚本按需滚动创建未来分区，见 §5.3）
CREATE TABLE usage_daily_2026_08 PARTITION OF usage_daily
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE usage_daily_2026_09 PARTITION OF usage_daily
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');

CREATE TABLE billing_items (
    id              BIGSERIAL PRIMARY KEY,
    org_id          UUID NOT NULL,
    period_start    DATE NOT NULL,
    period_end      DATE NOT NULL,
    item_kind       TEXT NOT NULL,           -- 'media_minutes' / 'seat' 等
    quantity        NUMERIC(10,2) NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX idx_billing_items_org_period ON billing_items (org_id, period_start);

CREATE TABLE qa_stats (
    id              BIGSERIAL PRIMARY KEY,
    project_id      UUID NOT NULL,
    stat_date       DATE NOT NULL,
    qa_pass_count   INT NOT NULL DEFAULT 0,
    qa_block_count  INT NOT NULL DEFAULT 0,
    tm_hit_rate     REAL,
    UNIQUE (project_id, stat_date)
);
```

### 4.8 `audit_db`

```sql
CREATE TABLE audit_logs (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL UNIQUE,     -- 幂等消费用（架构设计书 §6.7）
    org_id          UUID NOT NULL,
    actor_user_id   UUID,
    action          TEXT NOT NULL,            -- 'glossary.term_updated' 等
    resource_type   TEXT NOT NULL,
    resource_id     TEXT NOT NULL,
    before_state    JSONB,
    after_state     JSONB,
    ip              INET,
    occurred_at     TIMESTAMPTZ NOT NULL,
    ingested_at     TIMESTAMPTZ NOT NULL DEFAULT now()
) PARTITION BY RANGE (occurred_at);

CREATE TABLE audit_logs_2026_08 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-08-01') TO ('2026-09-01');
CREATE TABLE audit_logs_2026_09 PARTITION OF audit_logs
    FOR VALUES FROM ('2026-09-01') TO ('2026-10-01');

CREATE INDEX idx_audit_logs_org_occurred ON audit_logs (org_id, occurred_at DESC);
CREATE INDEX idx_audit_logs_resource ON audit_logs (resource_type, resource_id);
```

---

## 5. 索引设计与慢查询预案

### 5.1 核心高频查询索引（延续架构设计书 §5.4，本书补齐落地明细）

| 查询场景 | 索引 | 说明 |
|---|---|---|
| 任务列表按项目+状态+时间排序 | `idx_tasks_project_status_created` | task-service `GET /v1/tasks?project_id&status` 分页查询 |
| TM 精确匹配 | `idx_tm_exact_match (project_id, source_lang, target_lang, source_hash)` | translation-core `TMMatch` 高频调用，命中走索引唯一定位 |
| TM 语义召回 | `idx_tm_vectors_hnsw USING hnsw (embedding vector_cosine_ops)` | pgvector HNSW，`m=16, ef_construction=64`（默认参数起步，压测后按 §17 容量规划方法论调优） |
| 对账任务扫描滞留任务 | `idx_tasks_stalled` 部分索引（`WHERE status IN (...)`） | 只索引"未完成"状态子集，避免全表扫描且索引体积小 |
| 审计日志按组织+时间查询 | `idx_audit_logs_org_occurred` | 管理端 `GET /v1/audit-logs` 主查询路径 |

### 5.2 慢查询监控预案

- 全部逻辑库启用 `pg_stat_statements`，`postgres_exporter` 采集，Grafana 面板阈值 P95 > 200ms 告警（与架构设计书 §5.4 一致）。
- 慢查询排查标准流程：`EXPLAIN (ANALYZE, BUFFERS)` → 检查是否命中预期索引 → 若为新查询模式导致缺索引，走正常迁移流程新增索引（避免生产环境临时 `CREATE INDEX` 不加 `CONCURRENTLY` 导致锁表）。
- 新增索引一律使用 `CREATE INDEX CONCURRENTLY`，避免阻塞在线写入（该操作不可在事务块内执行，需在迁移脚本中标注 `-- sqlx-migrate: no-transaction`）。

---

## 6. 分区/分表策略

| 表 | 分区方式 | 分区数/粒度 | 理由 |
|---|---|---|---|
| `project_db.translation_memory` | HASH（按 `project_id`） | 16 分区 | TM 预估百万级句对（架构设计书 §17.4/技术选型 ADR-30），按租户哈希分区可将单租户大量导入的写入压力分散到不同分区，同时查询天然带 `project_id` 条件可被分区裁剪（Partition Pruning） |
| `project_db.tm_vectors` | 不分区（随 `translation_memory` 逻辑关联，HNSW 索引本身对大数据量有较好的近似检索性能，暂不需要物理分区） | — | 当前量级 pgvector 单表 HNSW 足够，达到千万级再评估拆分（技术选型 ADR-30 结论） |
| `report_db.usage_daily` | RANGE（按 `usage_date`，月粒度） | 按月滚动创建，worker-service 定时任务提前创建下月分区 | 用量统计表持续增长且主要按时间范围查询/归档，月分区便于按月归档/清理旧分区 |
| `audit_db.audit_logs` | RANGE（按 `occurred_at`，月粒度） | 按月滚动创建 | 审计日志合规要求保留 90 天+（架构设计书 §6.2 `audit.events` Retention 90 天为 Kafka 层，DB 层保留更久，见 §7），月分区便于按保留策略批量 DROP 过期分区而非逐行 DELETE（DROP PARTITION 是元数据操作，远快于批量 DELETE） |
| `task_db.tasks` / `task_media_items` / `asr_transcripts` / `subtitle_segments` | **暂不分区** | — | 当前 50–3000 并发用户量级下任务表增长速度可控（按 §17.2 容量估算，日均任务量级不会短期内达到分区收益显著的规模），先建好索引观察增长曲线，触发分区评审阈值（如单表超过 5000 万行或查询 P95 明显劣化）后再引入 RANGE 分区，避免过早分区增加维护复杂度（延续架构设计书 §1.2「不过度设计」原则） |

分区维护：新增未来分区、清理过期分区的定时任务由 `worker-service` 承接（§3.9 已列出的 Cron 调度职责范围）。

---

## 7. 数据保留与归档策略

| 数据 | 保留策略 | 归档/清理方式 |
|---|---|---|
| `task_db.tasks` 及关联明细 | 在线保留 180 天，之后归档 | 定时任务导出为 Parquet 落对象存储归档桶，PostgreSQL 侧物理删除；免费版/试用版可缩短至 30 天（按订阅套餐差异化，具体阈值由 `subscriptions` 表配置驱动） |
| `file_db.files`（原始文件/派生产物对象存储原文） | 与所属任务保留策略一致 + 用户主动删除（软删除后 30 天物理清理，留一次误删恢复窗口） | 软删除标记 `status='deleted'`，定时任务清理超过 30 天的软删记录及对象存储 Blob |
| `project_db.translation_memory` / `terms` | 长期保留（TM/术语库是核心资产，不设默认过期） | 仅用户主动删除，无自动清理任务 |
| `audit_db.audit_logs` | 在线分区保留 180 天（合规留痕），180 天后归档冷存储保留至少 3 年（视行业合规要求可延长） | 月分区 `DETACH PARTITION` 后导出归档，而非直接 DROP，保证合规可追溯 |
| `notification_db.notifications` | 90 天 | 定时任务批量删除超期记录（非核心资产，允许硬删除） |
| `report_db.usage_daily` / `billing_items` | 长期保留（计费依据，不设自动清理） | — |
| Kafka Topic Retention | 见架构设计书 §6.2 表格（3–90 天不等，DLQ 30 天） | Kafka 侧自动按 Retention 策略清理，与 PostgreSQL 侧保留策略是两套独立机制，PostgreSQL 数据不依赖 Kafka 保留期 |

---

## 8. 与 Kafka / Debezium 的对应关系

### 8.1 各库 Outbox 表与 CDC Connector 对应

| 逻辑库 | Outbox 表 | Publication | Debezium Connector 名 | 目标 Topic 前缀 |
|---|---|---|---|---|
| `task_db` | `task_events_outbox` | `task_outbox_pub` | `cats-task-outbox-connector` | `task.events` / `task.media.*.*` |
| `project_db` | `outbox_event` | `project_outbox_pub` | `cats-project-outbox-connector` | `project.events` |
| `user_db` | `outbox_event` | `user_outbox_pub` | `cats-user-outbox-connector` | `user.events` |
| `file_db` | `outbox_event` | `file_outbox_pub` | `cats-file-outbox-connector` | `file.events` |

> `auth_db`/`notification_db`/`report_db`/`audit_db` 当前无需 CDC 转发（`notification_db`/`report_db`/`audit_db` 是事件的**终点消费方**而非发布方；`auth_db` 的登录成功/失败事件量小且时效性要求不高，MVP 阶段由 auth-service 应用层直接同步产出审计事件即可，暂不建 Outbox+CDC 通道，符合架构设计书 §1.2 六问判定——按需增量引入）。

### 8.2 Debezium Connector 配置示例（以 `task_db` 为例）

```json
{
  "name": "cats-task-outbox-connector",
  "config": {
    "connector.class": "io.debezium.connector.postgresql.PostgresConnector",
    "database.hostname": "task-db-rw.cats-core.svc.cluster.local",
    "database.port": "5432",
    "database.user": "debezium_task",
    "database.dbname": "task_db",
    "plugin.name": "pgoutput",
    "slot.name": "task_outbox_slot",
    "publication.name": "task_outbox_pub",
    "table.include.list": "public.task_events_outbox",
    "transforms": "outbox",
    "transforms.outbox.type": "io.debezium.transforms.outbox.EventRouter",
    "transforms.outbox.table.field.event.id": "event_id",
    "transforms.outbox.table.field.event.key": "aggregate_id",
    "transforms.outbox.table.field.event.type": "event_type",
    "transforms.outbox.table.field.event.payload": "payload",
    "transforms.outbox.route.by.field": "event_type",
    "transforms.outbox.route.topic.replacement": "${routedByValue}",
    "heartbeat.interval.ms": "10000"
}}
```
`route.by.field`+`route.topic.replacement` 使得同一 Outbox 表内不同 `event_type`（如 `task.created`/`task.media.asr.requested`）可路由到不同 Topic，避免所有事件挤在单一 Topic 里由消费者自行过滤。

对应 PostgreSQL 侧准备：
```sql
ALTER SYSTEM SET wal_level = 'logical';
CREATE PUBLICATION task_outbox_pub FOR TABLE task_events_outbox;
CREATE ROLE debezium_task WITH REPLICATION LOGIN PASSWORD '***';
GRANT SELECT ON task_events_outbox TO debezium_task;
```

### 8.3 复制槽监控

沿用架构设计书 §7.4：`pg_replication_slots` 视图暴露 `confirmed_flush_lsn` 与当前 WAL LSN 差值，`postgres_exporter` 采集，超过 500MB 阈值触发告警。各 Outbox 表额外配一个定时清理任务（`worker-service`），删除 Debezium 已确认转发且超过 7 天的 Outbox 记录，避免表膨胀（架构设计书 §7.2 已提及，本书明确清理任务归属 worker-service）。

---

## 9. HA / 备份策略衔接

沿用架构设计书 §5.5、§5.6：CloudNativePG 管理，1 主 2 备（同步+异步流复制），WAL 连续归档 + 每日全量备份，全量保留 30 天/WAL 滚动保留 7 天，季度恢复演练。8 个逻辑库共享同一套 CNPG 集群实例（阶段一/二），按 §2 权限矩阵隔离账号，避免"同集群不同库"被误解为共享数据边界——**逻辑隔离与物理集群共享是两个独立维度**，本书的 8 库划分是逻辑/Schema 级隔离，不代表 8 套物理集群。
