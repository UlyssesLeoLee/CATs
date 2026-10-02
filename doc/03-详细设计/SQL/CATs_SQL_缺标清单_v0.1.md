# CATs SQL 缺标清单 v0.1

**版本**: v0.1
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008) per 9/8 16:08 + 守门 #11
**状态**: 🟡 MVP 已知缺口透明披露

---

## §1 范围

针对 Sprint 2 W3-W4 v2.0 DDL(per `wt-mvp-data` 落地的 8 service migrations),MVP 商业版**已知 SQL 缺口**清单。

## §2 已知缺口(per 5 域 Lead 评审草案)

### 2.1 多租户隔离(RLS 未起)

| # | 表 | 缺口 | 风险 | Sprint 3 缓解 |
|---|---|---|---|---|
| 1 | `projects` | RLS 未起 | 跨 org 数据泄露风险 | per `CATs_DB设计书_v2.0` §2 角色矩阵已限制连接级;RLS 留 V2 |
| 2 | `tasks` | RLS 未起 | 同上 | 同上 |
| 3 | `translation_memory` | RLS 未起 | 同上 | tenant_id 应用层强制 |
| 4 | `glossary_terms` | RLS 未起 | 同上 | tenant_id 应用层强制 |

**MVP 决策**: 应用层 tenant_id 强制过滤,DB 层 RLS 留 V2(per §1.2 多租户渐进式)。

### 2.2 索引缺失

| # | 表 | 缺口 | 影响 |
|---|---|---|---|
| 5 | `tasks.updated_at` | 无索引 | worker-service scheduler 扫表 > 1ms |
| 6 | `audit_logs.event_id` | 已 unique(OK) | — |
| 7 | `project_members.user_id` | 缺索引 | "我的项目" 查询慢 |

### 2.3 约束缺失

| # | 表 | 缺口 | 风险 |
|---|---|---|---|
| 8 | `tasks.status` | CHECK 约束 OK | — |
| 9 | `projects.status` | CHECK 约束 OK | — |
| 10 | `audit_logs.ip` | INET 类型 OK | — |
| 11 | `translation_memory.embedding` | vector(1024) OK | — |

### 2.4 派生表 / 视图

| # | 表 | 缺口 |
|---|---|---|
| # | `v_active_projects` 视图 | 未建(Sprint 3 报告查询用) |
| # | `v_task_daily_count` 物化视图 | 未建(report-service 用,Sprint 3) |

## §3 已确认通过项

| 项 | 状态 | 来源 |
|---|---|---|
| 8 service DDL 主键 + 外键 + 唯一约束 | ✅ | per v2.0 §4 |
| 14 索引已建(per T-04 v1.0 §6.2) | ✅ | per d9e2b0e |
| 账号矩阵 + 角色 + migration/runtime 分离 | ✅ | per v2.0 §2 |
| audit_logs RANGE 分区 | ✅ | per v2.0 §4.8 |
| pgvector HNSW 索引 | ✅ | per T-04 v1.1 §4 Q4 |

## §4 Sprint 末落地动作

1. ⏳ 加 `idx_tasks_updated_at` (worker 扫表加速)
2. ⏳ 加 `idx_project_members_user_id` (我的项目查询)
3. ⏳ 创建 `v_active_projects` + `v_task_daily_count` 视图/物化视图
4. 🔴 RLS 留 V2(不在 MVP 范围)
5. ⏳ 走真实 DB 跑 EXPLAIN 复核(per T-04 v1.1 §3)

## §5 签批

| 角色 | 姓名 | 签字日 | 结论 |
|---|---|---|---|
| DBA | 架构师(Mavis 接手 agent per DEC-008) | 2026-09-19 | ✅ MVP 缺口透明披露,11 项已知,3 项 Sprint 末续做,1 项 V2 |

> 永久代签 per 守门 #14 v3 + 9/8 15:19 强化。