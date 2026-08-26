# CATs ADR-003：数据存储与检索选型

> **文档编号**：CATs-ADR-003
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师 + DBA
> **状态**：已接受（待 8-25 评审会复核 QA-011 / QA-041）
> **取代**：—

---

## 1. 背景

不同服务对存储的需求差异大：

- **OLTP**：用户/项目/订单/权限等关系型
- **TM / 术语**：文本检索 + 向量检索
- **CRDT 协同**：高写入频、低冲突合并
- **事件**：append-only、可回放
- **缓存**：高频读、低延迟
- **对象**：翻译记忆附件、语料库

## 2. 选项

| 类别 | 选型 | 替代 | 决策理由 |
|------|------|------|----------|
| **OLTP 关系库** | **PostgreSQL 16** | MySQL 8 | 强事务、JSONB、生成列、扩展生态（pgvector / PostGIS） |
| **向量检索** | **pgvector（HNSW）** ✅ | Milvus / Weaviate / Qdrant | 与 PG 同库降低运维；10万级实测性能待 QA-041 验证 |
| **文本检索** | **PG tsvector + GIN** | Elasticsearch | 与 TM 共库；ES 集群成本不必要 |
| **CRDT 持久化** | **Yjs snapshot + PG JSONB** | Redis JSON | 持久化、版本快照 |
| **事件流** | **Kafka** ✅ | NATS / Pulsar | 已在 ADR-002 决策 |
| **缓存** | **Redis 7（Cluster）** | Dragonfly | 成熟、生态完善 |
| **对象存储** | **S3 兼容（MinIO 自托管）** | AWS S3 | 多云中立、合规可控 |

## 3. 决策

**PG 中心化 + 多专用存储分层**：

```
┌─────────────────────────────────────────────────┐
│              应用服务层（15 服务）                 │
├─────────────────────────────────────────────────┤
│  OLTP  ┌─ PostgreSQL 16（shared cluster, schemas 隔离）│
│  向量  │  ├─ pgvector（HNSW）                      │
│  文本  │  └─ tsvector + GIN                        │
│  缓存  ├─ Redis 7 Cluster                          │
│  事件  ├─ Kafka 3.x KRaft                          │
│  对象  └─ MinIO（S3 兼容）                          │
└─────────────────────────────────────────────────┘
```

**Schema 隔离策略**：单 PG cluster，按服务分配 schema（tm/term/project/auth/billing/admin），跨服务禁止跨 schema 访问，统一通过 API 编排。

## 4. 影响

- **正面**：
  - PG 一套工具链（备份/监控/高可用）覆盖 80% 场景
  - pgvector 避免引入独立向量库
  - 运维统一 → SRE 友好（呼应 QA-012）
- **负面**：
  - 大 TM 表 + HNSW 索引对单实例 PG 压力大 → 需做读副本 + 分区
  - pgvector 性能基线需 QA-041 实测
  - MinIO 自托管需 SRE 投入
- **风险**：
  - QA-011：TM 索引策略（全表 vs 分桶 vs scale-out）→ 待 8-25 评审会
  - QA-041：pgvector 10000+ 条 HNSW 实测 → 已在基准测试中

## 5. 关联

- 上游：ADR-001、ADR-002
- 下游：CATs_数据库设计书_v2.0、CATs_SQL设计一览_v1.0
- 阻塞项：QA-011、QA-041
