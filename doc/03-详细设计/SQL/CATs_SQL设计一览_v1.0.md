# CATs SQL 设计一览 v1.0（T-04 Sprint 1 整合版）

> **文档编号**：CATs-DD-048（CATs SQL 设计）
> **フェーズ**：48 SQL 設計（詳細設計 フェーズ）
> **关联任务**：150 任务 #48（Sprint 1 T-04 范围）+ 启动会决议 1+2（接口+模块设计书 v2.0 双升版基线对齐）
> **版本**：v1.0
> **创建日**：2026-09-01
> **状态**：评审前草稿（待 DDD Review 6 角色共识 + M1-Sprint 1 末整合）
> **密级**：仅社内
> **作者**：DBA Lead（Ulysses 兼任一人公司 12 角色 DBA 槽位 / Mavis 接手 agent per DEC-008 代签 per 2026-08-27 19:39 JST 强化授权）

---

## 文档管理信息

### 审批栏（6 角色 per 技术基线 v1.0+2 共识模型）

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 数据库设计书 v2.0 §4 cross-ref 100% 验证 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-01/T-02 DDL 落地对齐 + EXPLAIN 实施可行性 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 起草方 / 索引策略 / 性能基线 |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 5-8 条关键 SQL EXPLAIN ANALYZE 验证 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | Sprint 1 §2 T-04 完成判据核验 |

> 6 角色不兼任 per 2026-08-21 决议（5 域 Lead 严格独立 + SRE 平台 = 6 角）。本表 6 角色 = Sponsor + 5 域 Lead 独立签字栏。

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-01 | DBA Lead（Mavis 接手 agent per DEC-008）| 初版：M1-Sprint 1 T-04 范围（auth_db + user_db 4 表）+ 8 条关键 SQL + EXPLAIN 验证占位 + 已知缺口 5 项（v1.1 既有内容范围不匹配 / DB 设计书 v2.0 §4 schema 冲突等） |

---

## 0. 元信息

| 项 | 值 |
|----|---|
| **Worktree** | `D:/CATs/.worktrees/feat-m1-sprint1-t04-sql` |
| **分支** | `feat/m1-sprint1-t04-sql` |
| **commit baseline** | `49fdbb3`（per `git log -1`，WBS Sprint 1 跟踪 v1.0 落地前） |
| **关联基线（B0.0）** | `4f96f95`（CAB-001 v1.0） |
| **上游源文档** | 见 §0.1 引用清单（git 实证 12 份） |
| **下游引用** | T-06 集成测试 ITa 用例 / T-05 CI Pipeline DB fixture / Sprint 1 复盘 v1.0 |
| **覆盖范围** | auth_db（3 表）+ user_db（1 表）= 共 4 表 |
| **不覆盖范围** | project_db / task_db / tm_db / term_db / report_db / audit_db / notification_db / file_db（Sprint 2+ 范围） |

### 0.1 引用清单（git 实证 12 份）

> 引用纪律（per 2026-08-26 AI 协作文档治理强证据 + 2026-09-01 DTL-036 hotfix 案例）：以下每条引用均通过 `git log -1 --format='%H %s' -- <path>` 在本 worktree 实证，引用时注明 commit hash + 修订履历状态。

| # | 引用文档 | 路径 | commit hash | 用途 |
|---|---------|------|------------|------|
| 1 | CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `efd9e77` | §2 T-04 任务清单 + §5 R-03 风险 + §6.5 token-OLU 立项 |
| 2 | CATs_数据库设计书 v2.0+1 | `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md` | `cd40911` | §4 auth_db / user_db schema（cross-ref 目标）+ §5 索引策略 + §7 数据保留 |
| 3 | CATs_接口设计书 v2.0+1 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `0eb1e9f` | §3.1 auth 端点（login/refresh/logout/me）+ §3.2 user 端点（get/update） |
| 4 | CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f53` | §3 错误码分类 + §4 auth 端点错误码矩阵 + §5 审计事件类型映射 |
| 5 | CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | `d1b10fe` | §3 合规（等保 2.0 三级）+ §4 认证（密码策略 + Argon2id）+ §6 加密 + §7 审计 |
| 6 | T-01 完成 commit (auth-service + migrations) | `crates/auth-service/` + `crates/auth-service/migrations/` | `2146f53` | auth_db DDL 实物（0001 ~ 0004）+ Rust 源码 4 表关联 |
| 7 | T-02 完成 commit (user-service + migrations) | `crates/user-service/` + `crates/user-service/migrations/` | `89f72cd` | user_db DDL 实物（0001）+ Rust 源码 1 表关联 |
| 8 | CATs_Baseline 一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f95` | §3.3.3 D-D-048 SQL 设计一览 v1.0 待基线化 + §5.3 数据库 Schema 索引 |
| 9 | CATs_微服务架构设计书 v1.0+1 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §4.1 核心 8 MVP 服务（auth/user 边界）+ §5.1 8 逻辑库 |
| 10 | CATs_技术基线 v1.0+2 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §1 锁定 PG 18.6 + pgvector 0.8.6 + Rust 1.98.0 |
| 11 | CATs_token-OLU 框架 v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | `f6772ce` | §3.1 DBA Lead 系数 200K-350K tokens / 1 周上限 1M tokens |
| 12 | CATs_WBS Sprint 1 跟踪 v1.0 | `doc/05-其他/管理/CATs_WBS_Sprint1_跟踪_v1.0.md` | `49fdbb3` | §2.2 WBS 编码 PMO.S1.T04 + §3 Lead 估时表 + §5 5 commit 落地登记 |
| 13 | CATs_RACISLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | `efd9e77`（同期）| 启动会决议 4 落地 / 24h Consulted SLA |

> 上述 commit hash 全部经 `git log -1 --format='%H %s' -- <path>` 在 `D:/CATs/.worktrees/feat-m1-sprint1-t04-sql` worktree 实证通过（2026-09-01 22:25 JST 验证）。

### 0.2 术语

| 术语 | 含义 |
|------|------|
| **DDL** | Data Definition Language（CREATE TABLE / CREATE INDEX / ALTER TABLE 等） |
| **DML** | Data Manipulation Language（INSERT / UPDATE / DELETE / SELECT 等） |
| **CITEXT** | PostgreSQL 大小写不敏感文本类型（per DB 设计书 v2.0 §4.1 users_credential.email） |
| **HNSW** | Hierarchical Navigable Small World（pgvector 0.8.6 ANN 索引，本文档范围不涉及，留 Sprint 2） |
| **B-tree** | PostgreSQL 默认索引类型（本文档主要使用） |
| **Partial Index** | 部分索引（带 WHERE 条件，如 `idx_users_credential_is_active WHERE is_active = true`） |
| **EXPLAIN ANALYZE** | PostgreSQL 查询计划分析命令（实际执行 + 收集统计） |
| **Seq Scan** | 顺序全表扫描（> 1k 行表出现即不达标 per T-04 完成判据 ②） |
| **cross-ref** | cross-reference，跨文档互引一致性核对 |
| **R-03 风险** | Sprint 1 §5 风险 R-03：跨库 SQL 范围蔓延（per 任务拆解 v1.0+2 §5） |
| **OI** | Open Item（已知缺口，对应技术基线 v1.0+2 §8） |

### 0.3 范围限定（per Sprint 1 §5 R-03 风险 + 任务拆解 v1.0+2 §6.5 token-OLU DBA Lead 200K-350K tokens）

| 范围 | 包含 | 不包含 |
|------|------|--------|
| **逻辑库** | auth_db / user_db | project_db / task_db / tm_db / term_db / report_db / audit_db / notification_db / file_db |
| **表数** | 4 表（users_credential / refresh_token_revoke / audit_log / user_profile） | 其他 6 库 30+ 表 |
| **SQL 数** | 8 条关键 SQL（登录 / Token 刷新 / 审计查询 / 用户查询 / 用户创建 / 用户更新 / 用户角色查询 / 审计事件追加） | 其他库 SQL（待 Sprint 2 DBA Lead 升 v1.2/v2.0 整合） |
| **索引数** | 9+ 索引（含 DDL 默认 6 + 推荐增量 3） | 跨库联合索引 / 全文索引（GIN）/ pgvector HNSW |
| **DDL 文件** | 5 份（auth-service 4 + user-service 1） | 跨库 migration（V001 全集不存在） |

