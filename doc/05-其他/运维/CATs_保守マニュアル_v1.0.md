# CATs 保守マニュアル v1.1（基线升级）

> **文档编号**：CATs-OPS-MNT-001
> **バージョン**：v1.1（基线升级 WT-H2）
> **作成日**：2026-08-26
> **作者**：SRE + 架构师 + Rust Lead + DBA（worker 代签 per DEC-008）
> **基线引用**：[CATs_技术基线_v1.0 §1](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md)
> **状态**：M3 上线后 1 月触发（P2M3待触发索引 v1.0 §3.5 #54）
> **配套**：CATs_运用マニュアル v1.0 / CATs_ADR-001~005 / CATs_可热插拔部署与运维设计 v1.0

---

## 文档管理信息

### 审批栏

| 役割 | 氏名 | 承認 | 日付 | 備考 |
|------|------|------|------|------|
| 起案 | SRE + アーキテクト（worker 代签 per DEC-008） | ☑ | 2026-08-26 | v1.0 初版 |
| レビュー | — | ☐ | — | — |
| 承認 | — | ☐ | — | — |

### 修订履历

| バージョン | 日付 | 修订者 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-26** | **SRE + アーキテクト（worker 代签 per DEC-008）** | **M3 上线后 1 月触发：维护窗口 / 补丁 / 升级 / 迁移 / 紧急维护 / 清单 6 章** |
| **v1.1** | **2026-08-26** | **SRE + 架构师 + Rust Lead + DBA（worker 代签 per DEC-008）** | **基线升级(WT-H2)**:§2.3 升级时间表 "Minor 升级(PG 16.x.y)" → **"Minor 升级(PG 18.6.x)"**;"Major 升级(PG 16 → 17)" → **"Major 升级(PG 18.6 → 19)"**;**新增** "Minor 升级(Rust 1.98.x)" 行,半年一次;引用 `CATs_技术基线_v1.0 §1`** |

---

## 0. 适用范围

- **環境**：PROD（本番運用）、PRE（事前検証）
- **サービス範囲**：CATs 15 マイクロサービス（ADR-001 §3）+ 共有 K3s / PG（ADR-003 §3）/ Kafka（ADR-002 §3）/ Keycloak（ADR-005 §2.1）
- **目的**：本番稼働中の定期メンテナンス、計画アップグレード、緊急対応の標準化
- **読者**：SRE / アーキテクト / サービス Lead

---

## 1. 维护窗口

### 1.1 定期维护窗口

| 区分 | 时间 | 影响范围 | 通知 |
|------|------|----------|------|
| **週次** | 毎週 火・木 02:00-06:00 JST | 単一サービス Patch 適用 | 前日 18:00 まで |
| **月次** | 毎月 第2日曜日 02:00-08:00 JST | 複数サービス / OS / ミドルウェア | D-7 通知 |
| **四半期** | 毎四半期 第1日曜日 02:00-12:00 JST | 大型アップグレード / DB メジャーバージョン | D-30 通知 |
| **年次** | 毎年 12月 第2日曜日 02:00-24:00 JST | 全体 DR 演習 + 大型変更 | D-90 通知 |

### 1.2 窗口选择原则

- 周末 02:00-06:00 JST = 業務影響最小
- 客户结算期间（D-1 ~ D+1）禁止
- 客户重大活动期间（D-3 通知）禁止
- 春节 / 黄金周 / 盆 / 正月 = 冻结期（紧急除外）

### 1.3 窗口取消判定

- 窗口开始 24h 前错误预算消耗 > 50% → 推迟
- 客户 P0/P1 事件进行中 → 取消
- 大型客户活动临时通知 → 重新排期

---

## 2. 补丁管理

### 2.1 OS / 基础镜像

