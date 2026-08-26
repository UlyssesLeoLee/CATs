# CATs 運用マニュアル v1.0

> **文档编号**：CATs-OPS-RUN-001
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：SRE + 架构师（worker 代签 per DEC-008）
> **状态**：M3 上线前 1 月触发（P2M3待触发索引 v1.0 §3.4 #45）
> **配套**：CATs_可热插拔部署与运维设计 v1.0 / CATs_ADR-001~005

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| 起草 | SRE + 架构师（worker 代签 per DEC-008） | ☑ | 2026-08-26 | v1.0 初版 |
| 评审 | — | ☐ | — | — |
| 批准 | — | ☐ | — | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-26** | **SRE + 架构师（worker 代签 per DEC-008）** | **M3 上线前 1 月触发：日常运维 / 监控告警 / 备份恢复 / 灾备 / 容量 / 变更 / 安全 7 章** |

---

## 0. 适用范围与读者

| 项 | 内容 |
|---|---|
| 适用环境 | PROD（生产）；PRE（预生产）参照执行 |
| 服务范围 | 15 个微服务 + 4 共享库 + K3s 3 控制面 + PG/Kafka/Redis/MinIO（ADR-001 §3 / ADR-002 §2） |
| 读者 | SRE / 值班运维 / 架构师 / 客户技术支持 |
| 不在范围 | 开发环境自运维、单租户自托管模式、客户端桌面端故障 |
| 上游文档 | ADR-001 §3、ADR-003 §3、可热插拔部署与运维设计 v1.0 §10.7 |
| 下游文档 | インシデント対応プレイブック v1.0 / キャパシティ管理計画 v1.0 / 保守マニュアル v1.0 |

---

## 1. 日常运维清单

### 1.1 每日（Daily / 09:00 JST 班前 30 min）

| # | 任务 | 命令/工具 | 期望 | 异常升级 |
|---|------|----------|------|----------|
| D-1 | 全集群 Pod 健康 | `kubectl get pods -A \| grep -v Running` | 0 非 Running | インシデント対応プレイブック §4 |
| D-2 | 服务 SLO 仪表盘 | Grafana 看板 `cats-slo-overview` | 全部绿/黄 | P1 升级 |
| D-3 | 错误率 24h 趋势 | PromQL `sum(rate(http_requests_total{status=~"5.."}[24h]))` | < 0.1% | > 0.5% 立即 P1 |
| D-4 | Kafka consumer lag | Burrow / KEDA 指标 | < 1000 | > 10000 P1 |
| D-5 | PG 主从延迟 | `SELECT now() - pg_last_xact_replay_timestamp()` | < 5s | > 60s P1 |
| D-6 | DLQ 消息数 | `kafka-console-consumer --topic {domain}.dlq --from-beginning` | 0 | > 0 当日处理 |
| D-7 | 备份完成核查 | `barman-cloud check` + Harbor tag 列表 | 全量 02:00 ✓ | 缺失 P1 |
| D-8 | 工单 / 客户 ticket | 客服系统 | 当日 100% 响应 | — |
| D-9 | 安全事件摘要 | SIEM / WAF 日报 | 0 高危 | 任意高危 P0 |
| D-10 | 变更窗口预告 | `git log --since=yesterday --merges` | 与计划匹配 | 计划外变更 P2 |

### 1.2 每周（Weekly / 周一 10:00）

| # | 任务 | 工具 | 输出 |
|---|------|------|------|
| W-1 | 容量评审（CPU/内存/磁盘/QPS） | Grafana / Prometheus | 周报 §容量 |
| W-2 | 镜像不可变标签核查 | Harbor API | 违规清单 0 |
| W-3 | 依赖漏洞扫描 | `cargo audit` / `npm audit` / `pip-audit` | 漏洞清单 |
| W-4 | Keycloak 同步延迟 | Keycloak metrics | < 5 min |
| W-5 | DR 备份抽样恢复演练 | `barman-cloud restore --test` | 1 库 / 周 |
| W-6 | 审计日志完整性 | PG `audit_events` 周新增 + append-only 校验 | 报告 |
| W-7 | 运维 Runbook 反思 | 复盘近 7 日事件 | 改进项 |
| W-8 | 沟通: 周报 + SLO 月度趋势 | 邮件 / Slack | 周一 12:00 前 |