---

## 1. 目的与范围

### 1.1 与数据库设计书 v2.0 §4 的关系

**数据库设计书 v2.0+1**（commit `cd40911`，v2.1 = 2026-08-26 基线升级锁定 PG 18.6）是 CATs 8 逻辑库 DDL 的**设计基线**，覆盖全部 30+ 表。**SQL 设计一览 v1.0**（本文）是其**Sprint 1 范围深化**：

- 聚焦 auth_db（3 表）+ user_db（1 表）共 4 表
- 把 §4 DDL 片段**全文收录**到 §2（含 trigger / 索引 / 约束完整 DDL）
- 增加 §3 关键 SQL（每条含 触发端点 / 错误码 / 索引路径 / EXPLAIN 验证 / 性能目标）
- 增加 §4 索引策略（已有 + 推荐增量 + 维护）
- 增加 §5 性能基线（容量预估 / P95 目标 / EXPLAIN 验证状态）
- 增加 §6 安全考虑（参数化查询 / 密码哈希 / 审计不可篡改）
- 增加 §7 迁移路径（V001~V005 顺序 + 回滚策略）

### 1.2 与 Sprint 1 T-01 / T-02 的关系

- **T-01**（commit `2146f53`）：auth-service 实战深化 → 已落地 4 份 migration（0001 ~ 0004），覆盖 users_credential / refresh_token_revoke / audit_log
- **T-02**（commit `89f72cd`）：user-service 脚手架 → 已落地 1 份 migration（0001），覆盖 user_profile
- **T-04**（本文）：SQL 设计一览 v1.0 → 把上述 5 份 migration DDL 集中登记 + 8 条关键 SQL 走索引验证

```
T-01 (auth-service DDL 实物, 4 migrations)
    │
    ├──→ T-04 (本文, 引用 §2 集中登记 + §3 关键 SQL 走索引验证)
    │
T-02 (user-service DDL 实物, 1 migration)
    │
    └──→ T-04 (本文, 引用 §2 集中登记 + §3 关键 SQL 走索引验证)
```

### 1.3 完成判据（per Sprint 1 §2 T-04）

| # | 完成判据 | 状态 |
|---|---------|------|
| ① | SQL 设计一览 v1.0 提交至 `doc/03-详细设计/SQL/CATs_SQL设计一览_v1.0.md` | ✅ 本文档 |
| ② | 5-8 条关键 SQL `EXPLAIN ANALYZE` 全部走索引（无 Seq Scan on > 1k 行表） | 🟡 占位（待 e2e 测试验证，详见 §3 各 SQL 末尾） |
| ③ | 与数据库设计书 v2.0 §4 cross-ref 100% 覆盖 | 🟡 部分（见 §8 已知缺口 §3 DB 设计书 schema 冲突） |

---

## 2. DDL 集中登记

> **来源**：T-01 / T-02 实际落地的 5 份 migration 文件（per commit `2146f53` + `89f72cd`）。
> **冲突说明**：与数据库设计书 v2.0 §4 在 schema 细节上存在多处不一致（详见 §8 已知缺口 §3），本节以**实际 migration 为准**（DDL 实物权威），数据库设计书 v2.0 §4 留待 Sprint 2 升 v2.1 同步。

### 2.1 auth_db（3 表）

#### 2.1.1 `users_credential`（来源：0001_init.sql + 0002_add_email.sql）

```sql
-- 0001_init.sql (T-01 起始, commit 2146f53 之前 commit)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1
-- 引用: doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md §6.2 (auth_db 章节, per v2.1 注记)
-- 引用: doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §3 (认证: Argon2id 密码哈希)

-- 启用 pgcrypto 用于 gen_random_uuid()
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- users_credential 主表 (per CATs_Baseline一览 §5.1 auth_db 章节)
CREATE TABLE IF NOT EXISTS users_credential (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    username        TEXT UNIQUE NOT NULL,
    password_hash   TEXT NOT NULL,                  -- Argon2id (per §3 安全要件 v1.0)
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 0002_add_email.sql (T-01 实战深化 commit 2146f53)
-- 增量: 加 email TEXT (nullable, 无 UNIQUE 约束)
-- 选 nullable: 允许已存在的种子用户不被强制重置
ALTER TABLE users_credential
    ADD COLUMN IF NOT EXISTS email TEXT;

-- updated_at 自动维护 trigger
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS users_credential_set_updated_at ON users_credential;
CREATE TRIGGER users_credential_set_updated_at
    BEFORE UPDATE ON users_credential
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();

-- 索引
CREATE INDEX IF NOT EXISTS idx_users_credential_username
    ON users_credential (username);
CREATE INDEX IF NOT EXISTS idx_users_credential_is_active
    ON users_credential (is_active) WHERE is_active = true;
```

**字段说明**：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | UUID | PK, DEFAULT gen_random_uuid() | 全局唯一用户 ID |
| `username` | TEXT | UNIQUE NOT NULL | 登录用户名（per T-01 实际，与 DB 设计书 v2.0 §4.1 写的 `email CITEXT UNIQUE` 冲突，详见 §8 §3） |
| `password_hash` | TEXT | NOT NULL | Argon2id 哈希值（per 安全要件 v1.0 §4.2） |
| `is_active` | BOOLEAN | NOT NULL DEFAULT true | 用户启用标志（false = 软禁用，per 错误码表 v1.0 §3.3 `user_inactive`） |
| `email` | TEXT | NULL | 可选邮箱（0002 增量，**无 UNIQUE 约束**；DB 设计书 v2.0 §4.1 写 `email CITEXT NOT NULL UNIQUE`，冲突） |
| `created_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() | 创建时间 |
| `updated_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() | 更新时间（trigger 自动维护） |

**索引**：

| 索引名 | 类型 | 字段 | 用途 | 期望命中 SQL |
|--------|------|------|------|--------------|
| `users_credential_pkey` | B-tree | id | PK | §3.4 用户查询 |
| `users_credential_username_key` | UNIQUE B-tree | username | 登录唯一性 | §3.1 用户登录查询 |
| `idx_users_credential_username` | B-tree | username | 登录查询（与 UNIQUE 重复但保留显式命名） | §3.1 |
| `idx_users_credential_is_active` | Partial B-tree | is_active WHERE is_active = true | 活跃用户过滤 | §3.4（可选） |

#### 2.1.2 `refresh_token_revoke`（来源：0003_refresh_token_revoke.sql）

```sql
-- 0003_refresh_token_revoke.sql (T-01 实战深化 commit 2146f53)
-- 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
-- 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3 (refresh 端点)

-- 新增 refresh_token_revoke 表 (jti 撤销, JWT 轮换用)
-- 选 jti 而非整个 token: 减少存储, 验证 O(1) 查
-- 不设外键到 users_credential: 用户删除时审计仍保留
CREATE TABLE IF NOT EXISTS refresh_token_revoke (
    jti             UUID PRIMARY KEY,
    user_id         UUID NOT NULL,
    revoked_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    reason          TEXT NOT NULL  -- 'rotated' | 'logout' | 'admin_revoke' | 'expired'
);

-- 索引 (per 150 任务 #48 性能建议: 按 user_id 查询某用户全部撤销记录)
CREATE INDEX IF NOT EXISTS idx_refresh_token_revoke_user_id
    ON refresh_token_revoke (user_id);

-- 索引 (cleanup 作业: 删除过期撤销记录, 7 天前)
CREATE INDEX IF NOT EXISTS idx_refresh_token_revoke_revoked_at
    ON refresh_token_revoke (revoked_at);
```