| 项 | 工具 | 频率 | 流程 |
|---|------|------|------|
| K3s 节点 OS 升级 | `kured` + OS package | 每月 | 週次窗口内、节点 cordon → drain → 升级 → uncordon |
| 基础镜像 CVE 修复 | Harbor 镜像重建 + CI | 每周 | CI 触发 `trivy scan` → 高危修复 → rebuild |
| 内核升级 | 専用 playbook | 每季度 | 单独窗口，需重启 |

### 2.2 Rust / Node / Python 依赖

| 类型 | 工具 | 频率 | 流程 |
|------|------|------|------|
| Rust crates | `cargo audit` + Dependabot | 每周 | CVE 评估 → 升级 PR → Canary → 全量 |
| Node packages | `npm audit` + Dependabot | 每周 | 同上 |
| Python packages | `pip-audit` + Dependabot | 每周 | 同上 |
| Container base image | `grype` + `trivy` | 每周 | Harbor 镜像扫描 → 重建 |

**SLA**（運用マニュアル §7.6 联动）：
- 严重（CVE ≥ 9.0）：24h
- 高（7.0~8.9）：7d
- 中（4.0~6.9）：30d
- 低（< 4.0）：下次维护窗口

### 2.3 PostgreSQL 升级

| 升级类型 | 频率 | 流程 |
|----------|------|------|
| Minor 升级(PG 18.6.x) | 每月 | CNPG 滚动重启(无停机) |
| Major 升级(PG 18.6 → 19) | 每年 / 按需 | pg_upgrade + 长时间窗口 + DR 备份验证 |

**升级前 Checklist**：
- [ ] barman 完整备份成功
- [ ] DR 异地副本同步延迟 < 5 min
- [ ] 扩展兼容性验证（pgvector / PostGIS 等）
- [ ] 性能基线（PRE 校准）
- [ ] 回滚脚本就绪（< 30 min 切换回旧版）

### 2.4 Kafka 升级

- **Minor 升级**：rolling restart broker（< 10 min / broker）
- **Major 升级**：双集群蓝绿（ADR-002 §3 Kafka 决策）
- **客户端协议**：需评估新协议兼容性

### 2.5 Keycloak 升级

- Keycloak 季度发布，半年内全部采纳
- DB schema migration 在 PRE 验证
- 升级期间 IdP 切换（双活）→ 0 停机

---

## 3. 服务升级

### 3.1 升级策略选择

| 场景 | 策略 | 风险 | 时长 |
|------|------|------|------|
| Patch 升级（向后兼容） | **滚动升级（RollingUpdate）** | 低 | < 30 min |
| Minor 升级（含 API 扩展） | **滚动 + Feature Flag** | 中 | 1~2 h |
| Minor 升级（含 DB schema） | **Expand-Contract 三步法** | 中高 | 4~8 h（跨周） |
| Major 升级 / 破坏性变更 | **蓝绿（Blue-Green）** | 高 | 4~8 h |
| 大型 Feature Bundle 灰度 | **Canary 5% → 25% → 50% → 100%** | 中 | 1~2 周 |

### 3.2 滚动升级（Rolling Update）

适用于：单服务无状态升级、镜像替换。

```bash
# 1. 镜像推送
docker push harbor.cats.internal/<svc>:<version>-stable-<sha>

# 2. 触发 Argo CD 同步
argocd app sync <svc> --revision <tag>

# 3. 监控滚动状态
kubectl rollout status deploy/<svc> -n cats

# 4. 验证
./scripts/smoke-test.sh <svc>
```

**配置要求**（可热插拔 §3.5）：
- `maxSurge: 25%`
- `maxUnavailable: 0`（无停机）
- `readinessProbe` 必须存在且准确
- `preStop` hook 等待 in-flight 请求完成（默认 30s）

### 3.3 蓝绿（Blue-Green）

适用于：Major 升级 / 破坏性变更 / 跨服务 Feature Bundle。

