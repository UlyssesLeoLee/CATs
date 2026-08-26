# CATs インシデント対応プレイブック v1.0

> **文档编号**：CATs-OPS-INC-001
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：SRE + 架构师（worker 代签 per DEC-008）
> **状态**：M3 上线前 1 月触发（P2M3待触发索引 v1.0 §3.4 #46）
> **配套**：CATs_运用マニュアル v1.0 / CATs_ADR-001~005 / 可热插拔部署与运维设计 v1.0

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
| **v1.0** | **2026-08-26** | **SRE + 架构师（worker 代签 per DEC-008）** | **M3 上线前 1 月触发：分级 / 流程 / 升级矩阵 / 常见 Runbook / 沟通 / 复盘 6 章** |

---

## 0. 适用范围

- **场景**：P0~P3 全部生产事件（用户感知 + 内部告警）
- **服务范围**：CATs 全栈 15 服务（ADR-001 §3）+ 共享 K3s / PG / Kafka / Keycloak
- **不在范围**：开发环境、客户自托管实例、安全合规事件（走 §7.5 + 法律流程）
- **激活条件**：P0/P1 告警触发 / 客户报障 / 监控异常

---

## 1. 事件分级

| 等级 | 定义 | 影响 | 首次响应 | 解决 SLA | 客户沟通 |
|------|------|------|----------|----------|----------|
| **P0** | 核心功能完全不可用 / 大规模数据丢失 / 安全事件 | 收入损失 + 客户信任受损 | 5 min | 1 h | 即时（5 min 内） |
| **P1** | 核心功能部分受损（无 workaround）/ SLO 偏差 > 50% | 多客户可见 | 15 min | 4 h | 30 min 内 |
| **P2** | 次要功能异常 / 单一 endpoint 故障 | 少数客户 / 内部 | 1 h | 1 工作日 | 工单回复 |
| **P3** | 文案 / 体验 / UI 微小问题 | 极个别 | 1 工作日 | 2 周 | 下迭代 |

**判定原则**：
- 涉及数据丢失、合规、安全 → 直接 P0
- 影响 > 30% 客户 → P0；10~30% → P1；< 10% → P2
- 性能 SLO 偏差 > 50% → P1；> 20% → P2
- 与判定矛盾时按"高一级"处理

---

## 2. 响应流程

```
┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐
│ ① 発見   │───▶│ ② 分類   │───▶│ ③ 止血   │───▶│ ④ 復旧   │───▶│ ⑤ 復盤   │
│ 告警/   │    │ P0~P3   │    │ ロールバック │    │ 根治対応 │    │ Postmortem│
│ 報障/   │    │ 当番招集 │    │ 切り離し  │    │ RTO達成  │    │ 学び共有  │
│ 監視    │    │ 役割分担 │    │ 緩和策    │    │ 検証     │    │ 再発防止  │
└──────────┘    └──────────┘    └──────────┘    └──────────┘    └──────────┘
```

### 2.1 发现

| 来源 | 工具 | 责任人 |
|------|------|--------|
| 告警 | Alertmanager PagerDuty | 当番 SRE |
| 客户报障 | 客服 / Slack #cats-support | 客服 → 当番 SRE |
| 监控仪表盘 | Grafana 主动巡检 | 值班 SRE |
| 内部反馈 | Slack #cats-ops | 当番 SRE |

### 2.2 分类

1. 5 min 内判定 P0~P3（§1）
2. P0/P1 立即在 Slack #cats-incident 创建频道 `inc-YYYYMMDD-NNN`
3. 招集 IC（Incident Commander）+ 通信官 + 操作用

### 2.3 止血

| 等级 | 止血手段（默认） |
|------|------------------|
| P0 | Argo CD Rollback + 流量切回 + 关键服务 scale 副本 |
| P1 | Argo CD Rollback 或 Feature Flag 关闭 |
| P2 | 配置回滚 / 单服务重启 |
| P3 | 工单记录，下迭代处理 |

### 2.4 恢复

- 健康检查全绿
- 关键 E2E 5 个冒烟通过
- 错误率 / 延迟回到基线
- 客户可正常操作

### 2.5 复盘

- P0/P1：48 h 内 Postmortem
- P2：1 周内 Postmortem（可简）
- 见 §6 复盘模板

---

## 3. 升级矩阵

### 3.1 角色定义

| 角色 | 责任 | 默认人选 |
|------|------|----------|
| **IC（Incident Commander）** | 协调 / 决策 / 升级 | SRE Lead（工作时间）/ 当番 SRE（非工作） |
| **通信官** | 对内对外沟通 | 产品 / 客户成功 |
| **操作用** | 实际执行命令 | 值班 SRE |
| **SME** | 服务 / 组件专家 | 各域 Lead |
| **架构师** | 重大变更批准 | 架构师 |