**字段说明**：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `jti` | UUID | PK | JWT ID（per RFC 7519） |
| `user_id` | UUID | NOT NULL | 关联用户 ID（**不设 FK** 到 users_credential，跨表审计保留） |
| `revoked_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() | 撤销时间 |
| `reason` | TEXT | NOT NULL | 撤销原因：`rotated`（refresh 轮换）/ `logout`（登出）/ `admin_revoke`（管理员撤销）/ `expired`（过期清理） |

**索引**：

| 索引名 | 类型 | 字段 | 用途 | 期望命中 SQL |
|--------|------|------|------|--------------|
| `refresh_token_revoke_pkey` | B-tree | jti | PK 查 | §3.2 Token 刷新查询 |
| `idx_refresh_token_revoke_user_id` | B-tree | user_id | 按用户查所有撤销记录 | 监控 / 审计 |
| `idx_refresh_token_revoke_revoked_at` | B-tree | revoked_at | cleanup 过期记录（7 天前） | worker-service 定时任务 |

> **设计选择（vs DB 设计书 v2.0 §4.1）**：DB 设计书写 `sessions` 表（带 `refresh_token_hash TEXT UNIQUE NOT NULL` + `expires_at TIMESTAMPTZ NOT NULL` + `revoked_at TIMESTAMPTZ` + `client_kind TEXT CHECK IN ('tauri','web')`），T-01 实际选用**jti 撤销表**而非持久 session 表。理由：refresh token 是 JWT 自包含（不需 DB 查以验证签名），DB 只需记录**撤销**（黑名单语义），空间复杂度 O(撤销数) 而非 O(总 session 数)。详见 §8 §3 已知缺口。

#### 2.1.3 `audit_log`（来源：0004_audit_log.sql）

```sql
-- 0004_audit_log.sql (T-01 实战深化 commit 2146f53)
-- 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01
-- 引用: doc/05-其他/安全/CATs_安全要件定义书_v1.0.md §6 (审计)

