# CATs 性能試験（PT）レポート テンプレート v1.1

> **文档编号**：CATs-ST-PT-TPL-001
> **バージョン**：v1.1
> **作成日**：2026-08-26
> **作者**：架构师 + Rust Lead + DBA（worker 代签 per DEC-008）
> **状態**：M2 末触发（P2M3待触发索引 v1.0 §3.5 #50 / テスト設計書 v1.0 §10.3）
> **配套**：CATs_テスト設計書 v1.0 §4.5 / §10.3 / CATs_ADR-001 §3 / CATs_ADR-002 §3 / CATs_ADR-003 §3 / CATs_ADR-004 §3 / CATs_ADR-005 §2 / [CATs_技术基线_v1.0](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md) §1（PostgreSQL 18.6 + Rust 1.98.0）

---

## 文档管理信息

### 审批栏

| 役割 | 氏名 | 承認 | 日付 | 備考 |
|------|------|------|------|------|
| 起案 | 架构师 + Rust Lead + DBA（worker 代签 per DEC-008） | ☑ | 2026-08-26 | v1.1 テンプレ |
| レビュー | — | ☐ | — | — |
| 承認 | — | ☐ | — | — |

### 修订履历

| バージョン | 日付 | 修订者 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-26 | SRE + QA + アーキテクト（worker 代签 per DEC-008） | M2 末触发：環境 / シナリオ / 指標 / 結果 / 問題 / 改善 / 署名 7 章テンプレ |
| **v1.1** | **2026-08-26** | **架构师 + Rust Lead + DBA** | **基线升级：PostgreSQL 16.x → 18.6（CloudNativePG 1.30+）；Rust toolchain 1.81.x → 1.98.0（均引用 CATs_技术基线_v1.0 §1）** |

---

## 0. 适用范围

- **报告类型**：性能試験（Performance Test, PT）报告，每次正式性能试验输出 1 份
- **关联任务**：性能試験（任务 80 / P2M3待触发索引 v1.0 §3.5 #50）
- **上游文档**：CATs_テスト設計書 v1.0 §4.5（性能 SLO）、§10.3（CI/CD 集成与门禁）
- **下游文档**：CATs_障害試験レポート / セキュリティ試験レポート / GoNoGo 決議書
- **ADR 引用**：
  - ADR-001 §3（15 サービス + 4 共有ライブラリ清单）
  - ADR-002 §3（gRPC + Kafka 通信 / Consumer lag / event 流量）
  - ADR-003 §3（PG 中心化 + pgvector + barman）
  - ADR-004 §3（Tauri / Web 控制台 / 协同 WS 性能）
  - ADR-005 §2（Keycloak 多租户 / RBAC+ABAC 隔离对性能影响）

---

## 1. 试验环境

### 1.1 硬件

| 节点 | 数量 | 机型 | 用途 |
|------|------|------|------|
| K3s 控制面 | 3 | 4C8G | 控制面 + etcd |
| Worker (general) | 6 | 8C16G | 无状态应用 |
| Worker (stateful) | 3 | 8C32G | PG / Kafka / MinIO |
| PG primary | 1 | 8C32G + 500GB NVMe | 主库 |
| PG replica | 2 | 8C32G + 500GB NVMe | 从库 |
| Kafka broker | 3 | 8C32G + 1TB NVMe | 3 broker RF=3 |
| Redis | 3 | 4C16G | Cluster 模式 |
| MinIO | 4 | 4C8G + 4TB HDD | EC:2 |
| 压测源（k6/ghz/wrk2） | 3 | 16C32G | 客户端分布 |
| 监控（Prometheus / Grafana / Tempo / Loki） | 1 | 8C16G + 500GB SSD | 观测栈 |

### 1.2 软件版本

| 组件 | 版本 |
|------|------|
| K3s | 1.30.x |
| PostgreSQL | 18.6（CloudNativePG 1.30+） |
| Kafka | 3.7.x（KRaft） |
| Redis | 7.2.x |
| MinIO | RELEASE.2024-xx |
| Keycloak | 24.x |
| Rust toolchain | 1.98.0 |
| Node.js | 20 LTS |
| Python | 3.12 |
| k6 | 0.49.x |
| ghz | 0.1.x |
| wrk2 | 5.x |
| Tauri | 1.6.x |

> **版本基线**：PostgreSQL 18.6 + CloudNativePG 1.30+ + Rust 1.98.0 + pgvector 0.8.6，引用 [CATs_技术基线_v1.0 §1](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md)。

### 1.3 网络