### 3.2 升级时间线

| 事件年龄 | 行动 |
|----------|------|
| 0 min | 告警触发 / 报障接收 |
| 5 min | P0/P1 告警确认 + 招集 IC |
| 15 min | P0 升级 SRE Lead + 通信官到位；P1 升级 SRE Lead |
| 30 min | P0 升级架构师 + PMO + 客户成功 |
| 1 h | P0 仍未止血 → 升级 CTO + 客户管理层 |
| 4 h | P1 仍未恢复 → 升级 SRE Lead + 架构师 |
| 24 h | P0 Postmortem 草稿 |

### 3.3 通知模板（Slack）

```
[INC-YYYYMMDD-NNN] [P0/P1] <一句话摘要>
影响范围: <客户数 / 业务功能>
当前状态: <调查/止血/恢复>
IC: @user
通信官: @user
下次更新: HH:MM (每 15 min)
```

---

## 4. 常见 Runbook

### 4.1 PG 故障

**症状**：PG 写入失败 / 主从切换 / 复制延迟 / 连接耗尽

**止血**：
1. `kubectl get pods -n cnpg-system` 确认 CNPG 控制器健康
2. 副本状态：`cnpg status cats-pg-prod`
3. 主实例重启失败 → `kubectl delete pod cats-pg-prod-1`（自愈）
4. 主从全挂 → 启用异地冷备（见运用手册 §4.3）

**根因排查**：
```sql
-- 长事务
SELECT pid, age(clock_timestamp(), query_start), state, query
FROM pg_stat_activity WHERE state != 'idle' ORDER BY query_start;

-- 锁等待
SELECT blocked_locks.pid AS blocked_pid, blocking_locks.pid AS blocking_pid
FROM pg_locks blocked_locks
JOIN pg_locks blocking_locks ON blocking_locks.locktype = blocked_locks.locktype
WHERE NOT blocked_locks.granted;

-- 复制槽膨胀
SELECT slot_name, pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), restart_lsn)) AS lag
FROM pg_replication_slots;
```

**恢复后**：
- 验证 PITR 备份窗口
- 审计长事务根因
- 容量评审（连接数 / WAL 增长）

### 4.2 Kafka 故障

**症状**：Broker 宕机 / ISR 收缩 / Consumer lag 暴涨 / DLQ 积压

**止血**：
1. `kafka-broker-api-versions --bootstrap-server ...` 检查集群
2. 单 broker 宕机：自动 ISR 重平衡（< 30s）
3. 2 broker 宕机（RF=3）：手动切主 + 启用异地 MM2
4. Consumer lag 暴涨：临时 scale consumer pod

**根因排查**：
```bash
# Consumer group 状态
kafka-consumer-groups.sh --bootstrap-server $BS \
  --describe --group $GROUP

# DLQ 抽样
kafka-console-consumer.sh --topic $DOMAIN.dlq \
  --from-beginning --max-messages 10

# Topic 配置
kafka-configs.sh --bootstrap-server $BS \
  --entity-type topics --entity-name $TOPIC --describe
```

**恢复后**：
- DLQ 重放或归档（24h 内）
- 调整 partition / consumer 副本数
- Schema Registry 兼容性检查

### 4.3 LLM 网关故障

**症状**：翻译请求超时 / 5xx 飙升 / 流式中断 / 限流告警

**止血**：
1. `kubectl get pods -l app=llm-gateway` 确认健康
2. 上游 LLM 厂商故障 → 切备选厂商（特性开关）
3. 限流触发 → 临时提高 rate limit + 通知用户降级
4. 敏感项目访问云端 LLM（合规阻断）→ 强杀 + 审计

**根因排查**：
```bash
# 上游 LLM 健康
curl -sS https://api.openai.com/v1/models -H "Authorization: Bearer $KEY" | jq

# llm-gateway 日志
stern llm-gateway --since 10m | grep -E "ERROR|timeout"

# 错误码分布
curl -sS http://prom:9090/api/v1/query?query=sum by (status)(rate(llm_request_total[5m]))
```

**恢复后**：
- 限流阈值评审
- 厂商 SLA 复盘
- 备选厂商演练

### 4.4 Yjs 协同异常

**症状**：WS 连接断开 / CRDT 冲突 / 文档不同步 / 客户端报错

**止血**：
1. `kubectl get pods -l app=collab-ws` 确认 WS 网关健康
2. WS 进程崩溃 → K8s 自愈 + 客户端自动重连
3. 持久层（collab-persistence）故障 → 切只读模式（用户只读已加载快照）
4. 重大 bug → 强制重连 + 提示保存

