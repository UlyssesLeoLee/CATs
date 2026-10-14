# CATs MVP Backend 落地范围 / 已知缺口 / Sprint 末续做清单

**版本**: v0.1
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008) per 8/27 19:39 守门

---

## §1 本次实际落地(`feat/mvp-backend` 5 commit)

| commit | 内容 |
|---|---|
| `4b893b3` | `feat(common)`: 统一 `CatsError` + `ErrorCode` 30 个 + HTTP/gRPC 双向映射 (per 接口设计 v2.0+2 §1.4) |
| `ecca464` | `feat(common, rbac, project)`: MVP 后端基础设施 + project-service CRUD 落地 |
| `e7b9dae` | `feat(task, file)`: MVP 任务编排 + 文件落盘 |
| `d1e52c1` | `feat(notification, report)`: notification Kafka consumer + report-service 跨库只读 |

## §2 已就绪服务(MVP 商业版最小业务流能跑)

- ✅ `cats-common`: `CatsError` + `ErrorCode` 30 个 snake_case enum
- ✅ `cats-project-service`: CRUD(列表/创建/获取/更新成员)+ RBAC + 状态机
- ✅ `cats-task-service`: 创建/获取任务 + 状态机(pending/in_progress/completed/qa_blocked)
- ✅ `cats-file-service`: 上传 + 下载(本地落盘 `var/files/{org_id}/{file_id}` per MVP 简化)
- ✅ `cats-notification-service`: Kafka consumer + 写本地 log(MVP 简化)
- ✅ `cats-report-service`: 跨 `task_db` 只读用量统计
- ✅ `auth-service` + `user-service`: Sprint 1 已落地(原 commit efd9e77)

## §3 未就绪服务(MVP 末仍需续做)

| # | 服务 | 缺口 | 优先级 |
|---|---|---|---|
| 1 | `audit-service` | 完整 Kafka consumer + REST 查询 | 🔴 P0 |
| 2 | `worker-service` | 周期 worker 调度任务 → translation-core | 🔴 P0 |
| 3 | `translation-core` | gRPC server(`MatchTM` / `TranslateSegment` / `RunQA` / `BatchTranslate` per 现有 proto),AI GW trait,TM/Glossary 数据层 | 🔴 P0 |
| 4 | 集成测试 | 端到端 happy-path 1 条用例(从 login → project → task → translation) | 🟡 P1 |
| 5 | ai-gateway 集成 | worker-service/translation-core 注入 `cats-ai-gateway` provider | 🟡 P1 |

## §4 验证状态

- ✅ `cargo check` for `cats-common`、`cats-project-service`、`cats-task-service`、`cats-file-service`、`cats-notification-service`、`cats-report-service` 0 err(per worker 最后阶段输出)
- ⚠️ 未做 `cargo test` 完整跑(Sprint 末续做)
- ⚠️ 未做 `cargo check --workspace`(会跨入其他 worktree 范围,留给 root session 串联时跑)

## §5 Sprint 末续做清单

- [ ] `audit-service` Kafka consumer + `GET /v1/audit-logs` 分页
- [ ] `worker-service` 周期 task.assign 消费 + 委托 translation-core
- [ ] `translation-core` gRPC server + TM Lookup/glossary Match 实现 + 与 AI GW 接线
- [ ] `cargo test` 完整跑(8 服务 + translation-core + aigw)
- [ ] DDD Review 6 角色复审

---

**承认现状**:本次 Sprint 2 W3-W4 落地 6 核心 service 主体 + 1 公共 crate,**未落地** audit/worker/translation-core + 集成测试。续做清单见 §5。Sprint 2 末由 root session 串联 6 worktree + 真实 `cargo check --workspace` 一次跑通 + DDD Review 闭环。
