# CATs Kafka 物理发布设计 (K3s 阶段二)

> **文档编号**：CATs-DEPLOY-KAFKA-001
> **关联任务**：T-07 Kafka 物理发布（per 启动会决议 10 b 部分 + 决议 6 SRE 平台独立估算 v1.0 §4 阶段二）
> **关联 commit baseline**: `be712dc` (SRE 平台独立估算 v1.0) + `1b27b2b` (启动会决议 6+10) + `0b9cec6` (画图 v1.0 §3 部署架构)
> **截止**: 9/27 T-07 (per 启动会决议 10 + 任务拆解 §2 line 136)
> **版本**: v1.0
> **创建日**: 2026-09-11
> **状态**: 设计初稿 (SRE 平台 Lead 真人到位后 K3s 实施, per 启动会决议 6 SRE 估算 v1.0 §4)
> **作者**: SRE 平台 Lead (Ulysses 兼一人公司 / Mavis 接手 agent per DEC-008 代签)

---

## 文档管理信息

### 审批栏

| 角色 | 姓名 | 审批 | 日期 | 备注 |
|------|------|------|------|------|
| Sponsor (Ulysses 本人签) | Ulysses | ☐ | — | 一人公司 = Ulysses 持有 Sponsor 角色，不代签 |
| 架构师 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | Kafka 拓扑对齐 |
| Rust Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | 业务服务 Kafka client 集成 |
| SRE 平台 Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | K3s 部署 + Debezium CDC 实施 |
| DBA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | DB schema + Debezium source |
| QA Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | Kafka 集成测试 |
| PMO Lead | Ulysses（Mavis 代签 per DEC-008） | ☐ | — | T-07 进度跟踪 |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-09-11 | SRE 平台 Lead（Mavis 接手 agent per DEC-008） | 初版：Kafka 物理发布设计 + K3s 阶段二部署 + 8 topic 清单 + 实施步骤 7 步 + 7 已知缺口 |

---

## 0. 元信息

| 项 | 值 |
|----|----|
| **Kafka 版本** | Apache Kafka 3.7.0 (KRaft mode, 0 Zookeeper 依赖) |
| **部署平台** | K3s 阶段二 (3 master + N worker) per 决议 6 SRE 估算 v1.0 §4 |
| **关联文档** | SRE 平台独立估算 v1.0 (be712dc) + 微服务架构设计书 v1.0 (2910f3d) + 接口设计书 v2.0+2 (1f3c94d) + 告警规则 v1.0 (1d8926d) + 画图 v1.0 (0b9cec6) §3 部署架构 |
| **关联 K8s manifests** | `deploy/k3s/cats-kafka-statefulset.yaml` + `deploy/k3s/cats-kafka-topics.yaml` + `deploy/k3s/cats-kafka-sasl-secrets.yaml` (留 K8s Secret 注入) |
| **7 实施步骤** | 1. SASL 凭据生成 → 2. K8s Secrets 注入 → 3. Namespace 创建 → 4. ConfigMap + StatefulSet apply → 5. Topic 创建 → 6. Debezium 部署 → 7. Service 16 域 Kafka client 集成 |

### 0.1 引用清单 (git 实证)

| 引用文档 | 路径 | commit hash | 用途 |
|---------|------|------------|------|
| SRE 平台独立估算 v1.0 | `doc/05-其他/管理/CATs_M1-Sprint1_SRE平台独立估算_v1.0.md` | `be712dc` | §4 K3s 阶段二规划 + 真人到位依赖 |
| 微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | `2910f3d` | §15 Kafka 部署 + §14 阶段一/二 |
| 接口设计书 v2.0+2 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | `1f3c94d` | §5 Kafka Topics + §6 端到端 |
| 告警规则 v1.0 | `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` | `1d8926d` | §4 实施 + §6.5 Kafka 物理发布留 K3s 阶段二 |
| 错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | `2146f53` | §5.2 Kafka topic 名 per event_type 映射 (Sprint 2 升 v1.1) |
| 画图 v1.0 | `doc/02-基础设计/架构设计/CATs_画图_v1.0.md` | `0b9cec6` | §3 部署架构 K3s 阶段二 |

---

## 1. Kafka 集群架构

### 1.1 KRaft mode (3.7.0)