**根因排查**：
```bash
# WS 连接数
curl -sS http://prom:9090/api/v1/query?query=cats_collab_ws_connections

# collab-persistence 慢查询
stern collab-persistence --since 10m | grep -E "slow|timeout"

# 客户端错误（前端埋点）
# 通过 Grafana 看板 collab-client-errors
```

**恢复后**：
- 复盘 CRDT 合并逻辑
- 评估 PG JSONB 写入吞吐
- 客户端降级方案评审

---

## 5. 沟通模板

### 5.1 客户外发（事故通知）

```
[CATs 障害通知] YYYY-MM-DD HH:MM JST

■ 概要
○○功能 currently unavailable, currently investigating.

■ 影响範囲
<功能> - <客户数 or 全部>

■ 状态
- 调查 / 止血 / 恢复中
- 预计恢复: HH:MM（不能确定则写 "未定"）

■ 我们的対応
- IC: <姓名>
- 下次更新: HH:MM

ご不便おかけいたしまして申し訳ございません。
```

### 5.2 客户外发（恢复通知）

```
[CATs 障害復旧通知] YYYY-MM-DD HH:MM JST

■ 概要
○○功能 has been restored at HH:MM JST.

■ 影響範囲
<功能> - <客户数>

■ 原因
<简述，不暴露内部细节>

■ 再発防止
<1-2 句承诺>

ご不便おかけいたしまして申し訳ございませんでした。
```

### 5.3 内部 Status Update

```
[INC-YYYYMMDD-NNN] HH:MM Update
- 当前状态: <调查/止血/恢复/已恢复>
- 关键动作: <1-2 句>
- 下一步: <1 句>
- 预计: <时间 or 不确定>
- 阻塞: <if any>
```

---

## 6. 复盘模板（Postmortem）

### 6.1 头部

| 项 | 内容 |
|---|------|
| INC ID | INC-YYYYMMDD-NNN |
| 等级 | P0 / P1 / P2 |
| 开始时间 | YYYY-MM-DD HH:MM JST |
| 恢复时间 | YYYY-MM-DD HH:MM JST |
| 总时长 | HH:MM |
| IC | 姓名 |
| 影响客户数 | N |

### 6.2 时间线

| 时间 | 事件 | 操作人 |
|------|------|--------|
| HH:MM | 告警触发 |  |
| HH:MM | IC 招集 |  |
| HH:MM | 根因定位 |  |
| HH:MM | 止血执行 |  |
| HH:MM | 恢复 |  |

### 6.3 根因（5 Whys）

1. Why: ...
2. Why: ...
3. Why: ...
4. Why: ...
5. Why: 根因

### 6.4 影响

- 客户影响：功能 / 时长 / 数量
- 收入影响：估算
- 数据影响：丢失 / 不一致（如有）
- SLO 影响：错误预算消耗

### 6.5 行动项

| # | 行动 | 责任 | 截止 | 状态 |
|---|------|------|------|------|
| 1 | 短期止血改进 |  | D+7 |  |
| 2 | 中期根因修复 |  | D+30 |  |
| 3 | 长期架构改进 |  | D+90 |  |

### 6.6 学到的教训

- 检测延迟：告警是否及时？
- 响应效率：IC 决策是否快速？
- 工具：Runbook 是否完备？
- 流程：变更 / 灰度 是否到位？

---

## 7. 与上下游文档的关系

| 上游 | 引用 | 章节 |
|------|------|------|
| ADR-001 §3 | 15 服务清单 | §0 / §4 |
| ADR-002 §3 | gRPC + Kafka 通信 | §4.2 |
| ADR-003 §3 | PG 中心化 | §4.1 |
| ADR-004 §3 | 前端（Yjs 客户端） | §4.4 |
| ADR-005 §2 | Keycloak 鉴权 | §4.1（审计） |
| 可热插拔 §11 | 风险登记册 OPR-01~15 | §4 各节 |

| 下游 | 关系 |
|------|------|
| 运用マニュアル v1.0 §1.1 | D-1~D-10 异常升级目标 |
| キャパシティ管理計画 v1.0 | §4 各节容量相关 |
| 保守マニュアル v1.0 | §6 升级窗口 |

---

## 8. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | 各域 SME 联系人名单冻结 | SRE Lead | M3 上线前 1 周 |
| OI-2 | 异地 DR 实战演练 | SRE | M3 上线后 1 月 |
| OI-3 | Status page 公网部署 | 产品 | M3 上线前 2 周 |
| OI-4 | 自动 RCA 工具评估 | SRE + 架构 | M4 |

---

**文档结束（v1.0）**