```bash
# 1. 部署新版本（green）
kubectl apply -f green/  # 新版本 service label: version=green

# 2. 内部验证
kubectl port-forward svc/<svc>-green 8080:80
./scripts/e2e-test.sh

# 3. 流量切换（按比例）
istio virtualservice ...  # 5% → 50% → 100%

# 4. 监控 + 自动 abort（如有异常）

# 5. 旧版本 standby 24h
# 6. 删除旧版本
kubectl delete -f blue/
```

### 3.4 Canary（Argo Rollouts）

适用于：跨服务 Feature Bundle 灰度发布。

**配置示例**：
```yaml
apiVersion: argoproj.io/v1alpha1
kind: Rollout
metadata:
  name: pdf-translation-v2
spec:
  strategy:
    canary:
      steps:
        - setWeight: 5
        - pause: { duration: 30m }
        - setWeight: 25
        - pause: { duration: 2h }
        - setWeight: 50
        - pause: { duration: 4h }
        - setWeight: 100
      analysis:
        templates:
          - templateName: error-rate
          - templateName: p99-latency
        startingStep: 2
        failureLimit: 2
```

**自动 abort 触发条件**（可热插拔 §11 OPR-13）：
- 错误率 > 1% 持续 5 min
- P99 延迟 > SLO 50% 持续 10 min
- 业务指标异常（任务成功率 / 翻译质量）

### 3.5 数据库 Schema 升级（Expand-Contract）

```
① Expand:  加新列（NULL 允许）→ 部署应用同时写新/旧列
② Migrate: 后台回填历史数据（按 batch）
③ Switch:  部署应用只读写新列
④ Contract: 删除旧列
```

每步必须：
- 向前可回滚
- 备份就绪
- 监控覆盖
- 文档化

---

## 4. 数据迁移

### 4.1 迁移场景

| 场景 | 工具 | 时长 |
|------|------|------|
| 单表分区调整 | `pg_partman` + DDL | < 2 h |
| 跨 schema 迁移 | `dblink` + 验证脚本 | 4~8 h |
| 跨库迁移 | `barman` + 应用双写期 | 1~2 周 |
| 异构迁移（如 ES → PG） | 自定义 ETL | 视数据量 |
| 冷存迁移 | `mc mirror` + lifecycle | 后台 |

### 4.2 迁移流程

1. **评估**：数据量 / 停机影响 / 回滚成本
2. **PRE 演练**：全流程在 PRE 跑通
3. **备份**：迁移前 1h 全量备份
4. **双写期**（如需）：新旧两侧并行写，校验一致性
5. **切换**：流量切到新侧
6. **回读期**：7 天读双侧校验
7. **退役**：删除旧侧

### 4.3 大表迁移注意

- > 1 亿行表禁止 `ALTER TABLE` 长时间锁
- 用 `gh-ost` / `pg_repack` 工具
- 评估 replication lag
- 监控 long transaction

---

## 5. 紧急维护流程

### 5.1 触发条件

- 严重安全 CVE（score ≥ 9.0）
- 平台级 P0 事件需紧急修复
- 合规 / 法律要求立即变更
- 客户合同 SLA 违约紧急修复

### 5.2 流程（跳过 CAB 审批，事后报备）

```
触发（0 min）
   │
   ├─ IC 招集（SRE Lead + 架构师）
   │
   ├─ 风险评估 + 缓解（< 15 min）
   │
   ├─ 通知客户（D-0 立即，CS 主导）
   │
   ├─ 执行变更（< 24 h 内）
   │
   ├─ 验证 + 监控加强
   │
   └─ 24 h 内补 CAB 报备 + 事故报告
```

### 5.3 紧急变更 Checklist

- [ ] 风险评估：影响范围 + 回滚步骤
- [ ] IC 决策（不在 Slack 异步决策）
- [ ] 客户通知（CS 主导，文案模板见 プレイブック §5）
- [ ] 备份就绪（即使 5 min snapshot）
- [ ] 监控 dashboard 临时新增
- [ ] 值班人员 + 架构师 24h 待命
- [ ] 事后 24h Postmortem

