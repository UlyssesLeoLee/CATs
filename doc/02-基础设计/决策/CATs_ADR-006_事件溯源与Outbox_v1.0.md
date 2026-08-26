# CATs ADR-006：事件溯源与 Outbox 模式选型

> **文档编号**：CATs-ADR-006
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师(worker 代签 per DEC-008)
> **状态**：已接受
> **取代**：—

---

## 1. 背景

CATs 15 微服务在分布式事务与跨域事件一致性上需要明确策略。ADR-001 已落定微服务架构，ADR-002 已落定 gRPC + Kafka 通信底座，但"业务事件如何可靠落地"尚未决策。

要决策的核心问题：

- **强一致性 vs 最终一致性**：translation/document/project 等核心域的"写库 + 发 Kafka"如何保证原子性?
- **是否引入事件溯源(Event Sourcing, ES)**：核心域是否需要"事件流为真相源、状态为投影"?
- **M3 时间线约束**：8-25 / 8-28 评审会后只剩 14 周，复杂方案的收益是否匹配成本?

QA-014 已登记为 Open 项，要求评审会前给出明确决策。L-7(架构风险登记册)已将"分布式事务"识别为 ADR-001 的衍生风险，需 Outbox 应对——但 Outbox 与 ES 是不同抽象层级，决策不能合并。

核心域现状：

- **translation**：长任务(LLM 流式翻译、断点续传)→ 高价值 ES 候选
- **document**：版本快照、CRDT 协同 → 已通过 Yjs 解决协同一致性，不需要 ES
- **project**：聚合根传统 CRUD → 强一致性需求中等
- **tm / term / qa-engine**：高读、低写，事件需求弱

## 2. 选项

| 选项 | 描述 | 优势 | 劣势 |
|------|------|------|------|
| **A. Outbox only** ✅ | 业务表 + outbox 表同事务写，Debezium CDC 投递 Kafka | 实现成本低、运维成熟、与现有 PostgreSQL 中心化(ADR-003)契合、不改变业务模型 | 状态仍为权威源，事件仅作通知；不能"事件回放"重建状态 |
| **B. 部分核心域 ES** | translation 域引入 ES(事件为权威源，状态为投影) | 可回放、可审计、断点续传友好 | 双写迁移、projection 重建、快照策略、事件版本治理 4 项工程量均重；与 M3 时间线冲突 |
| **C. 全 ES** | 15 服务全部事件化 | 终极一致性、审计完备 | 工作量爆炸、团队能力缺口、查询模型要 CQRS 全套；不现实 |

补充说明：

- **选项 A 与 B 不互斥**：A 是"消息可靠投递"层，B 是"领域建模"层。本 ADR 决策"消息投递层全部用 Outbox"+"领域建模层除 translation 外不引入 ES"。
- 选项 C 即使在 6 个月窗口也不可行，与本项目无对比意义，仅列出以排除"为什么不选全 ES"。

## 3. 决策

**采用 A：Outbox 模式作为分布式事务的可靠投递底座；核心域(translation / document / project)暂不引入事件溯源。**

实施细则：

1. **Outbox 模式强制落地**
   - 所有"写业务表 + 发 Kafka"场景必须用 outbox 表同事务写入
   - Debezium CDC 监听 outbox 表 → 投递到 Kafka
   - 共享库 `common-grpc` 提供 `OutboxWriter` trait(参见 ADR-002 共享库分层)
2. **translation 域延后 ES 评估**
   - M1-M3 采用"长任务状态机 + 周期 checkpoint 写 DB"实现断点续传
   - M4+ 视运行数据(任务平均时长、断点续传失败率)再评估是否升 ES
   - 评估触发条件写入 `doc/03-详细设计/translation/ES_延后评估触发条件.md`(本 ADR 不立此文件，列入 M3 末评审议程)
3. **document 域不引入 ES**
   - Yjs CRDT 已解决协同一致性，版本快照走"周期快照 + delta 链"传统方案
4. **project 域不引入 ES**
   - 聚合根 CRUD 走 PG 强一致 + outbox 事件通知
5. **审计 / notify 域消费 outbox 事件**
   - 不直接监听业务表，事件流即审计流

## 4. 影响

- **正面**：
  - 与 ADR-001 / ADR-002 / ADR-003 已落定架构完全兼容，无架构冲突
  - Outbox 是业界标准模式(LinkedIn / Uber / Shopify 大规模生产验证)，风险可控
  - M1 即可落地，团队无学习成本(只需遵循 common-grpc 的 OutboxWriter 约定)
  - 保留未来在 translation 域升 ES 的选项，不锁死
- **负面**：
  - 业务表多一张 outbox 表(单服务 1 张，15 服务共 15 张)，需容量评估
  - Debezium CDC 增加运维组件(已在 53 任务 CI/CD 范围)
  - 事件不参与状态重建 → 审计追溯只能看事件流快照，不能从事件回放任意时间点状态
- **风险**：
  - 若 translation 域 M3 末评估显示 ES 收益大于成本，需追加 1-2 周迁移预算 → 需在 Q-031 WBS 预留 buffer
  - outbox 表膨胀 → 需建立周期清理任务(按事件已被下游消费 + 保留期 30 天)
  - Debezium 与 PostgreSQL 主版本绑定 → PostgreSQL 升级时需同步验证

## 5. 关联

- **上游**：ADR-001(微服务架构)、ADR-002(gRPC + Kafka 通信)、ADR-003(PostgreSQL 中心化)
- **下游**：translation 域详细设计 v1.0(Checkpoint 机制)、可观测性平台设计 v1.0(outbox lag 指标)
- **阻塞项**：QA-014(事件溯源 / Outbox 选型)