### 1.3 每月（Monthly / 每月 1 日）

| # | 任务 | 工具 | 输出 |
|---|------|------|------|
| M-1 | 容量规划评审 | 见キャパシティ管理計画 v1.0 | 月度趋势报告 |
| M-2 | 灾备 RTO/RPO 演练（部分） | 异地冷备恢复 | 演练报告 |
| M-3 | 密钥/证书轮换检查 | cert-manager + Vault | 90 天内到期清单 |
| M-4 | 架构 / 部署 变更影响回顾 | git log | 影响清单 |
| M-5 | 性能回归 | k6 基线 | 与上月偏差 |
| M-6 | 容量月报（含 token-OLU） | PMO 模板 | 月报 |
| M-7 | Keycloak 多租户隔离审计 | `SELECT schema, nspacl FROM pg_namespace` | 隔离违规 0 |
| M-8 | 数据归档（>90 天 completed task） | worker-service cron | 归档行数 |

### 1.4 季度（Quarterly / 每季度第 1 周）

| # | 任务 | 输出 |
|---|------|------|
| Q-1 | 完整 DR 演练（异地 + RTO/RPO 实测） | 演练报告 + RTO/RPO 复测 |
| Q-2 | 性能压测复测 | 性能試験レポート |
| Q-3 | 安全渗透复测 | セキュリティ試験レポート |
| Q-4 | 容量规划季度评审 | キャパシティ管理計画 v1.x |
| Q-5 | 故障注入全套（含 K3s 控制面 1/3 节点宕机） | 障害試験レポート |

---

## 2. 监控告警

### 2.1 技术栈

| 层 | 工具 | 用途 | 来源 |
|---|---|---|---|
| Metrics | Prometheus + Alertmanager | 指标采集 / 告警路由 | 可热插拔 §7.3 |
| Visualization | Grafana | 仪表盘 | 可热插拔 §7.3 |
| Logs | Loki + Promtail | 聚合日志 | 可热插拔 §7.3 |
| Traces | Tempo + OTel Collector | 链路追踪 | 可热插拔 §7.3 |
| Audit | PG `audit.events`（append-only） | 审计 | 可热插拔 §7.3.7 |
| Kafka 监控 | Burrow / KEDA consumer-lag | 消费积压 | ADR-002 §3 |
| PG 监控 | pg_exporter + barman-cloud 内置 | DB 健康 | ADR-003 §3 |

### 2.2 核心告警规则（示例）

| 告警名 | 表达式（简） | 持续 | 级别 | 通知 |
|--------|-------------|------|------|------|
| `PodCrashLooping` | `rate(kube_pod_container_status_restarts_total[5m]) > 0` | 5 min | P2 | Slack #cats-ops |
| `HighErrorRate5xx` | `sum(rate(http_requests_total{status=~"5.."}[5m])) / sum(rate(http_requests_total[5m])) > 0.01` | 5 min | P1 | PagerDuty |
| `PGReplicationLag` | `pg_last_xact_replay_timestamp - now() > 60s` | 2 min | P1 | PagerDuty |
| `KafkaConsumerLagHigh` | `kafka_consumergroup_lag > 10000` | 10 min | P1 | PagerDuty |
| `DiskPressure` | `node_filesystem_avail_bytes / node_filesystem_size_bytes < 0.1` | 5 min | P0 | PagerDuty + 短信 |
| `DLQBacklog` | `kafka_topic_partition_current_offset{topic=~".*\\.dlq"} - kafka_topic_partition_oldest_offset > 100` | 30 min | P1 | Slack #cats-ops |
| `BarmanBackupFailed` | absent(barman_backup_last_success_time) | 26h | P0 | PagerDuty |
| `KeycloakDown` | `up{job="keycloak"} == 0` | 2 min | P0 | PagerDuty |
| `YjsWSConcurrentHigh` | `cats_collab_ws_connections > 5000` | 10 min | P2 | Slack |
| `LLMGatewayTimeout` | `rate(llm_request_timeout_total[5m]) > 0.05` | 5 min | P1 | PagerDuty |