- 1 Gbps 内部网络
- 跨 AZ 模拟：100ms RTT + 0.1% 丢包
- 客户端 ↔ BFF 模拟 50ms RTT

### 1.4 数据规模

| 数据 | 规模 | 备注 |
|------|------|------|
| TM 记录 | 100K / 1M（按场景） | pgvector HNSW 索引就绪 |
| 术语 | 10K | |
| 用户 | 50K | 含 5K 在线峰值 |
| 项目 | 5K | |
| Kafka 积压 | < 1K events | 重放回放前清空 |

### 1.5 测试数据集

- 来自 PRE 环境的脱敏生产快照
- 多语言（中 / 日 / 英）
- 多媒体类型覆盖（文本 / Office / PDF / 视频 / 音频）

---

## 2. 试验场景

### 2.1 场景矩阵（来自テスト設計書 §4.5）

| 场景 | 目的 | 并发 | 数据规模 | 时长 |
|------|------|------|----------|------|
| **S1: 空载（Baseline）** | 建立基线 | 10 | 1K | 30 min |
| **S2: 中等（Normal）** | 验证 SLO | 500 | 100K | 2 h |
| **S3: 峰值（Peak）** | 验证容量 | 1000 | 100K | 1 h |
| **S4: 突发（Spike）** | 验证弹性 | 100→2000 瞬时 | 100K | 30 min |
| **S5: 浸泡（Soak）** | 验证稳定性 | 500 | 1M | 24 h |
| **S6: 容量（Volume）** | 验证大数据量 | 100 | 1M | 2 h |
| **S7: 故障叠加（Resilience）** | 验证韧性 | 500 | 100K | 1 h |

### 2.2 场景执行顺序

1. S1 → 建立基线
2. S2 → 核心 SLO 验证
3. S3 → 容量上限
4. S4 → 弹性
5. S5 → 稳定性
6. S6 → 大数据量
7. S7 → 故障叠加（与 障害試験レポート 联动）

---

## 3. 指标

### 3.1 应用层指标

| 指标 | 维度 | 目标（来自 ADR-001/003 + テスト設計書 §4.5）|
|------|------|-------|
| QPS | 服务 / endpoint | 服务级 SLO（キャパシティ管理計画 §1.4） |
| P50 / P95 / P99 延迟 | 服务 / endpoint | L0 < 100ms / L1 < 300ms / L2 < 1s / L3 < 10s |
| 错误率（4xx / 5xx） | 服务 | < 0.1% / < 0.5% |
| 吞吐量 | 服务 | req/s |
| 成功率 | 业务（翻译 / TM / 协同） | ≥ 99.5% |

### 3.2 资源层指标

| 指标 | 维度 | 阈值 |
|------|------|------|
| CPU 使用率 | 节点 / Pod / namespace | < 70% |
| 内存使用率 | 节点 / Pod | < 80% |
| 网络 I/O | 节点 / NIC | < 70% 容量 |
| 磁盘 IOPS | PG / Kafka | < 80% 容量 |
| 磁盘使用率 | 节点 / PV | < 75% |

### 3.3 中间件指标

| 组件 | 指标 | 目标 |
|------|------|------|
| PG | 活跃连接 / max_connections | < 80% |
| PG | 复制延迟 | < 5s |
| PG | WAL 增长 | < 10 GB/h |
| PG | 慢查询（> 1s）| < 0.1% |
| PG | lock waits | 0 长锁 |
| Kafka | Consumer lag | < 1000 |
| Kafka | ISR 收缩 | 0 |
| Kafka | DLQ 积压 | 0 |
| Redis | 内存使用 | < 70% maxmemory |
| Redis | hit rate | > 95% |

### 3.4 业务指标

| 指标 | 说明 |
|------|------|
| 翻译质量（人工抽样） | BLEU ≥ 0.4 / 客户评分 ≥ 4.0/5 |
| TM 召回率 | L0 ≥ 30% / L1 ≥ 50% |
| 协同并发连接 | 设计容量 80% 内 |
| 任务成功率 | ≥ 99% |

### 3.5 延迟分层 SLO（来自テスト設計書 §4.5）

| 层级 | 条件 | 目标延迟 | 路径 |
|------|------|----------|------|
| L0 | TM 精确匹配 | < 100ms p95 | 直接填充 |
| L1 | TM 模糊匹配 | < 300ms p95 | 填充+等待确认 |
| L2 | TM 未命中（默认） | < 1s p95（首字） | 单模型+术语+QA |
| L3 | 用户指定 | < 10s p95（完成） | 多模型投票+评审 |

