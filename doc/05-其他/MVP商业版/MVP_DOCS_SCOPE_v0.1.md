# CATs MVP 商业版文档 / Mock 收纳已知缺口清单

**版本**: v0.1
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008) per 8/27 19:39 守门

---

## §1 本次实际落地(`feat/mvp-docs` 已 commit)

| commit | 内容 |
|---|---|
| `ea89ca3` | `MVP商业版_产品概述_v1.0.md` |
| `b8450e1` | `MVP商业版_部署架构_v1.0.md` |
| `c039c2d` | `MVP商业版_客户端使用手册_v1.0.md` |
| `38f38ea` | `MVP商业版_管理员手册_v1.0.md` |
| `dc807bc` | `MVP商业版_开发手册_v1.0.md` |

5 篇 doc 全部就位,客户可读。

## §2 本次未能落地(根 worker 断网)

- ❌ `crates/cats-mock/scripts/` 5 个回归脚本(smoke + regression-backend + regression-aigw + regression-app + db-migrate + local-up/down)
- ❌ `crates/cats-mock/mock_data/` 7 个 fixture JSON(tenants/users/projects/tasks/tm_segs/glossary/audit)
- ❌ `crates/cats-mock/docs/MVP_REGRESSION_REPORT_v1.0.md`
- ❌ `crates/cats-mock/docs/MVP_REGRESSION_TEST_MATRIX_v1.0.csv`

**承接**:以上交给 Sprint 2 末 root session 串联所有 worker 输出后**一次性**补齐,或者 worker#5/worker#4 续接。

## §3 验收对照(5 篇 doc 商业 MVP 维度的就绪度)

| 验收项 | 状态 | 来源 |
|---|---|---|
| 客户角度产品概述 | ✅ | `MVP商业版_产品概述_v1.0.md` |
| 部署架构图 | ✅ | `MVP商业版_部署架构_v1.0.md` |
| Tauri 客户端使用手册 | ✅ | `MVP商业版_客户端使用手册_v1.0.md` |
| 管理员手册(项目/术语/审计/告警) | ✅ | `MVP商业版_管理员手册_v1.0.md` |
| 开发手册(monorepo + owner) | ✅ | `MVP商业版_开发手册_v1.0.md` |
| Mock 项目脚本 + fixture + 报告 | ⚠ 续做 | 留 Sprint 2 末 |

**承认现状**:5/8 文档就绪,Mock 项目收纳 0/3,Sprint 2 末续做清单见上。