> 完整规则在 Grafana / Alertmanager 配置仓库（`infra/observability/alerts/*.yaml`）。

### 2.3 通知路由

| 级别 | 通道 | 响应 SLA |
|------|------|----------|
| P0 | PagerDuty + 短信 + 全体 #cats-ops | 5 min 响应 |
| P1 | PagerDuty + Slack #cats-ops | 15 min 响应 |
| P2 | Slack #cats-ops | 1h 响应 |
| P3 | 工单系统 | 1 工作日 |

### 2.4 SLO 与错误预算

| SLO | 目标 | 错误预算（30 天） |
|-----|------|------------------|
| 核心 API 可用性 | 99.9% | 43.2 min |
| 翻译 LLM 端到端成功率 | 99.5% | 216 min |
| PG 主写入 P95 延迟 | < 50 ms | — |
| TM 匹配 P95 延迟 | < 300 ms（L1 模糊） | — |
| LLM 首字 P95 | < 1 s（L2） | — |

错误预算燃尽 > 80% 时触发 §6 变更冻结。

---

## 3. 备份恢复（PITR 流程）

### 3.1 备份策略

| 类型 | 工具 | 频率 | 目标 | 保留 | 来源 |
|------|------|------|------|------|------|
| **WAL 归档** | CloudNativePG + barman-cloud | 每 5 min | 对象存储（S3 兼容 / MinIO） | 7 天 | ADR-003 §3 |
| **全量备份** | `pgBackRest` | 每日 02:00 JST | 同上 | 30 天 | 可热插拔 §10.7 |
| **周备份** | `pgBackRest` full | 每周日 02:00 | 同上 | 12 月 | 可热插拔 §10.7 |
| **Schema 备份** | `pg_dump --schema-only` | 每日 03:00 | 异地（次区域） | 永久 | — |
| **Kafka topic 备份** | MirrorMaker 2 | 实时 | 异地集群 | 7 天 | ADR-002 §3 |
| **MinIO 对象** | `mc mirror` | 每日 04:00 | 异地 | 90 天 | ADR-003 §3 |

### 3.2 PITR 操作流程

1. **评估**：确认故障点（实例 / schema / 整库 / 跨库）
2. **止血**：见インシデント対応プレイブック §4 PG 故障
3. **选取恢复点**：
   ```sql
   SELECT pg_walfile_name(pg_last_xact_replay_timestamp());
   ```
4. **创建新副本**（不覆盖原集群）：
   ```bash
   barman-cloud restore \
     --cluster cats-pg-prod \
     --remote-ssh-command "ssh postgres@cats-pg-replica" \
     --target-time "2026-08-26 08:30:00+09:00" \
     cats-pg-prod
   ```
5. **校验**：`pg_dump --schema-only` 比对 + 抽样业务查询
6. **切换流量**：CNPG Cluster `switchover` 到恢复副本
7. **复盘**：写 PG-PITR 报告（30 min 内）

### 3.3 备份校验

| 项 | 频率 | 工具 | 失败响应 |
|---|---|------|----------|
| `barman check` | 每日 | barman-cloud | 自动 P1 告警 |
| 恢复 smoke test | 每周 | `barman-cloud restore --test` | 周报标注 |
| 异地恢复演练 | 每季度 | 见 §4 | 演练报告 |

---

## 4. 灾备（异地 + RTO/RPO 目标）

### 4.1 目标