---

## 4. 结果对比（SLO vs 实测）

### 4.1 场景 S2 核心结果（示例）

| 服务 | SLO QPS | 实测 QPS | SLO P95 | 实测 P95 | SLO 错误率 | 实测错误率 | 判定 |
|------|---------|----------|---------|----------|-----------|-------------|------|
| auth-service | 2000 | X | < 50ms | X | < 0.1% | X% | ✅/❌ |
| user-service | 1500 | X | < 80ms | X | < 0.1% | X% | ✅/❌ |
| project-service | 1000 | X | < 100ms | X | < 0.1% | X% | ✅/❌ |
| task-service | 800 | X | < 150ms | X | < 0.2% | X% | ✅/❌ |
| file-service | 500 | X | < 200ms | X | < 0.2% | X% | ✅/❌ |
| translation-core | 300 | X | < 300ms | X | < 0.3% | X% | ✅/❌ |
| llm-gateway | 200 | X | < 1000ms | X | < 0.5% | X% | ✅/❌ |
| tm-service | 1000 | X | < 100ms | X | < 0.1% | X% | ✅/❌ |
| term-service | 800 | X | < 80ms | X | < 0.1% | X% | ✅/❌ |
| collab-ws | 5000 conn | X | < 50ms | X | < 0.1% | X% | ✅/❌ |
| qa-engine | 300 | X | < 500ms | X | < 0.3% | X% | ✅/❌ |

> X = 实测值，每场景独立填写。

### 4.2 延迟分层达成

| 层级 | 目标 P95 | 实测 P95 | 目标 P99 | 实测 P99 | 判定 |
|------|----------|----------|----------|----------|------|
| L0 | < 100ms | X | < 200ms | X | ✅/❌ |
| L1 | < 300ms | X | < 500ms | X | ✅/❌ |
| L2 | < 1s | X | < 2s | X | ✅/❌ |
| L3 | < 10s | X | < 15s | X | ✅/❌ |

### 4.3 资源利用率（场景 S3 峰值时）

| 资源 | 警戒 | 实测峰值 | 余量 | 行动 |
|------|------|----------|------|------|
| 节点 CPU | 70% | X% | Y% | — |
| Pod 内存 | 80% | X% | Y% | — |
| PG CPU | 70% | X% | Y% | — |
| PG 连接 | 80% | X% | Y% | — |
| Kafka broker CPU | 70% | X% | Y% | — |
| Redis 内存 | 70% | X% | Y% | — |

### 4.4 场景 S5 浸泡（24h）关键趋势

| 指标 | 起始 | 6h | 12h | 18h | 24h | 漂移 |
|------|------|----|----|----|----|------|
| P95 延迟 | X | X | X | X | X | X% |
| 内存 RSS | X | X | X | X | X | X% |
| 错误率 | X% | X% | X% | X% | X% | X% |
| FD 数量 | X | X | X | X | X | X |
| GC 次数 | X | X | X | X | X | X |

### 4.5 场景 S4 突发（Spike）

| 指标 | 1k 基线 | 瞬时 2k | 恢复后 | 判定 |
|------|---------|---------|--------|------|
| P95 延迟 | X | X | X | 是否雪崩 / 是否恢复 |
| 错误率 | X% | X% | X% | — |
| HPA 扩容 | 副本数 | 副本数 | 副本数 | 是否 5 min 内响应 |
| 队列堆积 | X | X | X | 是否吸收 |

### 4.6 场景 S7 故障叠加（与 障害試験 联动）

| 故障注入 | 影响范围 | 业务影响 | 恢复时间 | 判定 |
|----------|----------|----------|----------|------|
| K3s 控制面 1/3 节点宕机 | K8s API | 业务感知 | X min | ✅/❌ |
| PG primary 重启 | 写入 30s 中断 | 业务感知 | X min | ✅/❌ |
| Kafka broker 1/3 宕机 | 跨服务事件 | 部分延迟 | X min | ✅/❌ |
| llm-gateway 上游故障 | 翻译链路 | 降级 | X min | ✅/❌ |
| collab-ws 进程崩溃 | WS 连接 | 重连 | X min | ✅/❌ |

---

## 5. 问题清单

| # | 问题 | 严重度 | 根因 | 影响 | 责任 |
|---|------|--------|------|------|------|
| PT-001 | 例：llm-gateway P95 超 SLO 50% | P1 | 上游 LLM 厂商限流 | L2 翻译延迟 | SRE + 厂商 |
| PT-002 | 例：tm-service pgvector 模糊匹配 > 300ms | P1 | 索引 ef_search 未调优 | L1 召回 | SRE + DBA |
| ... | | | | | |

