# CATs 功能点现状审计 v1.0

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-FEATURE-AUDIT-v1.0 |
| 日期 | 2026-09-20 |
| 作者 | Mavis（架构师 Lead 代签 per DEC-008） |
| 触发 | ULYS-125 功能点罗列（parent 委托调查） |
| 审计基线 | `agent/minimaxm3/ce1e3d60f6b2` @ 28f0afb（ULYS-110 Vercel 部署套件） |
| 引用 | V1.1-PLAN.md §0.3 §2 + BACKEND_STATUS_v0.1 + apps/cats-client/README.md |

---

## §1 调查方法

1. 全仓 `wc -l crates/*/src/*.rs` + `apps/cats-client/src/**/*.svelte` 量化代码深度
2. `grep TODO\|FIXME\|MVP 阶段未实现\|per apps/cats-client/TODO.md` 抓已知缺口
3. 对照 `api/openapi/cats-openapi-v1.0.1.yaml` §paths 业务 endpoint 清单
4. 对照 `doc/01-需求/需求规格说明/OFCAT_需求定义书_v1.1.md` §5.1 F1-F11 MVP/非MVP 划分
5. 读 `deploy/BACKEND_STATUS_v0.1.md` 知晓 binary 编译卡 rustc 1.98 metadata bug

## §2 当前版本功能全景（per 代码实证）

### §2.1 客户端 apps/cats-client (Tauri 2.x + Svelte 5)

**总代码量**: 1283 行 / 16 文件（src-tauri）+ 624 行 Svelte/TS