---

## 6. 维护 Checklist

### 6.1 维护前（D-7 ~ D-1）

- [ ] 维护申请 CR（含影响范围 / 回滚 / 验证 / 通知文案）
- [ ] CAB 审批通过（常规 / 重大变更）
- [ ] 客户通知（D-3 重大，D-7 年/季度）
- [ ] PRE 演练完成（所有步骤可执行）
- [ ] 备份完成 + 验证
- [ ] 监控 dashboard 新增 / 更新
- [ ] 值班表 + 升级路径确认
- [ ] 沟通模板就绪
- [ ] rollback runbook 演练
- [ ] 工单 / 客户群通知

### 6.2 维护中（窗口内）

- [ ] Status page 切到 maintenance
- [ ] Slack #cats-ops-incident 实时更新
- [ ] 每步执行 + 验证（10 min 健康观察）
- [ ] 错误率 / 延迟实时监控
- [ ] 客户高频问题快速响应
- [ ] 异常时立即 abort + 评估
- [ ] 关键节点截图 / 日志保存

### 6.3 维护后（D+1 ~ D+7）

- [ ] 验证报告（健康检查 / 烟雾测试 / 业务指标）
- [ ] 客户通知（恢复 + 变更总结）
- [ ] Status page 切回 normal
- [ ] CR 关闭 + 归档
- [ ] Postmortem（如有异常）
- [ ] 文档 / Runbook 更新
- [ ] 容量 / 性能 baseline 校准
- [ ] token-OLU 报告（per user 偏好）

### 6.4 季度维护评审

- 本季度维护次数 + 平均时长 + 失败率
- 紧急维护比例（目标 < 10%）
- 客户影响时长
- 改进项清单
- 容量 / 成本变化

---

## 7. ロールバック戦略まとめ

| 变更类型 | ロールバック | 时长 | 数据影响 |
|----------|--------------|------|----------|
| 镜像回滚 | `argocd app rollback` | < 5 min | 无 |
| 配置回滚 | `git revert` + sync | < 5 min | 无 |
| DB schema（已 Contract） | restore backup | < 30 min | 数据到备份点 |
| DB schema（仅 Expand） | 删新列 | < 5 min | 无 |
| 蓝绿切换 | 切回 blue | < 3 min | 无 |
| Canary 失败 | Argo Rollouts abort | < 3 min | 无 |
| 跨集群迁移 | 切流回旧集群 | < 15 min | 双写期 0 丢失 |

---

## 8. 上游 / 下游文档

| 上游 | 引用 | 章节 |
|------|------|------|
| ADR-001 §3 | 15 服务清单 | §0 |
| ADR-002 §3 | gRPC + Kafka 通信 | §2.4, §3.1 |
| ADR-003 §3 | PG 中心化 + CloudNativePG | §2.3, §3.5, §4 |
| ADR-004 §3 | 前端升级 | §3.2 |
| ADR-005 §2 | Keycloak | §2.5 |
| 可热插拔 §3.5 | 回滚契约 | §7 |
| 可热插拔 §11 | 风险 OPR-01~15 | §5 |
| 運用マニュアル v1.0 §6 | 变更管理 | §1, §3 |

| 下游 | 関係 |
|------|------|
| 運用マニュアル v1.0 | 連携 |
| インシデント対応プレイブック v1.0 | 緊急対応 |

---

## 9. 待办 / Open Items

| # | 項 | 責任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | 自动 rollback 工具评估 | SRE + 架构 | M3 上线后 2 月 |
| OI-2 | Argo Rollouts 全服务接入 | SRE | M3 上线前 |
| OI-3 | 双活 Keycloak 落地 | SRE | M4 |
| OI-4 | 季度评审自动化报告 | SRE + PMO | M3 上线后 3 月 |

---

**文档结束（v1.0）**