> 问题与 §6 优化建议一一对应。

---

## 6. 优化建议

| # | 建议 | 预期效果 | 工作量 | 优先级 | 责任 |
|---|------|----------|--------|--------|------|
| OPT-001 | llm-gateway 增加备选厂商 + 智能路由 | L2 P95 降 30% | M | P1 | 架构 + SRE |
| OPT-002 | pgvector 调优 ef_search + 建复合索引 | L1 降 50% | S | P1 | DBA + SRE |
| OPT-003 | HPA 扩 Kafka consumer 副本 + KEDA | 减少 lag | S | P2 | SRE |
| OPT-004 | Redis cluster 扩分片 | hit rate 提升 | S | P2 | SRE |
| ... | | | | | |

**工作量分类**：S = 1-3d / M = 1-2w / L = 1 月+。

---

## 7. 签字

| 角色 | 姓名 | 签字 | 日期 | 备注 |
|------|------|------|------|------|
| 试验负责 |  |  |  | 试验执行 + 报告编写 |
| QA Lead |  |  |  | 报告 review |
| SRE Lead |  |  |  | 性能基线签字 |
| 架构师 |  |  |  | 架构层面签字 |
| PMO |  |  |  | 任务关闭 |

---

## 8. 附录

### 8.1 工具命令参考

```bash
# k6 HTTP 压测
k6 run --vus 500 --duration 30m scripts/http-load.js

# ghz gRPC 压测
ghz --insecure --proto proto/cats.proto \
  --call cats.TranslationService.Translate \
  -d '{"source":"en","target":"ja","text":"hello"}' \
  -c 100 -z 30m localhost:50051

# wrk2 高精度延迟
wrk2 -t10 -c500 -d30m -R 5000 -L \
  -s scripts/wrk-translate.lua https://bff.cats.internal

# Kafka producer/consumer
kafka-producer-perf-test.sh --topic cats.test --num-records 1000000 \
  --record-size 1024 --throughput 10000 --producer-props bootstrap.servers=...
```

### 8.2 Grafana 仪表盘

- `cats-perf-overview`：全局概览
- `cats-perf-service-<svc>`：服务级
- `cats-perf-middleware`：PG / Kafka / Redis
- `cats-perf-business`：业务指标

### 8.3 数据归档

- k6 / ghz JSON 输出 → `infra/perf-results/YYYY-MM-DD/`
- Prometheus 记录 → 长期存储 13 月
- Grafana 仪表盘快照 → 永久
- 报告 PDF → `doc/04-测试/ST/reports/`

---

## 9. 上游 / 下游文档

| 上游 | 引用 | 章节 |
|------|------|------|
| CATs_テスト設計書 v1.0 §4.5 | 性能 SLO + 场景 | §2, §3 |
| CATs_テスト設計書 v1.0 §10.3 | CI/CD 集成与门禁 | §0 |
| ADR-001 §3 | 15 服务清单 | §3.1 |
| ADR-002 §3 | gRPC + Kafka 通信（Consumer lag、event 流量） | §3.3 |
| ADR-003 §3 | PG 中心化 + barman | §1.1, §3.3 |
| ADR-004 §3 | Tauri / Web 控制台 / 协同 WS 性能 | §3.4 |
| ADR-005 §2 | Keycloak 多租户 / RBAC+ABAC 隔离对性能影响 | §3.1, §3.4 |
| キャパシティ管理計画 v1.0 §1.4 | 服务级 SLO | §3.1 |

| 下游 | 关系 |
|------|------|
| CATs_障害試験レポート v1.0 | §4.6 故障联动 |
| CATs_セキュリティ試験レポート v1.0 | 并行输出 |
| GoNoGo 決議書 v1.0 | 上线前输入 |
| キャパシティ管理計画 v1.x | SLO baseline 更新 |

---

## 10. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | k6 / ghz 脚本库沉淀到 `infra/perf-scripts` | SRE + QA | M2 末 |
| OI-2 | 性能 baseline 自动化对比 | SRE | M3 上线前 |
| OI-3 | 业务指标（翻译质量）人工抽样模板 | QA | M2 末 |
| OI-4 | 长期 24h 浸泡 SRE 当班 | SRE | 每次试验 |

---

**模板结束（v1.0）**

> **使用说明**：每次正式性能试验，复制本模板 → 填入实测值（X → 实际数字）→ §5/§6 与问题/优化项一一对应 → 签字归档。