| 等级 | 故障 | RTO | RPO | 备注 |
|------|------|-----|-----|------|
| L1 | 单实例 / 单 Pod | < 5 min | 0 | 自愈 |
| L2 | 单服务全部副本 | < 15 min | 0 | HPA + PDB |
| L3 | 单 AZ 不可用 | < 30 min | < 5 min（PITR） | 跨 AZ 副本 |
| L4 | 主区域不可用 | < 4 h | < 15 min | 异地冷备激活 |
| L5 | 双区域不可用 | < 24 h | < 1 h | 异地对象存储 + 重建 |

### 4.2 异地冷备架构

```
主区域（ap-northeast-1）          异地（ap-southeast-1，warm）
┌─────────────────────┐          ┌─────────────────────┐
│ K3s 3 控制面 + N 节点 │          │ K3s 3 控制面 (standby)│
│ CloudNativePG 主+从  │  WAL ──▶ │ CloudNativePG replica │
│ Kafka 3 broker       │  MM2 ──▶ │ Kafka 3 broker       │
│ MinIO 主             │  mc    ▶ │ MinIO 异备           │
└─────────────────────┘          └─────────────────────┘
```

### 4.3 DR 演练 Checklist

- [ ] 演练申请 + CAB 审批（D-7）
- [ ] 通知客户（D-3）
- [ ] 主区域流量切出
- [ ] 异地 PG promote + PITR 校验
- [ ] Kafka MM2 切换主
- [ ] 应用层 health check 全绿
- [ ] 关键 E2E 5 个冒烟通过
- [ ] 计时 RTO / RPO 实测
- [ ] 切回主区域
- [ ] 写演练报告

---

## 5. 容量管理

> 详见 `CATs_キャパシティ管理計画_v1.0.md`（同批次文档）。本章给出概要。

| 维度 | 关键指标 | 阈值 | 工具 |
|------|----------|------|------|
| CPU | 节点 / Pod CPU | Pod > 70% 持续 10 min 触发 HPA | Prometheus + KEDA |
| 内存 | 节点 / Pod 内存 | Pod > 80% 触发扩容 | 同上 |
| 磁盘 | 节点 / PG / MinIO | > 75% 预警，> 85% 自动扩容 | 同上 |
| 网络 | NIC 带宽 / 跨 AZ 流量 | > 70% 预警 | node_exporter |
| QPS | gRPC + REST | 服务级 P99 偏差 > 30% | k6 + SLO |
| PG 连接 | `pg_stat_activity` count | > 80% max_connections | pg_exporter |
| Kafka lag | consumer lag | > 1000 预警 | Burrow |
| Yjs WS | `cats_collab_ws_connections` | > 80% 设计容量 | 自定义指标 |

扩容流程：HPA 自动 / 手动 `kubectl scale` / 节点池扩容（Cluster Autoscaler）。

---

## 6. 变更管理

### 6.1 变更分类

| 类别 | 示例 | 审批 | 窗口 |
|------|------|------|------|
| **标准变更** | 单服务 Patch 版本升级 | SRE Lead | 任意 |
| **常规变更** | 单服务 Minor 升级、配置变更 | CAB 周会 | 周二/四 10:00 |
| **重大变更** | 数据库 schema、跨服务 Feature Bundle | CAB + 客户通知（D-3） | 周末 02:00-06:00 |
| **紧急变更** | 安全 CVE 修复 | SRE Lead + 架构师（事后 24h 报备） | 任意 |

### 6.2 变更流程

1. **申请**：Jira Change Request（CR）含影响范围、回滚计划、验证
2. **审批**：CAB（变更评审委员会）每周二/四例会
3. **执行**：GitOps（Argo CD）自动同步 + `git revert` 一键回滚
4. **验证**：烟雾测试（5 核心 E2E）+ 健康检查
5. **关闭**：CR 附验证结果 + Grafana 截图

### 6.3 变更冻结期

- 错误预算燃尽 > 80% → 自动冻结常规变更
- 客户重大活动期间（D-3 通知）→ 冻结
- 月末结算窗口（D-1 至 D+1）→ 冻结非紧急变更
- M3 上线前 2 周 → 冻结常规变更

### 6.4 灰度与回滚