-- 新增 audit_log 表 (审计事件落库, 永不丢)
-- Kafka 物理落地推 K3s 阶段二 (per Sprint 1 §6.10 已知缺口)
-- 当前用 InMemoryAuditSink + DB 兜底, T-01 范围内可验证
CREATE TABLE IF NOT EXISTS audit_log (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL UNIQUE,                -- 防重
    user_id         UUID,                                 -- nullable for 系统事件
    event_type      TEXT NOT NULL,                        -- 'login' | 'logout' | 'refresh' | 'refresh_revoked' | 'login_failed' 等
    outcome         TEXT NOT NULL,                        -- 'success' | 'failure'
    detail          JSONB,                                -- 结构化 detail (per §3 错误码表 v1.0)
    source_ip       TEXT,                                 -- 客户端 IP (per §6.1 安全要件, 改 TEXT 简化 bind, 后续可补 inet::text 索引)
    user_agent      TEXT,                                 -- 客户端 UA
    occurred_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 索引: 按 user_id + 时间倒序 (查询某用户审计历史)
CREATE INDEX IF NOT EXISTS idx_audit_log_user_id_occurred_at
    ON audit_log (user_id, occurred_at DESC);

-- 索引: 按 event_type 分类统计
CREATE INDEX IF NOT EXISTS idx_audit_log_event_type
    ON audit_log (event_type);

-- 索引: 按时间 (cleanup 90 天前的)
CREATE INDEX IF NOT EXISTS idx_audit_log_occurred_at
    ON audit_log (occurred_at);
```

**字段说明**：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | BIGSERIAL | PK | 自增主键（高吞吐写入用 BIGSERIAL 而非 UUID） |
| `event_id` | UUID | UNIQUE NOT NULL | 业务幂等 ID（防重写） |
| `user_id` | UUID | NULL | nullable for 系统事件（如 healthz 失败） |
| `event_type` | TEXT | NOT NULL | 事件类型（per 错误码表 v1.0 §5：`login` / `logout` / `refresh` / `refresh_revoked` / `login_failed` / `me_access` / `me_failed` / `refresh_failed` / `password_change_failed` / `permission_denied` 等） |
| `outcome` | TEXT | NOT NULL | 成功 / 失败（`success` / `failure`） |
| `detail` | JSONB | NULL | 结构化 detail（如 `{"reason": "wrong_password", "attempts": 3}`） |
| `source_ip` | TEXT | NULL | 客户端 IP（per 安全要件 v1.0 §6.1，简化 bind 用 TEXT 而非 INET） |
| `user_agent` | TEXT | NULL | 客户端 UA |
| `occurred_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() | 服务端时钟（per 安全要件 v1.0 §6.1） |

**索引**：

| 索引名 | 类型 | 字段 | 用途 | 期望命中 SQL |
|--------|------|------|------|--------------|
| `audit_log_pkey` | B-tree | id | PK 查 | 内部 JOIN |
| `audit_log_event_id_key` | UNIQUE B-tree | event_id | 幂等防重 | INSERT ON CONFLICT |
| `idx_audit_log_user_id_occurred_at` | B-tree | (user_id, occurred_at DESC) | 按用户查审计历史（最近 50 条） | §3.3 审计事件查询 |
| `idx_audit_log_event_type` | B-tree | event_type | 按事件类型聚合统计 | 监控 / 告警 |
| `idx_audit_log_occurred_at` | B-tree | occurred_at | cleanup 90 天前记录 | worker-service 定时任务 |

> **设计选择（vs DB 设计书 v2.0 §4.8）**：DB 设计书把审计日志放在独立的 `audit_db.audit_logs` 表（含 `org_id` + `actor_user_id` + `action` + `resource_type` + `resource_id` + `before_state/after_state` + `occurred_at` 等 RANGE 分区月表），T-01 实际选用 **auth_db 内的 audit_log 表**（含 `event_id` + `user_id` + `event_type` + `outcome` + `detail` + `occurred_at`）。理由：T-01 范围仅 auth-service 自身审计，跨服务审计事件（user/project/task 等）走 Kafka topic `audit.event`（per 接口设计书 v2.0+1 §3.1）→ audit-service 落 `audit_db.audit_logs`（per DB 设计书 v2.0 §4.8）。auth_db.audit_log 是 **service-local 兜底**（防 Kafka 不可达时丢审计），与 audit_db.audit_logs **不重复**（前者是 auth-service 调试用，后者是合规持久化）。详见 §8 §3 已知缺口。

### 2.2 user_db（1 表）

#### 2.2.1 `user_profile`（来源：0001_init_user_db.sql）

```sql
-- 0001_init_user_db.sql (T-02 脚手架 commit 89f72cd)
-- 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §5.1 (8 逻辑库)
-- 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1 (user_db 接口契约 v1.0.0)
-- 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0+1.md §2 T-02

-- 设计选择 (per 缺标比错标安全):
-- - 与 auth_db.users_credential 分离: 认证凭据 (auth_db) vs 用户画像 (user_db)
--   → user_profile.user_id 是 UUID, 但**不**做 FK 到 auth_db.users_credential
--   → 理由: 跨服务不直连, 8 逻辑库边界清晰; user_id 一致性由调用方保证
-- - email 字段 nullable, 唯一索引 (允许匿名用户)
-- - display_name 是业务展示字段, 与 auth_db.username 解耦
CREATE TABLE IF NOT EXISTS user_profile (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID NOT NULL UNIQUE,                  -- 关联 auth_db.users_credential.id, 业务一致
    display_name    TEXT NOT NULL,
    email           TEXT,                                  -- 允许匿名 (无 email)
    avatar_url      TEXT,
    locale          TEXT NOT NULL DEFAULT 'ja-JP',         -- i18n 默认值
    timezone        TEXT NOT NULL DEFAULT 'Asia/Tokyo',
    is_active       BOOLEAN NOT NULL DEFAULT true,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 唯一索引: email 唯一 (允许 NULL 重复)
CREATE UNIQUE INDEX IF NOT EXISTS idx_user_profile_email_unique
    ON user_profile (email)
    WHERE email IS NOT NULL;

-- 索引: 按 is_active 过滤活跃用户
CREATE INDEX IF NOT EXISTS idx_user_profile_is_active
    ON user_profile (is_active) WHERE is_active = true;

-- updated_at 自动维护 trigger (per auth_db 0001 模式)
CREATE OR REPLACE FUNCTION trg_set_updated_at() RETURNS trigger AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS user_profile_set_updated_at ON user_profile;
CREATE TRIGGER user_profile_set_updated_at
    BEFORE UPDATE ON user_profile
    FOR EACH ROW
    EXECUTE FUNCTION trg_set_updated_at();
```

**字段说明**：

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| `id` | UUID | PK, DEFAULT gen_random_uuid() | user_profile 自有 ID（与 `user_id` 区分：`id` 是 user_profile 表 PK，`user_id` 是关联 auth_db 业务 ID） |
| `user_id` | UUID | UNIQUE NOT NULL | 关联 `auth_db.users_credential.id`（**应用层保证一致，非物理 FK**，per 8 逻辑库边界 per 微服务架构书 §1.2 原则 4） |
| `display_name` | TEXT | NOT NULL | 业务展示名（与 auth_db.username 解耦，per 决议 7） |
| `email` | TEXT | NULL | 邮箱（允许匿名，partial unique index） |
| `avatar_url` | TEXT | NULL | 头像 URL（指向对象存储 / 用户上传） |
| `locale` | TEXT | NOT NULL DEFAULT 'ja-JP' | i18n 默认值（per 项目管理计划书 §6.2，CATs 主语言 ja-JP） |
| `timezone` | TEXT | NOT NULL DEFAULT 'Asia/Tokyo' | 时区默认（per 部署环境 JST） |
| `is_active` | BOOLEAN | NOT NULL DEFAULT true | 启用标志 |
| `created_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() | 创建时间 |
| `updated_at` | TIMESTAMPTZ | NOT NULL DEFAULT now() | 更新时间（trigger 自动维护） |

**索引**：

| 索引名 | 类型 | 字段 | 用途 | 期望命中 SQL |
|--------|------|------|------|--------------|
| `user_profile_pkey` | B-tree | id | PK 查 | 内部 JOIN |
| `user_profile_user_id_key` | UNIQUE B-tree | user_id | 按 user_id 唯一查（业务主键） | §3.4 用户查询 |
| `idx_user_profile_email_unique` | Partial UNIQUE B-tree | email WHERE email IS NOT NULL | 邮箱唯一性（允许匿名） | email 冲突检测 |
| `idx_user_profile_is_active` | Partial B-tree | is_active WHERE is_active = true | 活跃用户过滤 | 列表查询 |

> **设计选择（vs DB 设计书 v2.0 §4.2）**：DB 设计书写 user_db 含 5 表（`orgs` + `users_profile` + `org_members` + `subscriptions` + `outbox_event`），T-02 实际仅落地 1 表（`user_profile`）。理由：Sprint 1 T-02 范围仅"脚手架落地"（per §2 T-02 完成判据 5/5），`orgs` / `org_members` / `subscriptions` / `outbox_event` 4 表是 Sprint 2+ user-service 实战深化时落地。T-02 完成时 `users_profile` 字段也比 DB 设计书少 `org_id` FK（DB 设计书 `users_profile` 有 `org_id UUID NOT NULL REFERENCES orgs(id)`），因 T-02 时 `orgs` 还未建。详见 §8 §3 已知缺口。

### 2.3 DDL 集中登记小结

| 库 | 表 | DDL 实物来源 | 主键 | UNIQUE 索引 | 普通索引 | Trigger |
|----|------|--------------|------|-------------|---------|---------|
| auth_db | users_credential | 0001 + 0002 | id | username | is_active (partial) | trg_set_updated_at |
| auth_db | refresh_token_revoke | 0003 | jti | — | user_id / revoked_at | — |
| auth_db | audit_log | 0004 | id | event_id | (user_id, occurred_at) / event_type / occurred_at | — |
| user_db | user_profile | 0001 | id | user_id | email (partial unique) / is_active (partial) | trg_set_updated_at |
| **合计** | **4 表** | **5 份 migration** | **4 PK** | **4 UNIQUE** | **9 普通索引** | **2 trigger** |

---

## 3. 关键 SQL（8 条）

> **EXPLAIN 验证约束（per 任务拆解 v1.0+2 §2 T-04 完成判据 ②）**：
> - 必须使用 **PG 18.6**（per 技术基线 v1.0+2 §1）
> - 真实数据 `EXPLAIN ANALYZE` 验证：本 worktree 内无 test fixture（per `git ls-files` 仅 5 份 migration 实物），故 8 条 SQL 均标 **🟡 待 e2e 测试时验证** + 预期索引路径
> - 接受标准：**Seq Scan on > 1k 行表 = 失败**（per 任务拆解 v1.0+2 §2 T-04 ②）

### 3.1 用户登录查询（auth_db.users_credential）

**触发端点**：`POST /v1/auth/login`（per 接口设计书 v2.0+1 §3.1 + 错误码表 v1.0 §4.1）
**错误码**：`invalid_credentials` (401) / `user_inactive` (401) / `invalid_request` (400) / `server_error` (500)
**审计事件**：`login` (success) / `login_failed` (failure)

```sql
-- 用户登录查询 (per auth-service 实际实现)
-- 命中索引: users_credential_username_key (UNIQUE B-tree on username)
SELECT id, username, password_hash, is_active, email, created_at, updated_at
FROM users_credential
WHERE username = $1
  AND is_active = true;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Index Scan using users_credential_username_key on users_credential
  (cost=0.27..8.29 rows=1 width=...)
  Index Cond: (username = $1)
  Filter: (is_active = true)
  Rows Removed by Filter: 0
```

**预期索引路径**：`users_credential_username_key` (UNIQUE B-tree) → 单行定位
**性能目标**：P95 < 5ms（含网络 + 应用层 argon2id verify 实际 ~50-200ms，DB 层查询 < 5ms）
**EXPLAIN 验证状态**：🟡 待 e2e（worktree 内无 test fixture）

### 3.2 Token 刷新查询（auth_db.refresh_token_revoke）

**触发端点**：`POST /v1/auth/refresh`（per 接口设计书 v2.0+1 §3.1 + 错误码表 v1.0 §4.2）
**错误码**：`invalid_token` (401) / `token_revoked` (401) / `invalid_token_type` (401) / `server_error` (500)
**审计事件**：`refresh` (success) / `refresh_failed` (failure) / `refresh_revoked` (旧 jti 撤销)

```sql
-- Token 刷新查询 - 检查 jti 是否在撤销列表
-- 命中索引: refresh_token_revoke_pkey (B-tree on jti)
SELECT jti, user_id, revoked_at, reason
FROM refresh_token_revoke
WHERE jti = $1;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Index Scan using refresh_token_revoke_pkey on refresh_token_revoke
  (cost=0.27..8.29 rows=1 width=...)
  Index Cond: (jti = $1)
```

**预期索引路径**：`refresh_token_revoke_pkey` → PK 单行定位
**性能目标**：P95 < 5ms
**EXPLAIN 验证状态**：🟡 待 e2e

> **设计选择**：refresh 端点先做 JWT 签名校验（应用层，无 DB），再查 jti 是否在撤销表。撤销表**仅记撤销**（黑名单），不存活跃 session 列表（per §2.1.2 设计选择）。如 jti 不在表 = 未撤销 = 允许 refresh；如在 = 已撤销 = 返回 401 token_revoked。

### 3.3 审计事件查询（auth_db.audit_log）

**触发端点**：管理端 `GET /v1/auth/audit?user_id=...`（per 接口设计书 v2.0+1 §3.1 扩展 / Sprint 1 范围未实现，留 Sprint 2）
**用途**：按用户查最近 50 条审计事件（用户自助 / 管理端审计展示）
**错误码**：（Sprint 1 范围未公开端点；预留 `user_not_found` / `forbidden`）

```sql
-- 审计事件查询 - 按用户查最近 50 条
-- 命中索引: idx_audit_log_user_id_occurred_at (B-tree on (user_id, occurred_at DESC))
SELECT id, event_id, user_id, event_type, outcome, detail, source_ip, user_agent, occurred_at
FROM audit_log
WHERE user_id = $1
ORDER BY occurred_at DESC
LIMIT 50;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Limit  (cost=... rows=50 width=...)
  ->  Index Scan Backward using idx_audit_log_user_id_occurred_at on audit_log
        (cost=... rows=... width=...)
        Index Cond: (user_id = $1)
```

**预期索引路径**：`idx_audit_log_user_id_occurred_at` → 复合索引顺序扫描（无需 Sort）
**性能目标**：P95 < 50ms（按用户最近 50 条，索引覆盖查询无回表）
**EXPLAIN 验证状态**：🟡 待 e2e

### 3.4 用户查询（user_db.user_profile）

**触发端点**：`GET /v1/users/{id}`（per 接口设计书 v2.0+1 §3.2）
**错误码**：`user_not_found` (404) / `forbidden` (403) / `server_error` (500)

```sql
-- 用户查询 - 按 user_id (业务 ID) 查 user_profile
-- 命中索引: user_profile_user_id_key (UNIQUE B-tree on user_id)
SELECT id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at
FROM user_profile
WHERE user_id = $1
  AND is_active = true;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Index Scan using user_profile_user_id_key on user_profile
  (cost=0.27..8.29 rows=1 width=...)
  Index Cond: (user_id = $1)
  Filter: (is_active = true)
```

**预期索引路径**：`user_profile_user_id_key` (UNIQUE B-tree) → 单行定位
**性能目标**：P95 < 5ms
**EXPLAIN 验证状态**：🟡 待 e2e

### 3.5 用户创建（user_db.user_profile）

**触发端点**：`POST /v1/users`（T-02 脚手架已 stub；Sprint 1 范围未公开端点，per 接口设计书 v2.0+1 §3.2 未列）
**错误码**：预留 `email_conflict` (409) / `invalid_request` (400) / `server_error` (500)
**审计事件**：预留 `user.created`（Kafka topic `user.events` per 接口设计书 v2.0+1 §3.2）

```sql
-- 用户创建 (per user-service T-02 实战, INSERT ... RETURNING)
-- 命中索引: user_profile_pkey / user_profile_user_id_key / idx_user_profile_email_unique
INSERT INTO user_profile (
    id, user_id, display_name, email, avatar_url, locale, timezone, is_active
) VALUES (
    gen_random_uuid(),  -- id (PK)
    $1,                 -- user_id (from auth_db.users_credential.id, 业务一致)
    $2,                 -- display_name
    $3,                 -- email (nullable, partial UNIQUE 约束)
    $4,                 -- avatar_url (nullable)
    COALESCE($5, 'ja-JP'),  -- locale (default)
    COALESCE($6, 'Asia/Tokyo'),  -- timezone (default)
    COALESCE($7, true)  -- is_active (default true)
)
RETURNING id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Insert on user_profile  (cost=... rows=1 width=...)
  ->  Result  (cost=... rows=1 width=...)
```

**预期索引路径**：INSERT 不走索引读取，但 UNIQUE 约束检查 `user_profile_user_id_key` (UNIQUE B-tree) + `idx_user_profile_email_unique` (partial UNIQUE B-tree on email) → 冲突检测 < 5ms
**性能目标**：P95 < 10ms（含 UNIQUE 约束检查）
**EXPLAIN 验证状态**：🟡 待 e2e

### 3.6 用户更新（user_db.user_profile）

**触发端点**：`PUT /v1/users/{id}`（per 接口设计书 v2.0+1 §3.2）
**错误码**：`user_not_found` (404) / `forbidden` (403) / `invalid_request` (400) / `server_error` (500)

```sql
-- 用户更新 (整体覆盖式 PUT per 接口设计书 §3.2)
-- 命中索引: user_profile_user_id_key (UNIQUE B-tree on user_id)
-- trigger: trg_user_profile_set_updated_at (自动更新 updated_at)
UPDATE user_profile
SET
    display_name = $1,
    email        = $2,
    avatar_url   = $3,
    locale       = COALESCE($4, locale),
    timezone     = COALESCE($5, timezone),
    is_active    = COALESCE($6, is_active)
WHERE user_id = $7
  AND is_active = true
RETURNING id, user_id, display_name, email, avatar_url, locale, timezone, is_active, created_at, updated_at;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Update on user_profile  (cost=0.27..8.29 rows=1 width=...)
  ->  Index Scan using user_profile_user_id_key on user_profile
        (cost=0.27..8.29 rows=1 width=...)
        Index Cond: (user_id = $7)
        Filter: (is_active = true)
```

**预期索引路径**：`user_profile_user_id_key` → PK 单行定位 + UNIQUE email 索引检查（如 email 变更）
**性能目标**：P95 < 10ms
**EXPLAIN 验证状态**：🟡 待 e2e

### 3.7 用户角色查询（user_db.user_profile）

**触发端点**：`GET /v1/auth/roles` 管理端 + 服务间 `AuthCheck` gRPC 内部调用（per 接口设计书 v2.0+1 §3.1）
**错误码**：`forbidden` (403) / `server_error` (500)
**用途**：RBAC 矩阵引用（per Sprint 1 T-03 决议 8 + 启动会）

```sql
-- 用户角色查询 - 按 user_id 数组批量查角色 (RBAC 矩阵引用)
-- 命中索引: user_profile_user_id_key (UNIQUE B-tree on user_id)
-- 注: 当前 T-02 user_profile 表无 role 字段; T-03 RBAC 矩阵 v1.0 决定 role 字段
-- 位置: auth_db.users_credential (per 决议 7) 或 user_db.user_profile
-- 本 SQL 假设 role 字段在 user_db.user_profile (Sprint 2 实战深化时确认)
SELECT user_id, display_name
FROM user_profile
WHERE user_id = ANY($1::uuid[])
  AND is_active = true;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Index Scan using user_profile_user_id_key on user_profile
  (cost=0.27..8.29 rows=N width=...)
  Index Cond: (user_id = ANY ($1))
  Filter: (is_active = true)
```

**预期索引路径**：`user_profile_user_id_key` UNIQUE B-tree → N 行索引扫描
**性能目标**：P95 < 20ms（批量查 10-100 个 user_id）
**EXPLAIN 验证状态**：🟡 待 e2e

> **已知缺口**：当前 user_db.user_profile 表无 `role` 字段（per §2.2.1）。T-03 RBAC 矩阵 v1.0 决议 7 提到"role 字段位置"待定（auth_db.users_credential vs user_db.user_profile），本 SQL 假设 user_db 路径。如 T-03 决议放 auth_db，本 SQL 应改查 `users_credential`（已有 `id` PK + 无 role 字段也需加 migration）。详见 §8 §4 已知缺口。

### 3.8 审计事件追加（auth_db.audit_log）

**触发端点**：所有 auth-service 端点（per T-01 实战深化，per 错误码表 v1.0 §5 审计事件 ↔ 错误码映射）
**错误码**：审计写入失败不阻塞业务（best-effort + 重试队列；T-01 用 InMemoryAuditSink + DB 兜底）
**审计事件**：所有 11 种 event_type 走本 SQL 路径

```sql
-- 审计事件追加 (append-only, per T-01 实战 + 安全要件 v1.0 §7.1)
-- 命中索引: audit_log_event_id_key (UNIQUE B-tree on event_id) 防重
INSERT INTO audit_log (
    event_id, user_id, event_type, outcome, detail, source_ip, user_agent, occurred_at
) VALUES (
    $1,        -- event_id (UUID v4, 防重, UNIQUE 约束)
    $2,        -- user_id (nullable for 系统事件)
    $3,        -- event_type (per 错误码表 v1.0 §5 枚举)
    $4,        -- outcome ('success' | 'failure')
    $5,        -- detail (JSONB, 结构化)
    $6,        -- source_ip (TEXT)
    $7,        -- user_agent (TEXT)
    COALESCE($8, now())  -- occurred_at (服务端时钟, 默认 now())
)
ON CONFLICT (event_id) DO NOTHING
RETURNING id, event_id, occurred_at;
```

**预期 EXPLAIN 输出**（待 e2e 验证）：

```
Insert on audit_log  (cost=... rows=1 width=...)
  Conflict Resolution: DO NOTHING
  ->  Result  (cost=... rows=1 width=...)
```

**预期索引路径**：INSERT 走 `audit_log_event_id_key` UNIQUE 约束检查（幂等防重） + 无主键回表
**性能目标**：P95 < 10ms（写入为主，索引维护开销小）
**EXPLAIN 验证状态**：🟡 待 e2e

> **设计选择（vs DB 设计书 v2.0 §4.8）**：audit_log 表在 auth_db（T-01 实战）vs audit_db.audit_logs（DB 设计书）。前者是 **service-local 兜底**（防 Kafka 不可达丢审计），后者是 **合规持久化**（RANGE 分区月表）。两者不重复：auth-service 事件先写 auth_db.audit_log（同步）+ 同时 publish Kafka `audit.event`（异步）→ audit-service 消费落 audit_db.audit_logs（per 接口设计书 §3.1 "auth.events" 异步事件段）。**双写不冲突**（event_id UUID 跨表独立）。

---

## 4. 索引策略

### 4.1 已有索引（per §2 DDL 实物）

| 库 | 表 | 索引数 | PK | UNIQUE | Partial | 普通 |
|----|------|--------|----|----|---------|------|
| auth_db | users_credential | 4 | 1 | 1 (username) | 1 (is_active) | 1 (username 显式) |
| auth_db | refresh_token_revoke | 3 | 1 | 0 | 0 | 2 (user_id / revoked_at) |
| auth_db | audit_log | 5 | 1 | 1 (event_id) | 0 | 3 ((user_id, occurred_at) / event_type / occurred_at) |
| user_db | user_profile | 4 | 1 | 2 (user_id / email partial) | 1 (is_active) | 0 |
| **合计** | **4 表** | **16** | **4** | **4** | **2** | **6** |

### 4.2 推荐索引（Sprint 1 内可选增量 / Sprint 2 必选）

| # | 库 | 表 | 推荐索引 | 用途 | Sprint 1 必选 | 备注 |
|---|----|------|---------|------|---------------|------|
| 1 | auth_db | users_credential | `idx_users_credential_email` (B-tree) WHERE email IS NOT NULL | email 登录支持（per DB 设计书 v2.0 §4.1 规划） | ⚪ 待决议 | T-01 0002 未加 UNIQUE 约束；Sprint 2 实战深化时决议 |
| 2 | auth_db | audit_log | `idx_audit_log_source_ip` (B-tree) WHERE source_ip IS NOT NULL | 按 IP 聚合异常登录检测（per 安全要件 v1.0 §7.1） | ⚪ Sprint 2 | 监控告警 |
| 3 | user_db | user_profile | `idx_user_profile_org_id` (B-tree) | 多租户 org 隔离（per DB 设计书 v2.0 §4.2 规划） | ⚪ Sprint 2 | T-02 时 org 表未建；Sprint 2 加 |

> **不推荐盲目加索引**：每个 B-tree 索引在写入时增加 O(log N) 维护开销，对 auth_db.users_credential / user_db.user_profile 这种"写少读多"表可加；对 audit_log 这种"写多读少"表**慎加**，每加一个索引 INSERT 成本 +5%-15%。

### 4.3 索引维护

| 维护项 | 频率 | SQL | 备注 |
|--------|------|-----|------|
| VACUUM ANALYZE | 自动（autovacuum）| per PG 默认 | users_credential / user_profile 更新后统计信息自动更新 |
| VACUUM FULL | 季度 | `VACUUM FULL ANALYZE users_credential;` | 碎片化严重时手动（仅维护窗口） |
| REINDEX CONCURRENTLY | 半年 | `REINDEX INDEX CONCURRENTLY users_credential_username_key;` | 索引膨胀（B-tree fillfactor 默认 90） |
| pg_stat_statements | 持续 | `pg_stat_statements` 扩展 | 监控慢查询 → 索引建议 |

**重要约束（per DB 设计书 v2.0 §5.2）**：新增索引一律使用 `CREATE INDEX CONCURRENTLY`，避免阻塞在线写入（该操作不可在事务块内执行，需在 migration 脚本中标注 `-- sqlx-migrate: no-transaction`）。

---

## 5. 性能基线

### 5.1 EXPLAIN ANALYZE 结果

| # | SQL | 预期索引 | 性能目标 | 验证状态 |
|---|------|---------|----------|----------|
| 3.1 | 用户登录查询 | users_credential_username_key (UNIQUE) | P95 < 5ms | 🟡 待 e2e |
| 3.2 | Token 刷新查询 | refresh_token_revoke_pkey | P95 < 5ms | 🟡 待 e2e |
| 3.3 | 审计事件查询 | idx_audit_log_user_id_occurred_at | P95 < 50ms | 🟡 待 e2e |
| 3.4 | 用户查询 | user_profile_user_id_key (UNIQUE) | P95 < 5ms | 🟡 待 e2e |
| 3.5 | 用户创建 | user_profile_user_id_key + idx_user_profile_email_unique | P95 < 10ms | 🟡 待 e2e |
| 3.6 | 用户更新 | user_profile_user_id_key | P95 < 10ms | 🟡 待 e2e |
| 3.7 | 用户角色查询 | user_profile_user_id_key | P95 < 20ms | 🟡 待 e2e |
| 3.8 | 审计事件追加 | audit_log_event_id_key (UNIQUE) | P95 < 10ms | 🟡 待 e2e |

> **8/8 全部待 e2e 验证**（worktree 内无 test fixture）。验证计划：T-06 集成测试 ITa 落地后，QA Lead 在 test fixture（per token-OLU v0.1 §3.1 QA Lead 200K-400K tokens 估时范围）跑真实 `EXPLAIN ANALYZE` 报告到 `doc/04-测试/集成测试报告/CATs_M1_S1_集成测试报告_v1.0.md`。

### 5.2 性能目标（per 单条查询 P95）

| 类别 | P95 目标 | 备注 |
|------|----------|------|
| 单行 PK 查（login / refresh / user_profile） | < 5ms | UNIQUE 索引单行定位 |
| 单行 UNIQUE 查（含 trigger 写）| < 10ms | trigger trg_set_updated_at 开销 |
| 复合索引扫描（audit_log by user + time LIMIT 50）| < 50ms | 索引覆盖 + LIMIT 早停 |
| 单行 INSERT（含 UNIQUE 检查）| < 10ms | UNIQUE 约束索引检查 |
| 批量查（ANY 数组 N=10-100）| < 20ms | 索引数组扫描 |

**不接受**：Seq Scan on > 1k 行表（per T-04 完成判据 ② + 任务拆解 v1.0+2 §2）。

### 5.3 容量预估

| 库 | 表 | 1 年预估行数 | 5 年预估行数 | 增长模式 | 备注 |
|----|------|--------------|--------------|----------|------|
| auth_db | users_credential | 50K-300K | 200K-1M | 线性（每用户 1 行）| per 微服务架构书 §1.1（50-3000 并发用户） |
| auth_db | refresh_token_revoke | 5M-30M | 20M-100M | 线性（每用户 100 撤销/年）| 含 7 天 cleanup |
| auth_db | audit_log | 50M-300M | 200M-1B | 线性（每用户 1000 事件/年）| 90 天 cleanup |
| user_db | user_profile | 50K-300K | 200K-1M | 线性（每用户 1 行）| 同 users_credential |
| **合计** | 4 表 | **105M-630M** | **420M-2.1B** | — | — |

> **分区策略**（per DB 设计书 v2.0 §6 + T-04 范围限定）：
> - Sprint 1 范围**不分区**（per "不超 1 周上限" DBA Lead 1 周 1M tokens）
> - Sprint 2+ 按 DB 设计书 §6 规划：audit_log RANGE 月分区（90 天合规保留） + translation_memory HASH 16 分区（其他库，不在本文档范围）

### 5.4 慢查询预案（per DB 设计书 v2.0 §5.2）

- 全部逻辑库启用 `pg_stat_statements`（per DB 设计书 v2.0 §5.2 + 监控告警）
- `postgres_exporter` 采集 + Grafana 面板阈值 P95 > 200ms 告警
- 慢查询排查标准流程：`EXPLAIN (ANALYZE, BUFFERS)` → 检查是否命中预期索引 → 若为新查询模式导致缺索引，走正常 migration 流程新增索引（避免生产环境临时 `CREATE INDEX` 不加 `CONCURRENTLY` 锁表）

---

## 6. 安全考虑

### 6.1 SQL Injection 防护（per 错误码表 v1.0 §3.2 + 安全要件 v1.0 §9.2）

- **全部使用参数化查询**（`sqlx::query!` 宏或 `bind` 参数，绝不字符串拼接）
- Rust Lead 在 T-05 CI Pipeline 加 SAST 规则（cargo clippy + sqlx-migrate dry-run 校验无字符串拼接）
- 禁止：f-string 拼接 SQL、format!() 拼接 SQL、字符串模板拼接 SQL

### 6.2 敏感字段加密（per 安全要件 v1.0 §6.2）

| 字段 | 加密方式 | 备注 |
|------|----------|------|
| `users_credential.password_hash` | **Argon2id**（per 安全要件 v1.0 §4.2）| 应用层加密，不存明文；DB 仅存哈希 |
| `refresh_token_revoke.jti` | 不加密 | jti 是 UUID，本身不可逆推出 token |
| `audit_log.detail` | 不加密（JSONB）| detail 含 `reason` 等业务字段，不含 PII 明文 |
| `user_profile.email` | 不加密 | 业务展示字段，需明文查询 |
| 备份 | AES-256（per 安全要件 v1.0 §6.5）| 物理备份全量加密 |

### 6.3 审计事件不可篡改（per 安全要件 v1.0 §7.1 + DB 设计书 v2.0 §7）

- **append-only**：audit_log 表无 UPDATE / DELETE 权限给应用层（运行时角色 `svc_auth` 仅 INSERT + SELECT，per DB 设计书 v2.0 §2 权限矩阵）
- 物理删除仅 90 天后 worker-service 定时任务批量 DELETE（per DB 设计书 v2.0 §7 + 安全要件 v1.0 §6.5 审计日志 7 年保留 → 物理备份保留，不在线保留）
- **防重写**：`event_id UNIQUE` 约束保证幂等，防 replay 攻击
- **跨表对账**：auth_db.audit_log（service-local）vs audit_db.audit_logs（合规持久化）event_id 独立但 event_type / user_id / occurred_at 一致，便于离线对账

### 6.4 跨库一致性（user_id 业务一致，per §2.2.1 设计选择）

- `auth_db.users_credential.id` ≡ `user_db.user_profile.user_id`（应用层保证，非物理 FK）
- 不一致检测：worker-service 定期 LEFT JOIN（按 service token）找 user_profile 缺失的 user_id → 告警
- 跨服务调用时（auth-service → user-service）传 user_id（per 接口设计书 §1.2 "Gateway → 核心业务服务 Header 注入 X-Cats-User-Id"）

---

## 7. 迁移路径

### 7.1 auth_db 迁移（V001 ~ V003，per T-01 实际）

| 版本 | 文件 | commit | 内容 | 不可逆性 |
|------|------|--------|------|----------|
| V001 | `0001_init.sql` | (T-01 起始 commit) | CREATE EXTENSION pgcrypto + CREATE TABLE users_credential + trigger + 2 索引 | 低 |
| V002 | `0002_add_email.sql` | `2146f53` | ALTER TABLE users_credential ADD COLUMN email TEXT | 低（仅加列）|
| V003 | `0003_refresh_token_revoke.sql` | `2146f53` | CREATE TABLE refresh_token_revoke + 2 索引 | 中（新表）|
| V004 | `0004_audit_log.sql` | `2146f53` | CREATE TABLE audit_log + 3 索引 | 中（新表）|

### 7.2 user_db 迁移（V001，per T-02 实际）

| 版本 | 文件 | commit | 内容 | 不可逆性 |
|------|------|--------|------|----------|
| V001 | `0001_init_user_db.sql` | `89f72cd` | CREATE TABLE user_profile + 2 索引 + trigger | 中（新表）|

### 7.3 迁移回滚策略（per Expand-Contract 模式 + DB 设计书 v2.0 §3）

- **Expand 阶段**：新增列 / 表（V002 ~ V004 / user_db V001）— 当前 4 步均在此阶段
- **Contract 阶段**（Sprint 2+）：Sprint 2 实战深化时清理 V002 前的 username 字段（如决定全切 email）— 需 DB 设计书 v2.1 升版
- **回滚 SQL**（V002 示例）：`ALTER TABLE users_credential DROP COLUMN IF EXISTS email;` — 不可逆风险低（V002 加列无 UNIQUE 约束）
- **CI 校验**（per DB 设计书 v2.0 §5.2）：`sqlx migrate run --dry-run` 在 CI 跑通 + `cargo test -p cats-m1-s0-smoke auth` 验证连接

### 7.4 工具链

| 服务 | 迁移工具 | 路径 | 理由 |
|------|----------|------|------|
| auth-service | sqlx-migrate | `crates/auth-service/migrations/` | Rust 生态原生集成（per DB 设计书 v2.0 §3 表格）|
| user-service | sqlx-migrate | `crates/user-service/migrations/` | 同上 |
| CI 校验 | sqlx-cli | `cargo install sqlx-cli` | `sqlx migrate run --dry-run` 在 PR 跑通 |

---

## 8. 已知缺口

> **缺标比错标安全**（per 2026-08-26 AI 协作文档治理强证据 + DTL-036 v1.4 hotfix 案例）。以下 5 项已知缺口**逐项 git 实证** + **不编造回溯叙事**。

### §1 v1.1 既有内容范围不匹配（critical）

**现状**：本文件路径 `doc/03-详细设计/SQL/CATs_SQL设计一览_v1.0.md` 在 worktree 内已有 538 行 v1.1 内容（涵盖 TM/术语/项目/段/llm_usage/segments/RLS 等 7+ 库范围，含 `tm_segments` / `terms` / `projects` / `segments` / `user_activity` / `tu` / `outbox` / `versions` / `user_roles` 等**不存在的表**）。

**冲突**：
- 文件路径 v1.0 vs 文件内容 self-declared v1.1（per 文档管理信息 §修订履历 v1.1 修订日 2026-08-26）
- v1.1 内容覆盖范围 7+ 库 vs T-04 Sprint 1 范围 2 库（auth_db + user_db）
- v1.1 内容含 `tm_segments.embedding` / `terms.source_text` / `projects.tenant_id` / `segments.translated_by` / `user_activity.duration_ms` 等**与数据库设计书 v2.0 §4 schema 不一致的字段**（DB 设计书 v2.0 §4.3 写 `translation_memory`，不是 `tm_segments`；§4.2 写 `projects.org_id`，不是 `tenant_id`）

**本文处理**：
- 本文档（v1.0）**完整覆盖并替换** v1.1 既有内容
- v1.1 既有内容**不删 git 历史**，可通过 `git log -p --follow doc/03-详细设计/SQL/CATs_SQL设计一览_v1.0.md` 实证
- Sprint 2 DBA Lead 应升 **v1.2**，把 T-04 v1.0（auth_db+user_db 范围）+ v1.1 历史（7+ 库范围）整合，覆盖完整 8 库

**待 DDD Review 决议**：v1.1 历史内容是否需独立归档为 `CATs_SQL设计一览_历史v1.1.md`（per "不删历史"原则）？还是直接升 v1.2 整合？

### §2 EXPLAIN ANALYZE 验证全 8/8 待 e2e（high）

**现状**：本 worktree `D:/CATs/.worktrees/feat-m1-sprint1-t04-sql` 内无 test fixture（经 `git ls-files crates/auth-service/tests/ crates/user-service/tests/` 验证，仅 5 份 migration 实物 + 源码；T-06 集成测试 ITa 范围尚未实现）。

**本文处理**：8 条关键 SQL 均标 🟡 待 e2e 验证 + 预期索引路径（如 §3.1 / §3.2 / ... 末尾"EXPLAIN 验证状态"列）。

**验证计划**：
- T-06 集成测试 ITa 落地（per 任务拆解 v1.0+2 §2 T-06 200K-400K tokens）
- QA Lead 跑真实 `EXPLAIN ANALYZE` 报告到 `doc/04-测试/集成测试报告/CATs_M1_S1_集成测试报告_v1.0.md`
- 测试数据：1k / 10k / 100k / 1M 行级别（per T-04 完成判据 ② 接受标准 Seq Scan on > 1k 行表 = 失败）

### §3 数据库设计书 v2.0 §4 schema 与 T-01/T-02 实际 migration 冲突（critical）

**现状**：数据库设计书 v2.0+1（commit `cd40911`）§4.1 / §4.2 与 T-01 / T-02 实际落地的 migration 在 7+ 处 schema 细节上不一致：

| # | 冲突点 | DB 设计书 v2.0 §4 | T-01/T-02 实际 migration | 影响 | 处置 |
|---|--------|-------------------|---------------------------|------|------|
| 1 | `users_credential.email` 类型 | `CITEXT NOT NULL UNIQUE` | `TEXT NULL` (0002 增量无 UNIQUE) | 登录查询 SQL 差异 | Sprint 2 升 DB 设计书 v2.1 |
| 2 | `users_credential` 是否含 `org_id` | `org_id UUID NOT NULL` | 不含 | 跨服务 user_id 一致性 | Sprint 2 决议 |
| 3 | `users_credential` 是否含 `status` / `mfa_enabled` | `status TEXT CHECK IN ('active','locked','disabled')` + `mfa_enabled BOOLEAN` | 仅 `is_active BOOLEAN` | RBAC 状态机差异 | Sprint 2 决议 |
| 4 | refresh 设计 | `sessions` 表（refresh_token_hash + expires_at + revoked_at + client_kind）| `refresh_token_revoke` 表（jti + user_id + reason）| 概念模型差异（持久 session vs jti 撤销）| Sprint 2 决议 |
| 5 | audit_log 位置 | `audit_db.audit_logs`（RANGE 月分区 + org_id + actor_user_id + action + resource_type + resource_id + before/after JSONB）| `auth_db.audit_log`（按 user_id + event_type + outcome + detail JSONB）| 跨库审计双写 | 已通过 service-local + 合规持久化双写解决（per §3.8 设计选择）|
| 6 | `users_profile.org_id` | `org_id UUID NOT NULL REFERENCES orgs(id)` | 不含 `org_id`（T-02 时 orgs 未建）| 多租户隔离 | Sprint 2 加 orgs 表 + 同步 |
| 7 | `user_db` 表数 | 5 表（orgs + users_profile + org_members + subscriptions + outbox_event）| 1 表（user_profile）| T-02 仅脚手架范围 | Sprint 2 实战深化时补 4 表 |

**本文处理**：本文档 §2 DDL 集中登记**以 T-01/T-02 实际 migration 为准**（DDL 实物权威），DB 设计书 v2.0 §4 留待 Sprint 2 升 v2.1 同步。

**待 DDD Review 决议**：
- 7 项 schema 冲突由架构师 Lead + DBA Lead 联合升 DB 设计书 v2.1 解决
- 时序：Sprint 1 末（9/27）前先出 DB 设计书 v2.1 草案 → Sprint 2 T-13（数据库详细设计）按 v2.1 实施

### §4 user_db.user_profile 无 role 字段（high）

**现状**：T-02 落地的 user_profile 表不含 `role` 字段（per §2.2.1 字段表），但 T-03 RBAC 矩阵 v1.0 + T-04 §3.7 用户角色查询 SQL 引用 `role` 字段。

**冲突**：Sprint 1 启动会决议 7 提到"user-service schema 决议"留 T-03 处理，但 T-03 范围仅"矩阵文档"（150K-300K tokens），不一定改 user_profile 表 DDL。

**待 T-03 DDD Review 决议**：
- 方案 A：`role` 字段放 `auth_db.users_credential`（per 当前 RBAC 路径）→ user-service 通过 gRPC `AuthCheck` 查 role
- 方案 B：`role` 字段放 `user_db.user_profile`（per T-04 §3.7 SQL 假设）→ 需 user-service migration V002 加 `role TEXT`
- 方案 C：建独立 `auth_db.role_bindings` 表（per DB 设计书 v2.0 §4.1 规划）→ user 多角色支持

### §5 跨库 JOIN / 事务策略未定义（medium）

**现状**：本文档范围限定 auth_db + user_db，跨库场景（如 auth-service 创建用户时同时写 auth_db.users_credential + user_db.user_profile）需分布式事务或最终一致性方案。

**当前处置**：T-02 脚手架无创建用户端点（per 接口设计书 v2.0+1 §3.2 未列），Sprint 1 范围仅 GET / PUT，跨库写入不在 Sprint 1 主路径。

**待 Sprint 2 决议**：
- 方案 A：Saga 编排（架构设计书 §1.2 原则 6 标记当前规模过度设计）
- 方案 B：Outbox + CDC（auth_db 写完 → Debezium 捕获 → Kafka → user-service 消费 → 写 user_db）
- 方案 C：同步 REST 调用（auth-service 写完 users_credential → 调 user-service 写 user_profile，含 idempotency-key）

---

## 9. 关联文档

| 文档 | 路径 | commit hash | 关联章节 |
|------|------|-------------|----------|
| CATs_数据库设计书 v2.0+1 | `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md` | `cd40911` | §4 / §5 / §6 / §7（cross-ref 目标）|
| CATs_接口设计书 v2.0+1 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `0eb1e9f` | §3.1 auth / §3.2 user |
| CATs_模块设计书 v2.0 | `doc/03-详细设计/模块设计/CATs_模块设计书_v2.0.md` | （v2.0 升版中 per 启动会决议 2）| T-07 类图 v1.0 引用本文 §3 关键 SQL |
| CATs_错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f53` | §3 错误码分类 / §4 auth 端点错误码 / §5 审计事件映射 |
| CATs_安全要件定义书 v1.0 | `doc/05-其他/安全/CATs_安全要件定义书_v1.0.md` | `d1b10fe` | §3 认证 / §4 密码策略 / §6 加密 / §7 审计 |
| CATs_M1_Sprint1_任务拆解 v1.0+2 | `doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md` | `efd9e77` | §2 T-04 任务清单 + §5 R-03 风险 + §6.5 token-OLU 立项 |
| CATs_Baseline 一览 v1.0 | `doc/05-其他/管理/CATs_Baseline一览_v1.0.md` | `4f96f95` | §3.3.3 D-D-048（待基线化）|
| CATs_微服务架构设计书 v1.0+1 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §4.1 核心 8 MVP 服务 / §5.1 8 逻辑库 |
| CATs_技术基线 v1.0+2 | `doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md` | `047dc9c` | §1 PG 18.6 + pgvector 0.8.6 |
| CATs_token-OLU 框架 v0.1 | `doc/05-其他/管理/CATs_token-OLU框架_v0.1.md` | `f6772ce` | §3.1 DBA Lead 200K-350K tokens |
| CATs_WBS Sprint 1 跟踪 v1.0 | `doc/05-其他/管理/CATs_WBS_Sprint1_跟踪_v1.0.md` | `49fdbb3` | §2.2 WBS 编码 PMO.S1.T04 |
| CATs_RACISLA 模板 v1.0 | `doc/05-其他/管理/模板/CATs_RACISLA模板_v1.0.md` | `efd9e77` | 24h Consulted SLA |

---

## 10. 修订履历（详）

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-01 | DBA Lead（Mavis 接手 agent per DEC-008）| 初版：M1-Sprint 1 T-04 范围（auth_db + user_db 4 表）+ 8 条关键 SQL + EXPLAIN 验证占位 + 已知缺口 5 项 |

---

**文档结束**

> **Sprint 1 状态**：本文为 T-04 完成判据 ① / ③ 部分提交；判据 ② EXPLAIN 验证 8/8 待 e2e（T-06 范围）。
> **DDD Review 周期**：per 启动会决议 4，6 角色 7 天内评审（9/2 - 9/8 JST）。
> **下一版（v1.2）计划**：Sprint 2 DBA Lead 整合 T-04 v1.0（auth_db+user_db）+ v1.1 历史（7+ 库范围），覆盖完整 8 库。