- **0 Zookeeper 依赖**: Kafka 3.3+ KRaft mode (per Kafka 3.7 文档)
- **3 broker + 3 controller**: 同一节点 3 副本 (3 StatefulSet)
- **SASL/SCRAM-SHA-512**: per 启动会决议 §14 (Kafka 生产/消费鉴权)
- **3 副本 + min.insync.replicas=2**: 数据强一致性 (per 启动会 §14)

### 1.2 部署资源

| 资源 | 规格 | 数量 |
|------|------|------|
| Kafka broker | 2 CPU + 4Gi mem + 100Gi 存储 | 3 StatefulSet |
| 持久化卷 | local-path storage class | 3 × 100Gi |
| Namespace | cats-data | 1 |
| Service | ClusterIP None (headless) | 1 |
| ConfigMap | cats-kafka-config | 1 |
| Secret | cats-kafka-jaas (K8s Secret 注入, 0 写实际凭据) | 1 |

### 1.3 Topic 清单 (8 topic per `deploy/k3s/cats-kafka-topics.yaml`)

| Topic | Partitions | Replicas | Retention | 用途 |
|------|------------|----------|-----------|------|
| `file.events` | 6 | 3 | 7 天 | file-service Outbox → file 事件 (per §6 步骤 5-6) |
| `task.events` | 12 | 3 | 7 天 | task-service Outbox → task 事件 (per §6 步骤 7-8) |
| `task.media.asr.requested` | 6 | 3 | 1 天 | ingestion → asr (per §6 步骤 10-11) |
| `task.media.asr.completed` | 6 | 3 | 1 天 | asr → task-service (per §6 步骤 13) |
| `task.media.subtitle.requested` | 6 | 3 | 1 天 | subtitle 处理 (per §6 步骤 14) |
| `task.media.subtitle.completed` | 6 | 3 | 1 天 | subtitle → render (per §6 步骤 17-18) |
| `task.media.render.requested` | 6 | 3 | 1 天 | render 处理 (per §6 步骤 18-19) |
| `task.media.render.completed` | 6 | 3 | 1 天 | render → task.completed (per §6 步骤 21-22) |
| `audit.events` | 6 | 3 | 30 天 | audit-service 审计 (per 错误码表 §5.2 + 告警规则 §6.5) |
| `project.events` | 3 | 3 | 7 天 | project 翻译核心缓存失效 |

**10 topic 全部按 微服务架构设计书 v1.0 §15 + 接口设计书 v2.0+2 §5 设计**

---

## 2. SASL/SCRAM 鉴权

### 2.1 凭据生成 (per 启动会 §14 Kafka SASL)

```bash
# 1. 生成 SCRAM-SHA-512 凭据 (在 Kafka 集群内执行, K8s Job)
kubectl exec -n cats-data cats-kafka-0 -c kafka -- \
  kafka-configs.sh --bootstrap-server localhost:9092 \
    --alter --add-config 'SCRAM-SHA-512=[password=YOUR_PASSWORD_HERE]' \
    --entity-type users --entity-name cats-service

# 2. 验证凭据 (per service 独立, 最小权限)
# - auth-service-producer: file.events, audit.events
# - auth-service-consumer: audit.events
# - user-service-producer: audit.events
# - ... (per 接口设计书 v2.0+2 §14 鉴权)

# 3. 凭据 0 写 git (per 守门 #5 8/27 11:06 JST hard ban)
#    → K8s Secret 注入 (per SRE 平台 Lead 真人到位)
```

### 2.2 ACL 配置 (最小权限)

| Service | Producer Topics | Consumer Topics | Group ID |
|---------|----------------|-----------------|----------|
| auth-service | audit.events | (无) | (无) |
| user-service | audit.events | (无) | (无) |
| file-service | file.events | (无) | (无) |
| task-service | task.events, audit.events | (无) | (无) |
| ingestion-service | task.media.asr.requested, task.media.ocr.requested, task.media.office.requested | file.events, task.events | cats-ingestion |
| asr-service | task.media.asr.completed, audit.events | task.media.asr.requested | cats-asr |
| ocr-service | task.media.ocr.completed, audit.events | task.media.ocr.requested | cats-ocr |
| subtitle-service | task.media.subtitle.completed, audit.events | task.media.asr.completed | cats-subtitle |
| office-converter-service | task.media.office.completed, audit.events | task.media.office.requested | cats-office |
| render-writer-service | task.media.render.completed, audit.events | task.media.render.requested | cats-render |
| notification-service | (无) | task.events | cats-notification |
| report-service | (无) | task.events, task.media.*.completed | cats-report |
| audit-service | (无) | audit.events | cats-audit |
| translation-core | (无) | project.events | cats-translation |