- **Canary 5% → 25% → 50% → 100%**（可热插拔设计 §12.2）
- 错误率 / 延迟 / 业务指标 综合判定
- 任一指标超阈值自动 abort + 切回 100% 旧版（< 3 min）
- 数据库迁移：Expand → Contract 三步法

---

## 7. 安全基线

### 7.1 身份与访问

- **认证**：Keycloak OIDC，强制 MFA（管理员 / 高权限角色）（ADR-005 §2.1）
- **授权**：RBAC + ABAC 混合（ADR-005 §2.2）
- **租户隔离**：schema 隔离 + namespace 隔离 + RLS 策略（ADR-005 §2.3）
- **运维访问**：跳板机 + 短时凭据 + 完整审计

### 7.2 镜像与供应链

- **不可变标签**：Harbor 强制（可热插拔 §3.4）
- **SBOM 生成**：CI 中 `syft` + `grype` 阻断已知严重漏洞
- **依赖扫描**：`dependabot` + 周度 `cargo audit` / `npm audit` / `pip-audit`
- **密钥扫描**：`gitleaks` PR 门禁 + 历史回扫

### 7.3 网络与传输

- 所有内部通信 mTLS（gRPC + Istio/Linkerd 待评估）
- 浏览器 ↔ BFF：TLS 1.3，HSTS 强制
- 内部服务端口：ClusterIP（不暴露 NodePort / LoadBalancer）
- 出口过滤：白名单 + 敏感项目 `force_local_model` 阻断云端 LLM

### 7.4 数据保护

- **静态加密**：PG TDE + MinIO SSE-KMS
- **传输加密**：TLS 1.2+ 强制
- **脱敏**：日志中禁止打印 PII / 凭据 / TM 原文（sample 模式除外）
- **备份加密**：barman-cloud 加密至对象存储

### 7.5 审计与合规

- 所有运维操作走 RBAC 强制（可热插拔 §7.3.7）
- `audit.events` 表 append-only（PG trigger 禁 UPDATE/DELETE）
- 审计日志异地归档（可热插拔 §11 OPR-14）
- 合规 fail-closed（架构设计书原则 3）：敏感项目访问云端 LLM 立即阻断

### 7.6 漏洞响应

| 等级 | 来源 | 响应 SLA |
|------|------|----------|
| 严重（CVE score ≥ 9.0） | NVD / 厂商 | 24 h |
| 高（7.0~8.9） | 同上 | 7 d |
| 中（4.0~6.9） | 同上 | 30 d |
| 低（< 4.0） | 同上 | 下次维护窗口 |

---

## 8. 与上下游文档的关系

| 上游 | 引用方式 | 章节 |
|------|----------|------|
| ADR-001 §3 | 15 微服务 + 4 共享库清单 | §0 |
| ADR-002 §3 | gRPC + Kafka 通信 | §2.1 / §4.2 |
| ADR-003 §3 | PG 中心化 + barman-cloud | §3 |
| ADR-004 §3 | React + Tauri 前端 | §7.3 |
| ADR-005 §2 | Keycloak + RBAC/ABAC | §7.1 |
| 可热插拔部署与运维设计 v1.0 | 部署 / 告警 / 备份 / 安全 | §1~7 |
| P2M3待触发索引 v1.0 §3.4 | 触发节点 | §0 |

| 下游 | 关系 |
|------|------|
| インシデント対応プレイブック v1.0 | §1.1 D-1~D-10 异常升级 |
| キャパシティ管理計画 v1.0 | §5 详见 |
| 保守マニュアル v1.0 | §6 变更窗口 |

---

## 9. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | 详细 SLO 数字经 PRE 校准后冻结 v1.1 | SRE + QA | M3 上线前 2 周 |
| OI-2 | 异地 DR 集群 L4 首次演练 | SRE | M3 上线后 1 月 |
| OI-3 | Alertmanager 完整路由配置提交到 infra 仓库 | SRE | M2 末 |
| OI-4 | token-OLU 维度纳入容量报告（per user 偏好） | SRE + PMO | M3 上线前 |

---

**文档结束（v1.0）**