| 模块 | 文件 | 行数 | 已支持 |
|---|---|---|---|
| 登录页 | routes/LoginPage.svelte | 119 | 用户名/密码登录 + JWT 接入 |
| 项目列表 | routes/ProjectsPage.svelte | 286 | 项目列表 + 创建 + 离线入队 + 同步 |
| 翻译页 | routes/TranslatePage.svelte | 219 | TM lookup UI（"MVP 阶段未实现" 实翻译段） |
| Hash 路由 | lib/router.svelte.ts | 14 | `#/login` `#/projects` `#/translate` 三路由 |
| API 封装 | lib/api.ts | 78 | 8 Tauri command 类型化 |
| Rust API 客户端 | src-tauri/src/api/{auth,client,projects,translate,error}.rs | 430 | JWT 注入 + reqwest + 错误信封 |
| Tauri command | src-tauri/src/commands/*.rs | 360 | 8 #[command] 绑定 |
| 离线 SQLite | src-tauri/src/offline/{db,schema.sql}.rs | 218 | 本地缓存 + 离线队列 |
| 应用状态 | src-tauri/src/state.rs | 122 | token + DB 连接 + 在线/离线 |

### §2.2 后端 16 服务代码深度（per wc -l）

| 服务 | main.rs | lib.rs | 其他 | 总行 | 业务实现 |
|---|---|---|---|---|---|
| **auth-service** | 81 | 57 | handlers.rs + audit.rs + tests | ~350 | ✅ 真实：JWT + 种子用户 + audit log |
| **user-service** | 54 | 55 | handlers.rs 229 + models.rs 120 + db.rs 129 | ~587 | ✅ 真实：CRUD 3 endpoint |
| **task-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **project-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **file-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **notification-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **report-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **audit-service** | 51 | 41 | consumer.rs + db.rs | ~200 | 🟡 partial：process_event 函数有，consumer loop 是 30s heartbeat stub |
| **worker-service** | 51 | 41 | handlers.rs 61 + scheduler.rs 83 + state.rs 23 | ~250 | 🟡 partial：scheduler 函数有，无 binding |
| **ingestion-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **asr-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **ocr-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **subtitle-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **office-converter-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **render-writer-service** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位 |
| **translation-core** | 51 | 41 | service.rs 172 + db.rs 145 + qa.rs 103 + ai_gateway.rs 120 + tm.rs/glossary.rs | ~700 | ✅ 真实：TM 匹配 + 术语 + QA + AI GW 客户端 |
| **cats-ai-gateway** | 165 | 82 | provider/ + router + quota + retry + compliance + service | ~1000 | ✅ 真实：4 mock provider + 配额 + 重试 + 合规 |
| **cats-bff** | 51 | 41 | — | 92 | ❌ 仅 /healthz 占位（计划声称 6 endpoints **未落地**） |
| **cats-rbac** | — | 522 | — | 522 | ✅ 真实：16 域 RBAC 中间件 |
| **cats-common** | — | 97 | — | 97 | ✅ 共享库 |

### §2.3 BFF（计划声称 vs 代码实证）

| 来源 | 声称 |
|---|---|
| V1.1-PLAN §0.3 wt-mvp-app | "Tauri 2.x 客户端壳 + BFF 6 endpoints + offline SQLite" |
| BACKEND_STATUS §1 §4 | "MVP 9/9 ✅ 但 binary 编译未通过" |
| **代码实证** | `crates/cats-bff/src/main.rs` 仅 51 行 + `route("/healthz", ...)` 1 endpoint |

**结论**: V1.1-PLAN 与 BACKEND_STATUS 报告口径与实际代码不一致。30 commit Sprint 2 W3-W4 中"5 sprint-100 crates 复制到 main"（commit 286ed8a）确实把代码塞进 main 了，但**cats-bff 6 endpoints 不在这 5 crate 里**——它仍在 `feat/mvp-app` worktree 未合 main。

## §3 计划内 vs 已实现对照（per OFCAT F1-F11）

| F# | 功能 | 阶段 | 状态 | 落地位置 |
|---|---|---|---|---|
| F1 | 选区捕获 | MVP | ❌ 未实现 | — |
| F2 | TM 匹配 | MVP | 🟡 partial | translation-core/service.rs `tm_exact_match` `tm_fuzzy_match` |
| F3 | 术语匹配与注入 | MVP | 🟡 partial | translation-core/db.rs `glossary_match` |
| F4 | 术语强制校验 | MVP | 🟡 partial | translation-core/qa.rs |
| F5 | 标签/占位符保护 | MVP | ❌ 未实现 | — |
| F6 | 单模型流式翻译 | MVP | 🟡 partial | cats-ai-gateway（mock provider） |
| F7 | 行内 overlay 编辑 | MVP | 🟡 partial | TranslatePage.svelte UI 有但实翻译未接 |
| F8 | 写回页面 | MVP | ❌ 未实现 | — |
| F9 | 保存进 TM | MVP | 🟡 partial | translation-core/db.rs `tm_update` |
| F10 | 合规路由 | MVP | ✅ partial | cats-ai-gateway/compliance.rs |
| F11 | 存量数据导入 | M2 | ❌ 未实现 | — |
| — | 项目管理 | MVP | ❌ 后端 stub | project-service 仅 /healthz |
| — | 任务调度 | MVP | ❌ 后端 stub | task-service 仅 /healthz |
| — | 文件存取 | MVP | ❌ 后端 stub | file-service 仅 /healthz |
| — | 通知 | MVP | ❌ 后端 stub | notification-service 仅 /healthz |
| — | 报表 | MVP | ❌ 后端 stub | report-service 仅 /healthz |
| — | 审计 | MVP | 🟡 partial | audit-service 有 process_event，consumer loop stub |
| — | 多媒体 6 服务（ingest/asr/ocr/subtitle/office/render） | M2 | ❌ 全部 stub | 仅 /healthz |
| — | BFF 6 endpoints | MVP | ❌ 未落地 | cats-bff 仅 /healthz |

**MVP 闭环率**: ~30%（核心组件有代码但未串通；后端 12/16 域 stub；BFF 缺业务 endpoint）

## §4 未开发完的功能（按可并行切片划分）

### §4.1 切片 A：BFF 业务 endpoint 串通（依赖最小、最高优先级）

**目标**: 把 cats-bff 从 /healthz 扩到 OpenAPI v1.0.1 §paths 8 endpoint 全部落地

| endpoint | 上游服务 | 状态 |
|---|---|---|
| POST /auth/login | auth-service | ❌ BFF 缺 |
| POST /auth/refresh | auth-service | ❌ BFF 缺 |
| POST /auth/logout | auth-service | ❌ BFF 缺 |
| GET /auth/me | auth-service + user-service | ❌ BFF 缺 |
| GET /projects | project-service | ❌ BFF 缺 |
| POST /projects | project-service | ❌ BFF 缺 |
| POST /tasks | task-service + translation-core | ❌ BFF 缺 |
| GET /healthz | 本地 | ✅ |

**复杂度**: 中。需要 reqwest 客户端 + JWT 转发 + 错误信封统一。

### §4.2 切片 B：后端核心 4 服务业务实现（独立、可并行）

| 服务 | 缺失 endpoint | 复用 |
|---|---|---|
| **project-service** | POST/GET/PATCH/DELETE /v1/projects | user-service/db.rs + cats-rbac |
| **task-service** | POST /v1/tasks + SSE /v1/tasks/{id}/events | translation-core/db.rs + Kafka stub |
| **file-service** | POST /v1/files (multipart) + GET /v1/files/{id} | sqlx |
| **notification-service** | GET /v1/notifications + WS /v1/notifications/ws | sqlx |

**复杂度**: 中。每个服务 main.rs + handlers.rs + db.rs + models.rs + Cargo.toml 依赖。

### §4.3 切片 C：辅助 2 服务（独立、可并行）

| 服务 | 缺失 endpoint |
|---|---|
| **report-service** | GET /v1/reports/{type}（基于 audit-service 表聚合） |
| **audit-service** | GET /v1/audit（查询）+ 真实 Kafka consumer（替换 30s heartbeat stub） |

### §4.4 切片 D：客户端 UI 补完（依赖切片 B 完成后的 backend API）

| 页面 | 缺失 |
|---|---|
| TranslatePage.svelte | 真实翻译段落编辑 + 写回 + 标签保护 UI |
| 新页 GlossaryPage.svelte | 术语条目浏览 + 新增 |
| 新页 TmPage.svelte | TM 条目浏览 + 100%/模糊匹配可视化 |
| 新页 TasksPage.svelte | 任务列表 + SSE 进度 |

### §4.5 切片 E：多媒体 6 服务（最低优先级 / M2 范畴）

ingestion / asr / ocr / subtitle / office-converter / render-writer 全部 stub，建议留 M2。

## §5 建议派子代理并行方案

| 子代理 | 切片 | 工作量估 | 路径 |
|---|---|---|---|
| sub-1 | A: cats-bff 8 endpoints | 1-2 天 | crates/cats-bff/ |
| sub-2 | B-1: project-service 业务 | 1 天 | crates/project-service/ |
| sub-3 | B-2: task-service + SSE | 1.5 天 | crates/task-service/ |
| sub-4 | B-3: file-service 上传下载 | 1 天 | crates/file-service/ |
| sub-5 | B-4: notification-service + WS | 1 天 | crates/notification-service/ |
| sub-6 | C-1: report-service 聚合查询 | 0.5 天 | crates/report-service/ |
| sub-7 | C-2: audit-service 真实 Kafka consumer | 1 天 | crates/audit-service/ |

**切片 D / E 不在本批子代理范围**——D 等 B 完成后才能调通，E 是 M2。

## §6 风险与约束（per 守门合规 12 维）

1. **rustc 1.98 metadata bug** 仍在，binary 编译无法验证（per BACKEND_STATUS §2）
2. **守门 #1 禁回溯** — 不能从 main 拆已合 commit
3. **守门 #14 v3** — Mavis 临时代签各角色
4. **守门 #9 v19** — git add 走 explicit path，不 -A
5. **cargo 1 次 per L11** — 每个子代理只 cargo check 自己 crate 一次
6. **业务 logic 改 0**（除非本切片明确允许）
7. **8/27 19:39 强化**：代签 author + 审批 + 修订人 3 行齐

## §7 结论

- **已落地**: auth-service / user-service / translation-core / cats-ai-gateway / cats-rbac / apps/cats-client（前端壳）
- **计划内未开发完**: cats-bff 业务 endpoint + 4 核心服务 + 2 辅助服务 = 7 个后端切片
- **M2 范畴**: 6 多媒体服务 + 客户端 UI 深度功能
- **MVP 闭环率**: ~30%（核心组件有代码但未串通；后端 12/16 域 stub；BFF 缺业务 endpoint）
- **建议**: 派 7 子代理并行开发 7 后端切片（切片 A/B/C），子代理各自独立 worktree

— Mavis（架构师 Lead 代签 per DEC-008 + 守门 #14 v3）