> **关键设计 (per 启动会 §14)**: 每服务独立凭据 + Topic 级 ACL + 最小权限

### 2.3 SASL/SCRAM 凭据 K8s Secret 注入

> **重要**: 凭据 0 写 git / 0 写 deploy yaml (per 守门 #5 8/27 11:06 JST hard ban)
> 由 SRE 平台 Lead 真人到位后 (per 启动会决议 6 SRE 估算 v1.0 §4) 通过 K8s Secret 注入:

```bash
# K8s Secret 模板 (per 服务, 0 写实际凭据)
kubectl create secret generic cats-kafka-jaas \
  --namespace=cats-data \
  --from-file=kafka_jaas.conf=/path/to/secret/kafka_jaas.conf
```

> Kafka JAAS 配置 模板 (`kafka_jaas.conf`):
> ```
> KafkaServer {
>   org.apache.kafka.common.security.scram.ScramLoginModule required
>     username="cats-admin"
>     password="<FROM_K8S_SECRET>";
> };
> KafkaClient {
>   org.apache.kafka.common.security.scram.ScramLoginModule required
>     username="cats-service"
>     password="<FROM_K8S_SECRET>";
> };
> ```

---

## 3. Debezium CDC 部署

### 3.1 Debezium Connect (per 微服务架构设计书 v1.0 §14 + 启动会 §6 实施)

```yaml
# 留 K3s 阶段二实施 (per 启动会决议 6 SRE 估算 v1.0 §4 真人到位依赖)
# Debezium Connect 集群部署
# - debezium/connect:2.5
# - PostgreSQL source connector
# - Outbox 表监听 (file_events_outbox / task_events_outbox / audit_events_outbox)
# - 自动发布到 Kafka topic
```

### 3.2 Outbox 表 → Kafka topic 映射 (per 接口设计书 v2.0+2 §6 端到端示例)

| Outbox 表 | Debezium Source | Kafka Topic | 业务事件 |
|-----------|-----------------|-------------|----------|
| `file_db.file_events_outbox` | `file_events_outbox` (event_type=file.uploaded) | `file.events` | 文件上传完成 |
| `task_db.task_events_outbox` | `task_events_outbox` (event_type=task.created) | `task.events` | 任务创建 |
| `audit_db.audit_events_outbox` | `audit_events_outbox` | `audit.events` | 审计事件 (K3s 阶段二启用, 当前 DbAuditSink) |

> **关键设计 (per 接口设计书 v2.0+2 §6 关键不变量)**: 业务数据 + Outbox 同一事务 (第 5/7 步), Debezium 只负责转发 Kafka, Kafka 短暂不可用不丢事件

---

## 4. 16 域 service Kafka client 集成

### 4.1 集成模式 (per rust-rdkafka 0.36)

```rust
// crates/<service>/src/kafka.rs 模板
use rdkafka::{
    config::ClientConfig,
    consumer::{Consumer, StreamConsumer},
    producer::{FutureProducer, FutureRecord},
};

pub struct KafkaConfig {
    pub bootstrap_servers: String,
    pub sasl_username: String,
    pub sasl_password: String,  // 0 写代码, K8s Secret 注入
}

pub async fn create_producer(config: &KafkaConfig) -> FutureProducer {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", &config.bootstrap_servers)
        .set("security.protocol", "SASL_PLAINTEXT")
        .set("sasl.mechanism", "SCRAM-SHA-512")
        .set("sasl.username", &config.sasl_username)
        .set("sasl.password", &config.sasl_password)
        .set("message.timeout.ms", "5000")
        .set("acks", "all")
        .set("enable.idempotence", "true")
        .create()
        .expect("Producer creation failed")
    producer
}
```

### 4.2 16 域 Kafka client 集成任务分配 (per 启动会决议 10 + T-07 阶段二)

| 服务 | Producer | Consumer | Sprint |
|------|----------|----------|--------|
| auth-service | audit.events | (无) | 阶段二 (Sprint 2) |
| user-service | audit.events | (无) | 阶段二 (Sprint 2) |
| project-service | project.events | (无) | 阶段二 (Sprint 2) |
| task-service | task.events, audit.events | (无) | 阶段二 (Sprint 2) |
| file-service | file.events, audit.events | (无) | 阶段二 (Sprint 2) |
| ingestion-service | task.media.*.requested | file.events, task.events | 阶段二 (Sprint 2) |
| asr-service | task.media.asr.completed | task.media.asr.requested | 阶段二 (Sprint 2) |
| ocr-service | task.media.ocr.completed | task.media.ocr.requested | 阶段二 (Sprint 2) |
| subtitle-service | task.media.subtitle.completed | task.media.asr.completed | 阶段二 (Sprint 2) |
| office-converter-service | task.media.office.completed | task.media.office.requested | 阶段二 (Sprint 2) |
| render-writer-service | task.media.render.completed | task.media.render.requested | 阶段二 (Sprint 2) |
| translation-core | (无) | project.events | 阶段二 (Sprint 2) |
| notification-service | (无) | task.events | 阶段二 (Sprint 2) |
| report-service | (无) | task.events, task.media.*.completed | 阶段二 (Sprint 2) |
| audit-service | (无) | audit.events | 阶段二 (Sprint 2) |
| cats-bff | (无) | (无, BFF 不直接连 Kafka) | N/A |

**16 域 0 当前 commit (除 auth-service T-01 + user-service T-02)**, 阶段二 Sprint 2 实施 (per Sprint 1 复盘 §4.2 完成度 12.5%)

---

## 5. 实施步骤 (7 步, SRE 平台 Lead 真人到位后)

### Step 1: SASL 凭据生成 (per §2.1)

```bash
kubectl exec -n cats-data cats-kafka-0 -c kafka -- \
  kafka-configs.sh --bootstrap-server localhost:9092 \
    --alter --add-config 'SCRAM-SHA-512=[password=...]' \
    --entity-type users --entity-name cats-admin
```

### Step 2: K8s Secrets 注入 (per §2.3)

```bash
# cats-admin 凭据
kubectl create secret generic cats-kafka-jaas \
  --namespace=cats-data --from-file=kafka_jaas.conf=/path/to/secret/kafka_jaas.conf

# 16 service 凭据 (per §2.2 ACL)
for service in auth-service user-service project-service task-service \
              file-service ingestion-service asr-service ocr-service \
              subtitle-service office-converter-service render-writer-service \
              translation-core notification-service report-service audit-service; do
  kubectl create secret generic "cats-${service}-kafka" \
    --namespace=cats-core --from-file=kafka_jaas.conf=/path/to/${service}/kafka_jaas.conf
done
```

### Step 3: Namespace + ConfigMap apply

```bash
kubectl apply -f deploy/k3s/cats-kafka-statefulset.yaml  # 含 Namespace + ConfigMap
```

### Step 4: StatefulSet apply

```bash
# StatefulSet 自动滚动创建 3 broker pod
kubectl apply -f deploy/k3s/cats-kafka-statefulset.yaml
kubectl rollout status statefulset/cats-kafka -n cats-data --timeout=300s
```

### Step 5: Topic 创建 (Strimzi KafkaTopic CRD)

```bash
# 需先安装 Strimzi Kafka Operator (per SRE 平台 Lead 实施)
kubectl apply -f deploy/k3s/cats-kafka-topics.yaml  # 10 KafkaTopic CRD
```

### Step 6: Debezium Connect 部署

```bash
# Debezium Connect 集群 (per §3.1)
# 留 K8s 阶段二 SRE 平台 Lead 真人到位后实施
# 当前 Sprint 1 = 阶段一 (single node MVP, 0 Kafka 物理发布)
```

### Step 7: 16 service Kafka client 集成

```bash
# per §4.1 + §4.2, 16 service 集成
# 留 Sprint 2 实施 (per Sprint 1 复盘 §4.2 12.5% 完成度)
```

### Step 8 (Bonus): 验证

```bash
# 验证 Kafka 集群
kubectl exec -n cats-data cats-kafka-0 -c kafka -- \
  kafka-topics.sh --bootstrap-server localhost:9092 --list

# 验证 SASL 鉴权
kubectl exec -n cats-data cats-kafka-0 -c kafka -- \
  kafka-console-producer.sh --bootstrap-server localhost:9092 \
    --producer-property security.protocol=SASL_PLAINTEXT \
    --producer-property sasl.mechanism=SCRAM-SHA-512 \
    --producer-property sasl.username=cats-admin \
    --producer-property sasl.password=$KAFKA_ADMIN_PASSWORD \
    --topic test

# 验证 Debezium CDC
psql -h <postgres-host> -U cats_user -d file_db \
  -c "INSERT INTO file_events_outbox (event_id, event_type, payload) VALUES (...)"
# 检查 Kafka topic
kubectl exec -n cats-data cats-kafka-0 -c kafka -- \
  kafka-console-consumer.sh --bootstrap-server localhost:9092 \
    --consumer-property security.protocol=SASL_PLAINTEXT \
    --consumer-property sasl.mechanism=SCRAM-SHA-512 \
    --consumer-property sasl.username=cats-admin \
    --consumer-property sasl.password=$KAFKA_ADMIN_PASSWORD \
    --topic file.events --from-beginning --max-messages 1
```

---

## 6. 升版流程

### 6.1 升版触发条件

- Kafka 版本升级 (3.7 → 3.8) → 重做 StatefulSet + 滚动升级
- Topic 数量调整 (新增 / 删除) → 更新 cats-kafka-topics.yaml
- SASL 凭据轮换 → 重新生成 + K8s Secret 注入
- Debezium 版本升级 → Connect 集群滚动升级
- 16 service Kafka client 升级 → per service rust-rdkafka 升级

### 6.2 升版流程

1. **PR 起草**：SRE 平台 Lead 主责
2. **DDD Review**：6 角色评审（含 Sponsor 本人签）
3. **CAB 决议**：v1.x → v2.0 需走 CAB-002
4. **基线化**：v 升 B-y.y，Baseline一览 + 接口设计书 §5 同步
5. **引用同步**：告警规则 v1.0 §4 + Sprint 复盘 §4.3 + 画图 v1.0 §3 同步更新

### 6.3 当前 v1.0 适用范围

- **时间窗口**：K3s 阶段二 (per 决议 6 SRE 估算 v1.0 §4)
- **关联 commit**：be712dc (SRE 估算) + 2910f3d (微服务架构) + 1f3c94d (接口设计) + 1d8926d (告警规则) + 0b9cec6 (画图) + 2146f53 (错误码表) + 1b27b2b (启动会决议 6+10)
- **升版预期**：v1.1 = 16 service Kafka client 集成后 (Sprint 2)

---

## 7. 已知缺口 (DDD Review 必查 per AI 协作文档治理 2026-08-26)

### 7.1 GAP-KAFKA-1: K3s 实际部署 0 落地 (per SRE 平台 Lead 真人到位依赖)

- `deploy/k3s/cats-kafka-statefulset.yaml` + `cats-kafka-topics.yaml` 设计落地
- K3s 集群实际部署留 SRE 平台 Lead 真人到位后实施 (per 启动会决议 6 SRE 估算 v1.0 §4)
- **当前状态**: 0 阻塞设计落地, 留 Sprint 2 起点 K3s 部署

### 7.2 GAP-KAFKA-2: Debezium Connect 0 详细 K8s manifest

- §3.1 仅概述 Debezium Connect 集群, 0 详细 K8s yaml
- **建议**: SRE 平台 Lead 真人到位后详细设计
- **当前状态**: 0 阻塞本设计 v1.0 落地

### 7.3 GAP-KAFKA-3: 16 service Kafka client 0 实质 commit (per Sprint 1 复盘 §4.2 12.5%)

- 16 域中 14 域 0 commit, Kafka client 集成 Sprint 2 实施
- 8 个 topic (task.media.ocr.requested / task.media.office.requested) 0 在 interface 文档明确, 留 Sprint 2 补
- **建议**: Sprint 2 W3 实施 task-service + file-service + ingestion-service + asr-service 4 域 Kafka client 集成
- **当前状态**: 0 阻塞本设计 v1.0 落地

### 7.4 GAP-KAFKA-4: Kafka topic 名映射跟 错误码表 v1.0 §5.2 不一致

- 错误码表 v1.0 §5.2 "本表 v1.0 → v1.1 升版时增加 §5.2 Kafka topic 名 per event_type 映射"
- 当前 Kafka topic 10 个 (file.events / task.events / task.media.*.{requested,completed} / audit.events / project.events) 0 在错误码表 v1.0 §5.2 详细列
- **建议**: 错误码表 v1.0 → v1.1 升版时补 §5.2 (per 错误码表 §8.1 + §1.2 升版触发)
- **当前状态**: 0 阻塞本设计 v1.0 落地

### 7.5 GAP-KAFKA-5: DDD Review 6 角色 7 天评审 9/4 截止已逾期 5 天 (per Sprint 1 复盘 §9.1)

- 本 Kafka 物理发布设计 v1.0 + 画图 v1.0 + 告警规则 v1.0 + 接口设计书 v2.0+2 + Sprint 1 复盘 v1.0 + Sprint 概要模板 v1.0 = 6+ commit 待评审
- 5 域 Lead 真人到位率 0%, Mavis 永久代签
- **建议**: Sprint 1 末 9/27 前补 7 天评审窗口
- **当前状态**: 0 阻塞本设计 v1.0 落地

### 7.6 GAP-KAFKA-6: 0 写 SASL 凭据 (per 守门 #5 8/27 11:06 JST hard ban)

- `cats-kafka-jaas` K8s Secret 模板留 SRE 平台 Lead 真人到位后注入
- 0 写实际凭据到 git (per 守门 #5 8/27 11:06 JST hard ban 0 env 打印)
- **建议**: K8s Secrets 由 SRE 平台 Lead 真人到位后通过 sealed-secrets 或 external-secrets 注入
- **当前状态**: 0 阻塞本设计 v1.0 落地

### 7.7 GAP-KAFKA-7: 9/27 截止前 Sprint 2 W3-W4 实施窗口

- Kafka 物理发布 9/27 截止 = T-07 主线 (per 启动会决议 10)
- 当前 Sprint 1 = W2 周五 (9/11) 14:55 JST, W3 (9/14-9/20) + W4 (9/21-9/27) 16 天
- 16 service Kafka client 集成 0 实质 commit (除 auth + user + cats-mock)
- **建议**: W3 实施 4 域 Kafka client 集成 (task / file / ingestion / asr), W4 实施剩余 10 域 + Debezium Connect + 验证
- **当前状态**: 0 阻塞本设计 v1.0 落地, 9/27 截止风险中

---

## 8. 关联文档

| 文档 | 路径 | 用途 |
|------|------|------|
| SRE 平台独立估算 v1.0 | `doc/05-其他/管理/CATs_M1-Sprint1_SRE平台独立估算_v1.0.md` | §4 K3s 阶段二规划 + 真人到位依赖 |
| 微服务架构设计书 v1.0 | `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` | §15 Kafka 部署 + §14 阶段一/二 |
| 接口设计书 v2.0+2 | `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` | §5 Kafka Topics + §6 端到端 |
| 错误码表 v1.0 | `doc/05-其他/管理/CATs_错误码表_v1.0.md` | §5.2 Kafka topic 映射 (升 v1.1 时补) |
| 告警规则 v1.0 | `doc/05-其他/可观测性/CATs_告警规则_v1.0.md` | §4 实施 + §6.5 Kafka 物理发布留 K3s 阶段二 |
| 画图 v1.0 | `doc/02-基础设计/架构设计/CATs_画图_v1.0.md` | §3 部署架构 K3s 阶段二 |
| 启动会决议纪要 v1.0 | `doc/05-其他/会议记录/CATs_M1_Sprint1_启动会决议纪要_v1.0.md` | 决议 6 SRE + 决议 10 Kafka + 错误码闭环 |
| Sprint 1 复盘 v1.0 | `doc/05-其他/复盘纪要/CATs_Sprint1_复盘纪要_v1.0.md` | W1+W2 12 天复盘 + Sprint 2 范围初稿 |
| K8s manifests | `deploy/k3s/cats-kafka-statefulset.yaml` + `cats-kafka-topics.yaml` | K3s 阶段二部署 |

---

**Kafka 物理发布设计结束 (v1.0, 2026-09-11, T-07 Kafka 物理发布部分设计落地, 7 实施步骤 + 7 已知缺口 + 8 关联文档引用)**
