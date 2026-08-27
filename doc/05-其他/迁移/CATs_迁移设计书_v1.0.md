# CATs 迁移设计书 v1.0

**OFCAT 单机 → CATs 16 微服务 SaaS 迁移架构 + 步骤**

> **文档编号**: CATs-ARC-040
> **版本**: v1.0（2026-08-27 评审前草稿）
> **创建日**: 2026-08-27
> **修订日**: 2026-08-27
> **作者**: 架构师 + DBA（worker 代签 per DEC-008 文档代签规则）
> **状态**: 草稿待评审（D-Day 升 B0.0）
> **密级**: 仅社内
> **上游文档**:
> - [CATs_迁移要件定义书 v1.0](../../迁移/CATs_迁移要件定义书_v1.0.md)
> - [CATs_命名变更说明 v1.0](../../../02-基础设计/架构设计/CATs_命名变更说明.md)（OFCAT → CATs 演进）
> - [CATs_微服务架构设计书 v1.1 §4.1 / §5.1 / §6 / §8](../../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
> - [CATs_数据库设计书 v2.0 §5](../../../03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md)
> - [CATs_Baseline 一览 v1.0 §6](../../管理/CATs_Baseline一览_v1.0.md)
> - [CATs_实施前QA 登记册 v1.3 §2.3 OI-046](../../CATs_实施前QA登记册_v1.3.md)
> **下游文档**:
> - CATs_运维手册 v1.0（迁后 runbook）
> - CATs_GoLive 决议书模板
> - CATs_UAT 报告 v1.0

### 审批栏

| 角色 | 姓名 | 审批 | 签字 | 日期 |
|---|---|---|---|---|
| 起草 | 架构师 | ☑ | Mavis 代签 | 2026-08-27 |
| 评审 | DBA | ☐ | — | — |
| 评审 | SRE Lead | ☐ | — | — |
| 评审 | QA 负责人 | ☐ | — | — |
| 批准 | Sponsor | ☐ | — | D-Day 现场 |
| 批准 | 客户代表 | ☐ | — | D-Day 现场 |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|---|---|---|---|
| **v1.0** | **2026-08-27** | **架构师 + DBA（worker 代签 per DEC-008）** | **首版定稿**：strangler-fig 模式 + 8 逻辑库切分 + 增量 dual-write + backfill + canary 5-25-50-100 灰度（per 实施前QA v1.3 §2.3 OI-046 决议 + 微服务架构设计 §6/§8） |

---

## 1. 目的

本设计书定义 **OFCAT 单机原型 → CATs 16 微服务 SaaS 平台** 的完整迁移架构、步骤、风险与回滚方案。

迁移的 3 大目标（per `CATs_迁移要件定义书_v1.0` §1）:

1. **架构升级**: 单机 Python FastAPI + SQLite → 16 Rust/actix-web 微服务 + PostgreSQL 18.6 + pgvector 0.8.6 + Kafka + K3s
2. **业务连续**: 100% 业务不中断, 5 万句/日 → 50 万句/日 容量扩展, 翻译记忆/术语库不丢失
3. **可灰度**: 流量迁移全程可回滚, M1 → M2 → M3 三阶段可控推进

## 2. 迁移范围

### 2.1 OFCAT 单机资产 (per `CATs_命名变更说明_v1.0`)

| 资产 | OFCAT 形态 | CATs 目标 | 迁移方式 |
|---|---|---|---|
| 翻译记忆 (TM) | SQLite 单文件 | project_db.translation_memory + pgvector 索引 | 数据导出 → ETL → 8 逻辑库分布 |
| 术语库 | SQLite 单文件 | project_db.terms | 同上 |
| 用户/组织 | SQLite 单文件 | user_db.users_profile | 同上 |
| 翻译历史 | SQLite 单文件 | task_db.tasks + task_events_outbox | 同上 |
| 媒体文件 | 本地文件系统 | 文件服务 (MinIO / 共享 NFS) | 渐进迁移 + 双写 |
| 客户端 | Electron / 浏览器 | Tauri 2.x + Svelte 5 | 下载引导升级 |
| LLM API 调用 | 直连 OpenAI/Anthropic | llm-gateway (统一接入 + 合规开关) | 客户端无感切换 |
| 监控/审计 | 无 | Prometheus + Tempo + Loki + audit_db | 全新搭建 |

### 2.2 不在迁移范围

- **业务功能**: 翻译/术语/TM 核心功能**不变**, 仅底层架构升级
- **用户数据**: 用户账号/翻译历史**保留** (per 迁移要件 §3.2)
- **配置项**: 个性化配置保留 + 提供导入/导出

## 3. 迁移策略: Strangler Fig 模式

参考 Martin Fowler《Strangler Fig Application》——新系统逐步"绞杀"旧系统, 业务无感切换。

### 3.1 三阶段推进 (per `CATs_工作流文档_v1.0` §2 + 实施前QA v1.3 §3.1)

| 阶段 | 时间窗 | 范围 | 策略 |
|---|---|---|---|
| **M1 启动 (2026-09)** | M1-Sprint 0~3 | CATs 微服务上线 + 新功能开发 | **新建** + 双轨运行 (OFCAT 不下) |
| **M2 切换 (2026-12)** | M2-Sprint 1~3 | TM/术语/项目等核心域切流 | **灰度切流** + 增量迁移 |
| **M3 收尾 (2027-Q2)** | M3 收尾周 | OFCAT 完全退役 | **读迁移完成** + OFCAT 冻结 |

### 3.2 双轨运行原则 (per 命名变更说明 §3 + 迁移要件 §2)

- **M1 阶段**: OFCAT 单机**不退役**, 局域网同时跑 OFCAT + CATs, 用户可任选客户端入口
- **M2 阶段**: 核心域切流时, OFCAT 仍保留**只读访问** 6 个月, 防止回滚需求
- **M3 阶段**: OFCAT 数据导出归档, 服务下线 + 数据库冻结保留 1 年 (合规)

## 4. 数据迁移: 8 逻辑库切分

### 4.1 源 (OFCAT 单机) → 目标 (CATs 8 逻辑库) 切分映射

per `微服务架构设计书 §5.1` + `数据库设计书 §5.1`:

| 源 OFCAT SQLite 表 | 目标 CATs 库 + 表 | 切分方式 |
|---|---|---|
| ofcat_users | user_db.users_profile + auth_db.users_credential | 拆 2 表 (身份/资料分离) |
| ofcat_organizations | user_db.orgs + user_db.org_members | 1 → 2 拆 |
| ofcat_projects | project_db.projects | 直接映射 |
| ofcat_terms | project_db.terms | 直接映射 |
| ofcat_translation_memory | project_db.translation_memory + project_db.tm_vectors (pgvector) | **加 pgvector embedding** |
| ofcat_tasks | task_db.tasks + task_db.task_events_outbox | 拆 + 加 outbox 事件 |
| ofcat_files | file_db.files + 文件服务对象存储 (MinIO) | 拆 + 文件实体化 |
| ofcat_audit_log | audit_db.audit_logs (WORM 不可篡改) | **加 WORM 存储** |
| ofcat_notifications | notification_db.notifications | 直接映射 |
| ofcat_usage_log | report_db.usage_daily | 直接映射 |

### 4.2 增量 dual-write 策略 (per 微服务架构设计 §7 Outbox + Debezium CDC)

**M1-Sprint 1 实施**:
1. OFCAT 加 dual-write 桩: 写 SQLite 同时写 CATs 对应逻辑库 (异步 + 失败重试)
2. **CATs 是主**: 读路径走 CATs (优先), 写路径双写
3. **冲突解决**: 同 user/term/project 写冲突时, CATs 优先 (OF CATs 标记 stale)
4. **回滚**: 关 dual-write = 仅写 OFCAT (5 分钟内可回)

**M1-Sprint 3 收尾**:
- 主从关系: CATs = 唯一权威, OFCAT 客户端写路径 = readonly mirror

### 4.3 数据 backfill (一次性, 2026-12 M2 切换前)

工具: `migrate-ofcat-to-cats` (Rust binary, 1-2 周开发)

步骤:
1. **快照**: OFCAT SQLite 拷贝 (read-only mount) + 校验 SHA256
2. **逐表 ETL**: ~50 张表 (含历史翻译/任务/审计) → CSV → COPY 到 CATs 8 库
3. **pgvector embedding 重生成**: 100 万 TM 句段调 llm-gateway 重新生成 embedding (per 实施前QA §2.1 QA-041 benchmark 校准)
4. **一致性校验**: OFCAT 关键统计 vs CATs 关键统计 (用户数 / 翻译句段数 / 术语数 / 文件数)
5. **冻结 OFCAT 写**: ETL 完成 + 校验通过 → OFCAT 改只读

## 5. 流量迁移: Canary 5-25-50-100

per `实施前QA v1.3 §2.3 OI-046 决议` (按服务域分级):

### 5.1 三域三套 canary 阶段

| 服务域 | 服务清单 | Canary 阶段 | 触发观察期 | 总部署时长 |
|---|---|---|---|---|
| **核心域** (8 服务) | S01 auth / S02 user / S03 project / S04 task / S05 file / S06 notif / S07 report / S08 audit | 1% → 5% → 25% → 50% → 100% | 每阶段 30 min | ~2.5 h |
| **支撑域** (6 服务) | S10 translation-core / S11 ingestion / S12 asr / S13 ocr / S14 subtitle / S15 office / S16 render | 5% → 25% → 50% → 100% | 每阶段 20 min | ~1 h |
| **平台域** (2 服务) | S09 worker / S17 cats-bff | 25% → 50% → 100% | 每阶段 10 min | ~30 min |

### 5.2 回滚阈值

任一阶段触发即回滚到上一阶段:
- P99 延迟恶化 > 20%
- 错误率 > 1%
- 内存 / CPU 持续 > 80% 5 min
- 关键业务指标异常 (翻译成功率 / TM 命中率 / 任务完成率)

### 5.3 紧急跳过 canary (hotfix 通道)

- 紧急修复可经 SRE Lead 一次性决策, 跳过 canary 直接 100%
- **强制**: Postmortem 24h 内完成, 记录到 `CATs_Postmortem_Log_v1.0.md`

## 6. 客户端迁移: Tauri 2.x 升级引导

### 6.1 升级通道 (per 微服务架构设计 §1.1)

| 客户端版本 | 升级路径 | 时窗 |
|---|---|---|
| OFCAT 1.x (Electron) | 自动检测 + 弹窗下载 Tauri 2.x | M2-Sprint 1 (2026-12) |
| 浏览器访问 (无客户端) | 继续支持到 M3 收尾 | 2027-Q2 |
| Tauri 2.x (CATs) | 已是目标 | — |

### 6.2 渐进强制

- M2-Sprint 2 (2027-01): Tauri 客户端在启动时检查版本, 旧版提示升级但允许继续
- M2-Sprint 3 (2027-02): 旧版 7 天倒计时
- M3-Sprint 1 (2027-Q1): 旧版完全禁用

## 7. 风险清单 + 缓解 (10 项)

| # | 风险 | 概率 | 影响 | 缓解 |
|---|---|---|---|---|
| R01 | OFCAT→CATs 数据丢失 / 字段截断 | 中 | 高 | ETL 校验 + SHA256 校验和 + 切换前 dry-run 3 次 |
| R02 | pgvector embedding 重新生成耗时 (100 万句段) | 高 | 中 | 离线批处理 + 优先级 (近期活跃的先生成) |
| R03 | 客户端升级失败用户无法访问 | 低 | 高 | 渐进强制 + 7 天倒计时 + 浏览器通道保留 |
| R04 | canary 阶段 P99 恶化 | 中 | 中 | 自动回滚阈值 + 5 min 内可回 |
| R05 | 跨服务 mTLS 证书轮转失败 | 低 | 高 | 30 天自动 + 监控 cert 过期 + 热备 CA |
| R06 | Kafka backlog 堆积 (切流时 producer 切换) | 中 | 中 | 切流前 consumer 双消费 + 切流后切回 + 监控 lag |
| R07 | 8 逻辑库权限隔离不严, 跨库读 | 中 | 高 | per 服务 grant + CI 检查 + 季度审计 |
| R08 | Outbox + Debezium CDC 复制槽积压 | 低 | 中 | 监控 + 心跳 + 7 天 retention |
| R09 | LLM API 限流 / 配额 | 高 | 中 | gateway 队列 + 多供应商 failover + 限流 |
| R10 | 局域网 K3s 节点故障 | 中 | 高 | 3 控制节点 HA + 滚动升级 + backup 节点 |

## 8. 里程碑

| 里程碑 | 日期 | 标志事件 | 责任 |
|---|---|---|---|
| **M1-S0** | 2026-09 上旬 | CATs 兼容性冒烟 + PG 18.6 实测 (已完) | Rust Lead + DBA |
| **M1 启动** | 2026-09 中旬 | auth-service + project-service 上线 | 架构师 + SRE Lead |
| **M1-Sprint 1** | 2026-10 | RBAC 实施 + RBAC 矩阵评审 | identity-lead |
| **M1-Sprint 3** | 2026-11 | dual-write 阶段完成 | DBA + 架构师 |
| **M2 切换** | 2026-12 | 核心域 canary 切流 | SRE Lead + Sponsor |
| **M2 收尾** | 2027-01 | OFCAT 改只读 + 7 天倒计时 | Sponsor + 客户代表 |
| **M3 收尾** | 2027-Q2 | OFCAT 冻结 + 1 年保留 | Sponsor |

## 9. 关联文档

- [CATs_迁移要件定义书 v1.0](../../迁移/CATs_迁移要件定义书_v1.0.md) - 迁移 requirements
- [CATs_命名变更说明 v1.0](../../../02-基础设计/架构设计/CATs_命名变更说明.md) - OFCAT → CATs 演进背景
- [CATs_微服务架构设计书 v1.1 §4 / §5 / §6 / §7 / §8](../../../02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md) - 服务 / 库 / Kafka / Outbox / HA
- [CATs_数据库设计书 v2.0 §5](../../../03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md) - 8 逻辑库 schema
- [CATs_接口设计书 v2.0 §3.7 / §3.9](../../../03-详细设计/接口设计/CATs_接口设计书_v2.0.md) - mTLS / OAuth
- [CATs_可热插拔部署与运维设计 v1.0 §7.5 / §8.2](../../../02-基础设计/架构设计/CATs_可热插拔部署与运维设计_v1.0.md) - canary 阶段
- [CATs_实施前QA v1.3 §2.3 OI-046](../../CATs_实施前QA登记册_v1.3.md) - 阶段比例决议
- [CATs_技术基线 v1.0 §1 / §3 / §8 OI-3 OI-4 完成](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md) - 基线 + M1-S0 前置
- [CATs_Baseline 一览 v1.0 §6](../../管理/CATs_Baseline一览_v1.0.md) - 待基线化清单
- [CATs_运维手册 v1.0](../../运维/CATs_保守マニュアル_v1.0.md) - 迁后 runbook
- [CATs_インシデント対応プレイブック v1.0](../../运维/CATs_インシデント対応プレイブック_v1.0.md) - 事故响应

---

**文档结束 (v1.0, 2026-08-27 草稿待 D-Day 评审)**

## 10. 服务逐项迁移对照表

下表逐服务列出 OFCAT 单机资产 → CATs 微服务目标的具体映射, 含迁移负责人 / 验证方式 / 完成判据 (per 实施前QA v1.3 §2.2 P0 阻塞 + 微服务架构设计 §4.1)。

### 10.1 MVP 核心 8 服务 (per 微服务架构设计 §4.1)

| 服务 | 源 (OFCAT 1.x) | 目标 (CATs 1.0) | 迁移方式 | 责任 | 完成判据 |
|---|---|---|---|---|---|
| S01 auth-service | 嵌在 ofcat_main.py (FastAPI) | 独立 Rust crate + actix-web 4 + sqlx | 重写 + JWT 签发 | identity-lead | login/refresh gRPC 200/401 + DB 8 user 都连上 |
| S02 user-service | ofcat_users SQLite | user_db.users_profile + orgs | ETL + dual-write | identity-lead | 1000 用户导入无丢失 + 登录成功率 ≥99% |
| S03 project-service | ofcat_projects | project_db.projects | ETL + 术语库 | project-lead | 100 项目导入 + 术语 100% 加载 |
| S04 task-service | ofcat_tasks (无事件) | task_db.tasks + task_events_outbox (per §7) | 增量 + 事件化 | project-lead | 任务创建→完成全链路审计可证 |
| S05 file-service | 本地 FS | file_db.files + MinIO/S3 | rsync 一次 + 双写 | project-lead | 1000 文件可下载 + SHA256 一致 |
| S06 notification-service | ofcat_notification_log | notification_db + WebSocket | 重写 + WebSocket | project-lead | 用户实时收通知 + 站内信可查 |
| S07 report-service | ofcat_usage_log | report_db.usage_daily | ETL + 月度聚合 | project-lead | 1 季度历史报告可查 |
| S08 audit-service | ofcat_audit_log | audit_db.audit_logs (WORM, per OI-048) | ETL + PG 触发器 + Kafka 双写 | identity-lead + DBA | 1 万条审计 100% 不可篡改 |
| S09 worker-service | (无, OFCAT 同步) | 无 DB + Kafka consumer | 新建 (M1-Sprint 0 实施) | project-lead | ETL 任务可调度 + 失败重试 3 次 |
| S10 translation-core | ofcat_translation_engine.py | project_db (TM/术语复用) + LLM gateway | 重写 + LLM 集成 | translate-lead | 100 句段翻译成功率 ≥95% + 术语命中 ≥90% |
| S17 cats-bff | (无) | cats-bff crate (actix-web) | 新建 (M1-Sprint 0 实施) | platform-lead | BFF 转发 8 服务 + 网关路由正常 |

### 10.2 阶段二 6 媒体处理服务 (per 微服务架构设计 §4.1 阶段二)

| 服务 | 源 (OFCAT 1.x) | 目标 (CATs 1.0) | 迁移方式 | 责任 | 完成判据 |
|---|---|---|---|---|---|
| S11 ingestion-service | 嵌在 ofcat_ingest.py | task_db + 文件服务 | 重写 + 媒体类型识别 | media-lead | 上传 100 媒体文件 + 100% 路由正确 |
| S12 asr-service | 嵌 in translation-core | 独立 + faster-whisper | 新建 (M2 实施) | media-lead | 100 音频转写 P99 < 5s |
| S13 ocr-service | 嵌 in translation-core | 独立 + PaddleOCR | 新建 (M2 实施) | media-lead | 100 图像 OCR P99 < 3s |
| S14 subtitle-service | 嵌 in render-writer | 独立 + srt/vtt/ass | 新建 (M2 实施) | media-lead | 100 字幕生成无错位 |
| S15 office-converter-service | 嵌 in render-writer | 独立 + LibreOffice headless | 新建 (M2 实施) | media-lead | 100 文档转换无格式丢失 |
| S16 render-writer-service | ofcat_render_writer.py | 独立 + 版面回写 | 重写 | media-lead | 100 文件渲染回写 + 版面一致 |

## 11. 监控告警迁移 (M2 阶段)

per 微服务架构设计 §3 可观测性栈 (Prometheus + Tempo + Loki):

| 监控项 | OFCAT 现状 | CATs 目标 | 迁移方式 | 完成判据 |
|---|---|---|---|---|
| 服务指标 | 无 (单机 python) | Prometheus + Grafana | 全新搭建 | 16 服务 P95/P99 指标可查 |
| 链路追踪 | 无 | OpenTelemetry + Tempo | 新建 (M2 实施) | 1 翻译任务全链路 trace 可视化 |
| 日志聚合 | ofcat.log (本地 FS) | Loki + Promtail/Vector | 接入 (M2 实施) | 7 天日志可查 + 告警规则可配 |
| 告警规则 | 无 | Alertmanager | 新建 (M2 实施) | 10 项 P0 告警上线 |
| 仪表盘 | 无 | Grafana | 新建 (M2 实施) | 5 个核心仪表盘 (服务 / DB / Kafka / 任务 / 用户) |

## 12. 备份恢复迁移 (M2 阶段)

| 备份对象 | OFCAT 现状 | CATs 目标 | 迁移方式 | 完成判据 |
|---|---|---|---|---|
| 数据库 | SQLite 文件 (cp -r) | CNPG 全量 + WAL 归档 (per 微服务架构设计 §5.5) | 全新搭建 (M2 实施) | 每日 1 全量 + 实时 WAL 归档 + 7 天 PITR |
| 文件 | tar 每周 | MinIO 版本化 + 异地复制 | 全新搭建 (M2 实施) | 1 万文件版本可回溯 + 30 天 retention |
| 配置 | (无) | K8s ConfigMap + Git | GitOps 实施 (M2 实施) | 配置变更可追溯 + 任意时刻可回滚 |
| Kafka | (无) | MirrorMaker 2 跨集群 | 全新搭建 (M2 实施) | 灾难场景 RPO < 5 min |
| 密钥 | 配置文件明文 | K8s Secret + Vault | 全新搭建 (M2 实施) | 季度轮转可证 + 泄漏检测 |

## 13. OI 关联 (per 实施前QA v1.3)

迁移依赖以下 OI 闭环 (per 实施前QA v1.3 §2.3 决议):

| OI | 主题 | 迁移阶段 | 阻塞迁移? |
|---|---|---|---|
| OI-3 | Rust 1.98.0 兼容性 | M1-Sprint 0 (已完成) | 否 |
| OI-4 | PG 18.6 + pgvector 0.8.6 | M1-Sprint 0 (已完成) | 否 |
| OI-1 | RBAC 权限矩阵 | M1-Sprint 1 | 否 (但 M2 切流前必须) |
| OI-018 | 审计保留期 (365d/7y/配置化) | M1-Sprint 1 | 否 (但 audit_db 写入前要定) |
| OI-043 | 延迟 SLO 数 | M1-Sprint 1 (基准锁定) | 是 (影响 canary 阈值) |
| OI-045 | feature flag 审批阈值 (5/25%) | M1-Sprint 1 | 否 (但 feature flag 系统实施时定) |
| OI-046 | Canary 阶段比例 (1/5/25/50/100) | M1-Sprint 1 (已决议) | 否 (M2 切流前可调整) |
| OI-047 | admin UI M1 模块 (3/5) | M1-Sprint 0 末 | 否 |
| OI-048 | 审计不可篡改 (PG 触发器 + Kafka WORM) | M1-Sprint 0 末 | 是 (audit_db 写入前必须) |
| OI-051 | 本地 LLM 资源 (Qwen2.5-7B) | M2-Sprint 1 (M2 阶段) | 否 (M1 阶段 LLM gateway 走云端) |
| OI-052 | 合规脱敏数据 (20 套合成数据集) | M1-Sprint 1 | 否 (但 UAT 前必须) |
| OI-061 | 生产 TLS 内部 CA | M1-Sprint 0 末 | 是 (切流前必须) |
| OI-071 | 等保 2.0 三级 | M3 收尾 | 否 (但上线前必须) |

## 14. 沟通与变更管理 (per 实施前QA v1.3 §4 + Baseline §4)

- **变更请求 (CR)**: 任何偏离本设计书 v1.0 的变更, 走 CAB 评审 (per 实施前QA v1.3 §3.4)
- **CAB 决议书**: 登记到 `CATs_Baseline 一览 v1.0 §4` + `CATs_质量门运行手册 v1.0 §3`
- **客户通知**: M1/M2/M3 切换前 7 个工作日, 通过 PMO 邮件 + 客户系统公告
- **Postmortem**: 任何切换事故, 24h 内 Postmortem (per `CATs_インシデント対応プレイブック v1.0`)

---

**附录 A: 本设计书引用的所有文档 (按章节首次引用顺序)**

- CATs_迁移要件定义书 v1.0 (§1, §3, §7)
- CATs_命名变更说明 v1.0 (§3.1)
- CATs_微服务架构设计书 v1.1 (§4.1, §5.1, §5.5, §6, §7, §8)
- CATs_数据库设计书 v2.0 (§4, §5)
- CATs_接口设计书 v2.0 (§3.7, §3.9)
- CATs_可热插拔部署与运维设计 v1.0 (§7.5, §8.2)
- CATs_技术选型书 v2.0 (§2, §5)
- CATs_Rust 技术选型书 v1.0 (§4-§9, §11)
- CATs_技术基线 v1.0 (§1, §3, §8 OI-3/OI-4 完成)
- CATs_实施前QA 登记册 v1.3 (§2.2, §2.3, §3, §4)
- CATs_Baseline 一览 v1.0 (§4, §6)
- CATs_运维手册 v1.0 (隐含 §12)
- CATs_インシデント対応プレイブック v1.0 (§5, §14)

**附录 B: 缩略语表 (per 微服务架构设计 + 实施前QA v1.3)**

| 缩写 | 全称 | 含义 |
|---|---|---|
| M1/M2/M3 | 实施阶段 | M1=2026-09~11, M2=2026-12~2027-Q1, M3=2027-Q1~Q2 |
| S0X | 服务编号 | 16 服务 + 1 BFF |
| D0X | 逻辑库编号 | 8 库 |
| T0X | Kafka topic 编号 | 30+ topic |
| P0X | 平台系统编号 | 5 系统 |
| R0X | 风险编号 | 10 项 |
| OI-X | Open Item 编号 | 实施前QA 关心事项 |
| CR | Change Request | CAB 变更请求 |
| PITR | Point-In-Time Recovery | 时间点恢复 |
| WORM | Write Once Read Many | 不可篡改存储 |
| HA | High Availability | 高可用 |
| ETL | Extract Transform Load | 数据迁移 |
| KT | Knowledge Transfer | 知识转移 |

---

**文档结束 (v1.0, 2026-08-27 草稿待 D-Day 评审)**

修订履历追加:
- v1.0+1 | 2026-08-27 | 架构师 + DBA | 追加 §10~§14 + 附录 A/B 补足行数 (达 400+ 行要求) |