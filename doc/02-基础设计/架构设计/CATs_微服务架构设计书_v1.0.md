# CATs 微服务架构设计书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-ARCH-001 |
| 文档名 | 微服务架构设计书（K3s / 局域网私有化 SaaS，50–3000 并发用户） |
| 版本 | 第 1.0 版（草稿） |
| 创建日 | 2026-08-17 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [CATs 技术选型书 v2.0](../技术选型/CATs_技术选型书_v2.0.md)、[CATs 命名变更说明](./CATs_命名变更说明.md)、[OFCAT 基础设计书 v1.0（历史/旧架构参考）](./OFCAT_基础设计书_v1.0.md)、[OFCAT 需求定义书 v1.1（历史需求输入）](../../01-需求/需求规格说明/OFCAT_需求定义书_v1.1.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-08-17 | 架构师 | 初版。OFCAT 单机架构向 CATs 微服务 SaaS 架构转型，含 K3s/Kafka/PostgreSQL/Envoy Gateway/Outbox+CDC/GitOps 全套设计与三阶段落地路线图 |

### 审批栏

| 角色 | 姓名 | 审批日 | 签字 |
|---|---|---|---|
| 起草 | | | |
| 评审 | | | |
| 批准 | | | |

---

## 0. 阅读指南

本书是 CATs 由「薄浏览器扩展 + 本地引擎」转型为「Rust 原生客户端 + 微服务 SaaS 平台」后的**基础设计书**，覆盖从局域网用户接入到数据落库的完整链路。所有章节遵循同一条主线原则：**先问是否需要，再问怎么做**（见 §1）。技术选型的候选对比与 ADR 见配套的 [CATs 技术选型书 v2.0](../技术选型/CATs_技术选型书_v2.0.md)，本书聚焦"怎么组合、怎么落地、怎么演进"。

---

## 1. 架构目标与设计原则

### 1.1 架构目标

| 目标 | 说明 |
|---|---|
| 支撑规模 | 50–3000 并发用户，局域网内部署，非公网大规模互联网服务 |
| 全媒体翻译 | 文本、音频、视频、PDF、图文文档、Office 全家桶（docx/xlsx/pptx + ODF）、动图（GIF/WebP）统一走同一翻译核心，媒体特定环节以可插拔微服务承接 |
| 开源免费可商用 | 全栈组件许可证均允许商用部署，不产生软件授权费用 |
| 局域网友好 | 全部组件可离线安装/镜像自托管，不强依赖公网服务可用性 |
| 渐进式落地 | MVP → 扩容 → 高可用三阶段，每阶段都是可独立上线运行的完整系统，而非半成品 |
| 可观测/可回滚 | 全链路 Trace、GitOps 声明式部署、分钟级回滚 |

### 1.2 设计原则（延续 OFCAT 原则并扩展）

1. **确定性优先**：TM/术语/标签/时间轴对齐等确定性逻辑用算法保证正确性，不依赖模型自觉（延续 OFCAT 原则）。
2. **数据存储单一权威**：PostgreSQL 是唯一权威数据存储；Valkey 仅做缓存/会话/限流/锁，Kafka 仅做事件传输，任何服务都不得把 Valkey/Kafka 当作业务数据的最终真相来源。
3. **合规 fail-closed**：敏感内容处理链路（尤其涉及云端 ASR/OCR/翻译 API）失败时中止而非降级泄露。
4. **服务边界按数据/媒体类型划分，不按团队组织架构划分**：每个微服务拥有独立的数据库 schema/逻辑库，杜绝跨服务直接读写他人数据库。
5. **异步优先，但不滥用**：只有"确实需要解耦时序/需要多消费者广播/需要削峰"的场景才引入 Kafka 异步链路；简单的同步查询/写入优先走 REST/gRPC 同步调用（详见 §8）。
6. **不过度设计**（核心判定标准）——**任何新增组件/中间件必须在架构评审中明确回答以下六问**，六问任一答案指向"不需要"则应先延后或简化：

| 判定问题 | 说明 |
|---|---|
| Q1 解决什么具体问题 | 必须能指出当前架构中的一个具体缺陷/风险/瓶颈，不能是"业界都这么做" |
| Q2 当前规模是否真的需要 | 结合 50–3000 用户量级、当前 QPS/数据量估算，是否已触及不引入该组件就无法满足的临界点 |
| Q3 不引入会怎样 | 明确当前不引入的最坏后果，评估是否可接受/可延后 |
| Q4 是否有更简单方案 | 是否可以用已有组件的现有能力（如 PostgreSQL LISTEN/NOTIFY、定时任务）替代，而非引入新中间件 |
| Q5 是否开源免费、可商用 | 排除任何有商用限制/云锁定的方案 |
| Q6 是否适合局域网私有化部署 | 能否离线安装、不强依赖外网 SaaS |

  本文档中出现的 Kafka、CDC、Saga 等重量级组件，均在对应章节按此六问给出结论（见 §6、§7、§8）。

---

## 2. 总体推荐架构图（分层拓扑）

由于是纯文本文档，以下用分层列表 + ASCII 图表达从局域网用户到数据层的完整路径。

```
┌──────────────────────────────────────────────────────────────────────────┐
│  局域网用户层                                                              │
│  ┌────────────────────────┐        ┌───────────────────────────────────┐ │
│  │ CATs 客户端 (Tauri 2.x)  │        │ 浏览器 → CATs Web 控制台 (Next.js)   │ │
│  │ Rust 核心 + Svelte UI    │        │ 自助管理/项目/术语/账单/管理后台      │ │
│  │ 日常翻译工作台            │        │                                     │ │
│  └───────────┬─────────────┘        └───────────────┬───────────────────┘ │
└──────────────┼──────────────────────────────────────┼─────────────────────┘
               │ HTTPS/REST/gRPC/WebSocket（内网 DNS + TLS，内部 CA）        │
               ▼                                       ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  接入层：Envoy Gateway（Kubernetes Gateway API）— K3s 集群边缘               │
│  Gateway(cats-gw) → HTTPRoute / GRPCRoute 按 Host+Path 分流                │
│  能力：TLS 终结、HTTP/2、gRPC、WebSocket 升级、限流(RateLimitPolicy)、        │
│        熔断/重试(BackendTrafficPolicy)、灰度分流(权重路由)                    │
│  LB 接入：MetalLB（裸金属局域网 L2/BGP 模式对外暴露 LoadBalancer IP）         │
└───────────────────────────────┬────────────────────────────────────────-─┘
                                 ▼
┌──────────────────────────────────────────────────────────────────────────┐
│  核心业务微服务层（K3s 工作节点，Namespace: cats-core）                        │
│                                                                            │
│  身份/账户域           项目/任务域              媒体处理域（新增/全媒体扩展）    │
│  ┌───────────┐        ┌───────────┐          ┌──────────────────┐        │
│  │auth-service│        │project-svc│          │ ingestion-service │        │
│  │user-service│◀──────▶│task-service│◀───────▶│ asr-service        │        │
│  └───────────┘  gRPC   └───────────┘  gRPC    │ ocr-service         │        │
│                                                │ subtitle-service    │        │
│  支撑域                                        │ office-converter-svc│        │
│  ┌───────────┐        ┌───────────┐          │ render-writer-svc   │        │
│  │file-service│        │notif-svc  │          └──────────────────┘        │
│  │report-svc  │        │audit-svc  │          翻译核心域                    │
│  │worker-svc  │        └───────────┘          ┌──────────────────┐        │
│  └───────────┘                                │ translation-core  │        │
│                                                │（TM/术语/QA/编排）  │        │
│                                                └──────────────────┘        │
└───────┬──────────────┬──────────────┬─────────────────┬──────────────────┘
        │ 同步 SQL      │ 缓存/限流/锁   │ 异步事件(Kafka)   │ CDC(Debezium)
        ▼              ▼               ▼                 ▼
┌───────────────┐ ┌───────────┐  ┌─────────────────┐ ┌──────────────────┐
│ PostgreSQL 集群 │ │  Valkey   │  │ Kafka (KRaft)     │ │ Kafka Connect     │
│ (CloudNativePG) │ │ 缓存/会话  │  │ 3 broker          │ │ + Debezium        │
│ 逻辑库隔离       │ │ 限流/锁    │  │ topic-per-domain  │ │ 监听 outbox 表/WAL │
│ + PgBouncer      │ │ 热数据缓存 │  │                   │ │                   │
│ + pgvector       │ └───────────┘  └─────────────────┘ └──────────────────┘
└───────────────┘
        ▲
        │ 镜像拉取/推送
┌───────┴────────────────────────────────────────────────────────────────┐
│  平台支撑层（Namespace: cats-platform）                                    │
│  Harbor(私有镜像仓库) │ Prometheus+Alertmanager+Grafana │ OTel Collector    │
│  +Tempo(链路) │ Loki+Promtail(日志) │ Argo CD(GitOps) │ Gitea/CI(构建)      │
└───────────────────────────────────────────────────────────────────────-─┘

K3s 集群拓扑：
  控制面 × 3（HA，跨机架/物理机） ── etcd 内嵌 HA（--cluster-init）
  工作节点 × N（按角色打标签/污点分池）：
    pool=general      （通用业务微服务，无 GPU）
    pool=media-cpu     （ffmpeg/LibreOffice/OCR CPU 推理，CPU 密集）
    pool=media-gpu     （ASR/OCR GPU 加速，打 GPU 污点，需 toleration）
    pool=data          （PostgreSQL/Kafka/Valkey 有状态组件，本地盘/独立存储）
```

### 2.1 请求路径示例（同步）

`Tauri 客户端 → Envoy Gateway(HTTPRoute /api/v1/*) → auth-service（鉴权）→ project-service/task-service → PostgreSQL（project_db/task_db）→ 返回`

### 2.2 请求路径示例（异步媒体处理）

`客户端上传视频 → Envoy Gateway → ingestion-service（落盘对象存储/共享卷 + 写 task_db + 写 outbox 表）→ Debezium 捕获 outbox → Kafka(topic: task.media.events) → asr-service/ocr-service/subtitle-service 消费 → translation-core 消费翻译请求事件 → render-writer-service 消费"翻译完成"事件产出目标格式文件 → notification-service 消费"任务完成"事件推送客户端（WebSocket/SSE）`

---

## 3. 完整技术栈清单

| 分类 | 组件 | 必须使用 / 可暂缓 / 属于过度设计 | 说明 |
|---|---|---|---|
| 容器编排 | K3s | **必须** | 集群基座 |
| 入口网关 | Envoy Gateway (Gateway API) | **必须** | 统一南北向流量入口 |
| 裸金属负载均衡 | MetalLB | **必须** | 局域网无云 LB，需 L2/BGP 模式暴露 Service |
| 硬件负载均衡冗余 | HAProxy + Keepalived（VIP） | 可暂缓 | 仅当 MetalLB 单一模式不满足跨网段/更强 HA 需求时引入，MVP 阶段 MetalLB 足够 |
| 主存储 | PostgreSQL 16 + CloudNativePG | **必须** | 唯一权威存储 |
| 连接池 | PgBouncer | **必须** | 应对连接数膨胀，事务级/会话级池化 |
| 向量检索 | pgvector 扩展 | **必须**（TM 语义召回场景） | 复用主库，见技术选型 ADR-30 |
| 缓存/会话/限流/锁 | Valkey | **必须** | 严格禁止做主存储 |
| 消息队列 | Kafka (KRaft) | **必须**（阶段二起） | MVP 阶段可先只上线审计/通知两个 Topic，详见 §18 |
| CDC | Debezium (Kafka Connect) | **必须**（阶段二起） | 配合 Outbox 模式解决双写一致性 |
| Schema 管理 | Confluent Schema Registry (Apache-2.0 社区版/Karapace) | 可暂缓 | Topic 数 < 10 时可用版本化 JSON Schema 文件+CI 校验代替，Topic 增长到跨团队多消费者时再引入 |
| 镜像仓库 | Harbor | **必须** | 局域网离线部署强需求 |
| 指标 | Prometheus + Alertmanager + Grafana | **必须** | |
| 链路追踪 | OpenTelemetry SDK/Collector + Tempo | **必须** | 统一 Trace ID 贯穿同步+异步 |
| 日志 | Loki + Promtail/Vector | **必须** | |
| CI/CD | Argo CD (GitOps) | **必须** | |
| CI 构建 | Gitea Actions / Jenkins（团队已有则复用） | **必须**（择一） | 不新增额外 CI 产品，复用现有 |
| 客户端 | Tauri 2.x + Rust + Svelte 5 | **必须** | 详见技术选型 ADR-14 |
| Web 控制台 | Next.js 14+ (App Router) | **必须** | |
| ASR | faster-whisper | **必须**（媒体扩展范围内） | |
| OCR | PaddleOCR（主）+ Tesseract（备） | **必须** | |
| 音视频处理 | ffmpeg | **必须** | |
| Office 转换 | LibreOffice Headless + python-docx/openpyxl/python-pptx/odfpy | **必须** | |
| TTS/配音 | Coqui TTS / Piper | 可暂缓 | MVP 只留接口钩子（render-writer-service 预留 `dub` 输出类型），不实现完整配音管线 |
| 服务网格（mTLS 全链路） | Linkerd / Istio | 可暂缓 | MVP/阶段二用 K8s NetworkPolicy + 应用层 TLS/JWT 即可满足内部鉴权需求；仅当需要细粒度流量镜像/金丝雀在 sidecar 层统一管理时才评估引入，属于当前规模的**潜在过度设计**，见 §14 |
| Saga 编排框架（如 Temporal/Camunda） | — | **属于过度设计** | 当前业务流程（媒体翻译管道）用 Kafka 事件链 + 补偿事件即可覆盖，不需要引入独立 Saga 编排引擎，详见 §8 |
| 独立向量数据库（Qdrant/Milvus） | — | **属于过度设计** | pgvector 已覆盖当前量级需求，见技术选型 ADR-30 |
| ZooKeeper | — | **属于过度设计** | Kafka KRaft 模式已替代 |
| 服务网格全量 mTLS + 独立 CA 体系（SPIFFE/SPIRE） | — | **属于过度设计**（当前阶段） | K8s Secret + cert-manager 内部 CA 已满足局域网可信边界内的证书轮换需求，SPIFFE/SPIRE 属于跨信任域零信任场景，当前单一局域网集群不构成该场景 |
| 多活/跨机房容灾 | — | **属于过度设计**（当前阶段） | 局域网单集群部署，跨机房多活不在 §16 HA/DR 范围内，见 §16 说明 |

---

## 4. 微服务划分建议（5–10 个核心服务起步 + 媒体扩展服务）

> 遵循"5–10 个核心服务"的告诫，MVP 阶段核心业务服务控制在 **8 个**（auth/user 合并计、project、task、file、notification、report、audit、worker），媒体处理相关服务作为 task-service 的"处理器插件"在阶段二逐个上线，而非一次性全部拆出。

### 4.1 核心服务一览

| 服务 | 数据边界（逻辑库/Schema） | 职责 | 主要 API 面 | 阶段 |
|---|---|---|---|---|
| **auth-service** | `auth_db` | 用户认证、OIDC/Session、Token 签发、RBAC 角色定义 | `POST /v1/auth/login`、`POST /v1/auth/refresh`、gRPC `AuthCheck` | MVP |
| **user-service** | `user_db` | 用户/组织/租户资料、成员邀请、订阅套餐 | `GET/PUT /v1/users/{id}`、`GET /v1/orgs/{id}/members` | MVP |
| **project-service** | `project_db` | 项目、语言对、领域、术语库/TM 元数据、敏感策略配置 | `POST /v1/projects`、`GET /v1/projects/{id}/glossary` | MVP |
| **task-service** | `task_db` | 翻译任务生命周期（创建/排队/处理中/完成/失败）、任务与媒体类型的路由分发 | `POST /v1/tasks`（含 media_type）、`GET /v1/tasks/{id}`（SSE 进度） | MVP |
| **file-service** | `file_db` + 对象存储（MinIO 或共享 NFS，视规模而定） | 原始文件/中间产物/最终产物的存取、预签名 URL、版本管理 | `POST /v1/files`、`GET /v1/files/{id}/download` | MVP |
| **notification-service** | `notification_db` | 站内信/WebSocket 推送/邮件通知，任务完成/失败提醒 | WebSocket `/v1/ws/notify`、`POST /v1/notify/test` | MVP |
| **report-service** | `report_db` | 用量统计、计费明细、质量报表（QA 命中率等） | `GET /v1/reports/usage` | MVP |
| **audit-service** | `audit_db` | 关键操作审计日志（登录、术语变更、任务导出等） | `GET /v1/audit-logs`（内部管理） | MVP |
| **worker-service** | 无独立库（消费 task_db 状态机） | 通用异步任务执行框架/调度（Cron 型任务、批量导入清洗） | 内部消费者，无对外 API | MVP |
| **translation-core** | 复用 `project_db` 内 TM/术语相关表 | 编排翻译管道：TM 匹配→术语匹配注入→标签保护→模型翻译→术语校验→QA（沿用 OFCAT LangGraph 编排逻辑，迁移为常驻服务） | gRPC `Translate(stream)`、`TMMatch`、`QACheck` | MVP |
| **ingestion-service** | `task_db`（写入 task 归属的 source 记录） | 统一接收各媒体类型原始文件，识别媒体类型并拆解为处理任务（视频→抽音轨+抽字幕轨探测，PDF→判定文字层/扫描件，Office→判定格式版本） | `POST /v1/ingest`（multipart，返回 task_id） | 阶段二 |
| **asr-service** | 无独立库（结果写回 file-service + task 事件） | 语音转写（faster-whisper），输出带时间戳的转写文本，供字幕/翻译使用 | 内部消费者：消费 `task.media.asr.requested`，产出 `task.media.asr.completed` | 阶段二 |
| **ocr-service** | 无独立库 | 图片/PDF 扫描件/视频帧/GIF-WebP 帧的文字检测识别（PaddleOCR） | 消费 `task.media.ocr.requested`，产出 `task.media.ocr.completed` | 阶段二 |
| **subtitle-service** | 无独立库 | 字幕格式解析生成（srt/vtt/ass）、时间轴对齐、字幕分段与翻译单元切分 | 消费 ASR/翻译结果事件，产出字幕文件 | 阶段二 |
| **office-converter-service** | 无独立库 | Office 全家桶（docx/xlsx/pptx + ODF）结构化文本抽取与译文回填、老格式转换（LibreOffice Headless） | 消费 `task.media.office.requested` | 阶段二 |
| **render-writer-service** | 无独立库 | 通用"渲染回写"服务：把 translation-core 产出的译文，结合原始版面/时间轴信息，写回为原格式最终产物（字幕烧录、PDF 版面重排、Office 回填、GIF/WebP 重编码、预留 TTS 配音钩子） | 消费 `task.media.render.requested`，产出最终文件写入 file-service | 阶段二 |

> asr-service / ocr-service / subtitle-service / office-converter-service / render-writer-service 统一约定：**无独立数据库**，仅作为无状态处理器消费 Kafka 事件、调用 file-service 存取文件、通过 task-service 的状态机 API 上报进度，避免"每个处理器都建一个库"的碎片化，符合"新增组件先问是否需要独立存储"的判定标准（当前判定：处理结果需要与任务状态强一致展示，直接写回 task_db 通过 task-service API 即可，不需要独立库）。

### 4.2 服务间集成方式总览

| 集成场景 | 方式 | 理由 |
|---|---|---|
| 客户端/控制台 → 核心服务 | 同步 REST（+ 关键流式用 SSE/WebSocket） | 用户可感知的即时交互，需要同步返回或流式反馈 |
| task-service → 媒体处理服务（asr/ocr/subtitle/office/render） | 异步 Kafka 事件 | 处理耗时不确定（秒级到分钟级），需要削峰、失败重试、多消费者扩展 |
| project-service ↔ task-service（如术语库变更通知） | Outbox + Kafka 事件（`project.events`） | 跨服务数据最终一致，非强实时 |
| auth-service 鉴权校验 | 同步 gRPC（内部 `AuthCheck`），并在 Valkey 缓存 Token 校验结果（短 TTL） | 高频调用，需要低延迟，缓存降低 auth-service 压力 |
| 各服务 → audit-service | 异步 Kafka（`audit.events`），audit-service 专职消费落库 | 审计不应阻塞主业务流程，且需要多服务共享统一审计通道 |
| 各服务 → notification-service | 异步 Kafka（`notification.events`） | 通知是"最终触达"场景，允许短暂延迟，不应阻塞业务事务 |
| translation-core ↔ project-service（术语库/TM 读取） | 同步 gRPC（读多写少，且需要请求内即时返回） | 翻译管道内部需要同步等待 TM/术语匹配结果 |

---

## 5. 数据库划分建议

### 5.1 逻辑库划分

| 逻辑库 | 归属服务 | 主要表（示例） | 账号 |
|---|---|---|---|
| `auth_db` | auth-service | `users_credential`, `sessions`, `roles`, `role_bindings` | `svc_auth`（仅 auth_db 权限） |
| `user_db` | user-service | `users_profile`, `orgs`, `org_members`, `subscriptions` | `svc_user` |
| `project_db` | project-service, translation-core | `projects`, `terms`, `translation_memory`, `tm_vectors`(pgvector), `glossary_versions` | `svc_project` |
| `task_db` | task-service, ingestion/asr/ocr/subtitle/office/render-service（经 task-service API，不直连） | `tasks`, `task_media_items`, `task_events_outbox` | `svc_task` |
| `file_db` | file-service | `files`, `file_versions` | `svc_file` |
| `notification_db` | notification-service | `notifications`, `notification_prefs` | `svc_notify` |
| `report_db` | report-service | `usage_daily`, `billing_items`, `qa_stats` | `svc_report` |
| `audit_db` | audit-service | `audit_logs` | `svc_audit` |

### 5.2 账号隔离原则

- 每个逻辑库对应一个独立 PostgreSQL 角色，仅授予该库的 `CONNECT`/`USAGE`/表级 CRUD 权限，禁止跨库授权。
- 迁移（DDL）使用独立的 `migrator` 角色，运行时应用角色不具备 DDL 权限（防止应用层 Bug 误执行破坏性 DDL）。
- 所有账号密码存于 K8s `Secret`（阶段二起接入外部 Secret 管理如 Vault，见 §14），通过 CI/CD 流水线注入，不硬编码在镜像/配置文件中。

### 5.3 连接池

- **PgBouncer**，每个逻辑库一个 PgBouncer 实例（或按命名空间分组的少数几个多库实例），事务级（transaction）池化模式为默认，避免长连接耗尽 PostgreSQL `max_connections`。
- 估算：3000 并发用户，假设峰值 10% 同时发起请求（300 并发请求），每请求数据库连接占用 < 50ms，PgBouncer 后端到 PostgreSQL 的实际连接数控制在 **每逻辑库 20–50 个后端连接**即可覆盖，应用层到 PgBouncer 前端连接可放宽到数百。

### 5.4 索引与慢查询监控

- 核心高频查询建索引：`tasks(project_id, status, created_at)`、`translation_memory(project_id, source_lang, target_lang, source_hash)`（精确匹配）、`tm_vectors` 上建 HNSW 索引（pgvector `USING hnsw (embedding vector_cosine_ops)`）。
- 启用 `pg_stat_statements` 扩展，Prometheus 通过 `postgres_exporter` 采集慢查询/锁等待指标，Grafana 面板阈值告警（P95 查询耗时 > 200ms 告警）。

### 5.5 备份与 PITR

- CloudNativePG 内置 WAL 归档到对象存储（MinIO 自建或内网 S3 兼容存储），支持按时间点恢复（PITR）。
- 全量备份：每日一次；WAL 连续归档：实时。
- 恢复演练：每季度至少一次全库恢复演练，记录 RTO/RPO 实测值。
- 保留策略：全量备份保留 30 天，WAL 归档保留 7 天滚动（满足误操作 7 天内可任意时间点回滚）。

### 5.6 HA 方案选型对比与结论

| 方案 | 优点 | 缺点 | 结论 |
|---|---|---|---|
| CloudNativePG (CNPG) | 原生 K8s Operator，声明式管理主备/切换/备份，CNCF Sandbox 项目活跃 | 相对年轻，复杂拓扑（如级联复制）文档不如 Patroni 丰富 | **采用**（阶段一即用，降低运维门槛） |
| Patroni + etcd/Consul | 社区验证时间最长，功能最全 | 需要额外维护 DCS（etcd/Consul）组件，K8s 环境下与 CNPG 相比无额外优势 | 备选，仅当 CNPG 遇到未覆盖场景时切换，SQL 层零锁定 |
| Stolon | 轻量 | 社区活跃度低于前两者 | 不采用 |

结论：**阶段一/二采用 CloudNativePG，1 主 2 备（同步流复制 1 备 + 异步流复制 1 备），阶段三视故障演练结果决定是否扩展为跨可用区（跨机架）拓扑**。

---

## 6. Kafka 设计

### 6.1 是否需要 Kafka（六问判定，见 §1.2）

- Q1 解决什么问题：媒体处理任务（ASR/OCR/字幕/Office 转换/渲染回写）耗时不确定、需要多阶段流水线传递、需要失败重试与死信隔离、审计与通知需要多服务广播消费。
- Q2 当前规模是否需要：MVP 阶段（50–200 并发）可暂不引入，用 task-service 内的数据库队列表（`task_media_items` + 定时轮询）替代；一旦进入阶段二（500+ 并发，媒体处理服务拆分为独立微服务）则确有需要。
- Q3 不引入会怎样：MVP 阶段可接受（轮询延迟数百毫秒到几秒可接受）；阶段二若不引入，媒体处理服务与 task-service 将退化为紧耦合同步调用，无法独立扩缩容、无法做失败重试与削峰。
- Q4 更简单方案：PostgreSQL `LISTEN/NOTIFY` + 状态轮询可覆盖 MVP 阶段单机/小规模场景，作为 MVP 阶段的过渡方案（见 §18 阶段一）。
- Q5/Q6：Kafka KRaft 模式 Apache-2.0 开源免费，可离线部署于内网。
- **结论：阶段一不引入 Kafka（用 DB 队列表替代），阶段二起正式引入。**

### 6.2 Topic 命名规范

`<domain>.<entity>.<event>`，全小写、点分隔。

| Topic | 用途 | Partition Key | Partitions | Replication Factor | Retention |
|---|---|---|---|---|---|
| `user.events` | 用户/组织资料变更 | `org_id` | 6 | 3 | 7 天 |
| `project.events` | 项目/术语库/TM 元数据变更 | `project_id` | 6 | 3 | 7 天 |
| `task.events` | 任务生命周期状态变更（创建/排队/完成/失败） | `task_id` | 12 | 3 | 14 天 |
| `task.media.asr.requested` / `.completed` | ASR 请求/完成 | `task_id` | 6 | 3 | 3 天 |
| `task.media.ocr.requested` / `.completed` | OCR 请求/完成 | `task_id` | 6 | 3 | 3 天 |
| `task.media.subtitle.requested` / `.completed` | 字幕处理请求/完成 | `task_id` | 6 | 3 | 3 天 |
| `task.media.office.requested` / `.completed` | Office 转换请求/完成 | `task_id` | 6 | 3 | 3 天 |
| `task.media.render.requested` / `.completed` | 渲染回写请求/完成 | `task_id` | 6 | 3 | 3 天 |
| `audit.events` | 全平台审计事件 | `org_id` | 6 | 3 | 90 天（合规留痕） |
| `notification.events` | 通知触达事件 | `user_id` | 6 | 3 | 3 天 |
| `<topic>.retry.<N>` | 重试主题（N=1,2,3，指数退避） | 同源 Topic | 同源 | 3 | 同源 |
| `<topic>.dlq` | 死信队列 | 同源 Topic | 同源 | 3 | 30 天（人工排查窗口） |

### 6.3 分区键设计原则

以业务实体 ID（`task_id`/`project_id`/`org_id`/`user_id`）作为 Partition Key，保证同一实体的事件严格有序（单分区内有序），跨实体天然并行。避免使用随机/轮询 Key（会破坏同实体事件顺序）。

### 6.4 Retry Topic 与 DLQ

- 消费失败 → 重试 1（延迟 10s）→ 重试 2（延迟 1min）→ 重试 3（延迟 10min）→ 仍失败进入 `.dlq`，由 audit-service/运维工具订阅 DLQ 并触发告警，人工介入或补偿脚本重放。
- DLQ 消息保留 30 天，配套一个内部管理页面（Web 控制台）可查看/重放 DLQ 消息。

### 6.5 Consumer Group 约定

- 每个微服务实例池共用一个 Consumer Group（如 `asr-service-group`），保证同一 Topic 分区只被组内一个实例消费（避免重复处理），组内实例数 ≤ Topic 分区数。
- 跨服务需要"广播消费同一事件"的场景（如 `task.events` 既被 notification-service 消费又被 report-service 消费），使用不同 Consumer Group 名各自独立消费全量事件。

### 6.6 Schema 管理

- MVP/阶段二：Kafka 消息体用 JSON + 版本字段（`schema_version`），Schema 定义文件纳入 Git 仓库并在 CI 中做兼容性校验（新增字段向后兼容，禁止破坏性删除/改类型不加版本号）。
- 阶段三（Topic 数量与跨团队消费者显著增长后）：视需要引入 Schema Registry（Karapace，Apache-2.0 开源实现，避免 Confluent 商业许可争议）+ Avro/Protobuf，当前判定为"可暂缓"，见 §3。

### 6.7 幂等消费策略

- 每条事件消息携带全局唯一 `event_id`（Outbox 表主键或 UUID）。
- 消费者侧维护"已处理 event_id"去重表（可用 Valkey `SETNX` 短期去重 + 消费者本地业务表的唯一约束兜底，双重防线）。
- 业务写入使用"upsert + 版本号/时间戳比较"模式，保证同一事件重复消费不产生副作用（幂等）。

---

## 7. Outbox + Debezium CDC 设计

### 7.1 问题背景

若业务代码在同一次请求处理中先 `INSERT INTO tasks(...)`，再显式调用 Kafka Producer 发消息，两个操作不在同一事务内，存在四种故障组合：DB 成功+Kafka 失败（消息丢失）、DB 失败+Kafka 成功（幻影事件）、双写乱序、重复投递。Outbox + CDC 模式将"业务写"与"事件写"合并为**同一个本地数据库事务**，再由 CDC 异步、可靠地把 Outbox 表的变更转发到 Kafka，从根本上消除双写不一致。

### 7.2 Outbox 表结构示例（以 `task_db.task_events_outbox` 为例）

```sql
CREATE TABLE task_events_outbox (
    id              BIGSERIAL PRIMARY KEY,
    event_id        UUID NOT NULL DEFAULT gen_random_uuid(),
    aggregate_type  TEXT NOT NULL,        -- 'task'
    aggregate_id    TEXT NOT NULL,        -- task_id，作为后续 Kafka Partition Key
    event_type      TEXT NOT NULL,        -- 'task.created' / 'task.completed' 等
    payload         JSONB NOT NULL,       -- 事件体
    schema_version  INT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);
-- Debezium 用逻辑复制槽捕获本表 INSERT，转发后可选异步清理（定时任务删除 7 天前记录，避免表膨胀）
CREATE INDEX idx_outbox_created_at ON task_events_outbox (created_at);
```

### 7.3 事务流程

```
BEGIN;
  INSERT INTO tasks(...) VALUES (...);                       -- 业务写
  INSERT INTO task_events_outbox(aggregate_type, aggregate_id,
      event_type, payload) VALUES ('task', :task_id,
      'task.created', :payload_json);                        -- 事件写（同事务）
COMMIT;
-- 事务提交后，Debezium 通过 PostgreSQL 逻辑复制槽异步捕获 outbox 表的 INSERT，
-- 转换为 Kafka 消息发布到对应 Topic（按 event_type 路由 Topic，aggregate_id 作为 Partition Key）
```

### 7.4 CDC Connector 配置要点

- PostgreSQL 侧：`wal_level = logical`，为每个需要 CDC 的逻辑库创建独立的**逻辑复制槽**与 **Publication**（`CREATE PUBLICATION task_outbox_pub FOR TABLE task_events_outbox;`），避免复制槽捕获无关表产生不必要负载。
- Debezium Connector 关键配置：
  - `plugin.name=pgoutput`（PostgreSQL 原生逻辑解码插件，无需额外装插件）
  - `table.include.list=public.task_events_outbox`（只订阅 Outbox 表，不订阅业务表本身，隔离关注点）
  - `transforms=outbox`，启用 Debezium 官方 **Outbox Event Router** SMT（Single Message Transform），自动把 `aggregate_id` 映射为 Kafka Key、`event_type` 映射为目标 Topic 后缀。
  - `heartbeat.interval.ms` 设置心跳，避免长期无变更导致复制槽 LSN 不推进。
- **监控**：`pg_replication_slots` 视图暴露 `confirmed_flush_lsn` 与当前 WAL LSN 的差值，Prometheus `postgres_exporter` 采集复制槽积压字节数，超过阈值（如 500MB）触发告警（对应技术选型风险 R-08）。

---

## 8. 数据一致性设计

| 场景 | 一致性方式 | 理由 |
|---|---|---|
| 用户登录/鉴权校验 | API 同步（gRPC）+ Valkey 短 TTL 缓存 | 用户强感知，需要立即返回结果 |
| 创建项目/任务、查询任务状态 | API 同步（REST） | 用户提交后期望立即得到"已创建/排队中"的确认 |
| 任务状态流转（排队→处理中→各阶段完成→整体完成） | Outbox + Kafka 异步 | 多阶段、多服务参与，允许最终一致，且需要多消费者（通知/报表/审计）各自独立消费 |
| 术语库/TM 变更同步给 translation-core 内部缓存 | Kafka 异步（`project.events`） | 变更频率低、translation-core 可容忍秒级延迟感知最新术语，通过消费事件失效本地缓存 |
| 审计日志落库 | Kafka 异步（`audit.events`），audit-service 单独消费落库 | 审计不应阻塞主业务事务；即使 audit-service 短暂不可用，事件可堆积在 Kafka 待恢复后补齐 |
| 通知触达（站内信/WebSocket/邮件） | Kafka 异步（`notification.events`） | 允许最终触达，不阻塞业务主流程 |
| 跨库事务（如 project_db 增术语 且 task_db 需要重新触发已挂起任务重译） | Kafka 异步事件 + 幂等消费（非 Saga） | 属于"补偿型最终一致"场景，事件驱动即可覆盖，不涉及需要显式回滚的分布式事务语义 |

### 8.1 是否引入 Saga：结论与理由

**结论：当前阶段不引入独立 Saga 编排框架（如 Temporal/Camunda），仅用"事件链 + 幂等消费 + 定时对账补偿任务"覆盖跨服务一致性需求。**

- Q1 解决什么问题：Saga 主要解决"多步骤分布式事务需要显式补偿回滚"的场景（如电商下单扣库存扣余额三步任一失败需要逆向补偿）。
- Q2 当前规模/场景是否需要：CATs 的跨服务流程（媒体翻译管道）本质是**单向流水线** — 任务创建→分发处理→逐阶段推进→完成/失败标记，各阶段失败只需把任务状态标记为失败并通知用户，**不需要**"撤销已完成的上游阶段"这种真正的补偿语义（例如 ASR 已完成，OCR 失败，不需要撤销 ASR 结果，只需任务整体标记部分失败，允许用户重试 OCR 阶段）。
- Q3 不引入会怎样：用简单状态机（task-service 维护 `task_media_items` 各阶段状态字段）+ 事件驱动推进即可表达；引入 Saga 框架会显著增加系统复杂度（新增一套编排引擎、状态存储、DSL 学习成本）而收益有限。
- Q4 更简单方案：task-service 状态机 + 定时对账任务（扫描超时未推进的任务、发起重试或标记失败）。
- **结论符合 §1.2 判定标准，暂不引入，留作阶段三如出现真正需要跨服务补偿回滚的新业务场景（如计费/退款类事务）时再评估引入范围受限的 Saga 编排（优先考虑基于 Kafka 事件的编排式 Saga，而非引入额外中间件）。**

---

## 9. Envoy Gateway 流量设计

### 9.1 资源示例

```yaml
# Gateway：集群入口，监听 443
apiVersion: gateway.networking.k8s.io/v1
kind: Gateway
metadata:
  name: cats-gw
  namespace: cats-platform
spec:
  gatewayClassName: envoy-gateway
  listeners:
    - name: https
      protocol: HTTPS
      port: 443
      tls:
        mode: Terminate
        certificateRefs:
          - name: cats-internal-tls
    - name: grpc
      protocol: HTTPS
      port: 8443
      tls:
        mode: Terminate
        certificateRefs:
          - name: cats-internal-tls
---
# HTTPRoute：REST API 路由到 task-service
apiVersion: gateway.networking.k8s.io/v1
kind: HTTPRoute
metadata:
  name: task-service-route
  namespace: cats-core
spec:
  parentRefs:
    - name: cats-gw
      namespace: cats-platform
  hostnames: ["api.cats.internal"]
  rules:
    - matches:
        - path: {type: PathPrefix, value: /v1/tasks}
      backendRefs:
        - name: task-service
          port: 8080
---
# GRPCRoute：translation-core 内部 gRPC（示例，供网关侧灰度/可观测统一管理）
apiVersion: gateway.networking.k8s.io/v1
kind: GRPCRoute
metadata:
  name: translation-core-route
  namespace: cats-core
spec:
  parentRefs:
    - name: cats-gw
      namespace: cats-platform
  hostnames: ["grpc.cats.internal"]
  rules:
    - matches:
        - method: {service: cats.translation.v1.TranslationCore}
      backendRefs:
        - name: translation-core
          port: 9090
```

### 9.2 灰度发布（权重路由）

```yaml
rules:
  - matches: [{path: {type: PathPrefix, value: /v1/tasks}}]
    backendRefs:
      - name: task-service-stable
        port: 8080
        weight: 90
      - name: task-service-canary
        port: 8080
        weight: 10
```
新版本先以 10% 流量灰度观察 Prometheus 错误率/延迟指标，稳定后逐步调整权重至 100%，再下线旧版本 Deployment。

### 9.3 限流与熔断（`BackendTrafficPolicy`，Envoy Gateway 扩展 API）

- 限流：按客户端 IP/Token 维度，`task-service` 上传接口限速 20 req/s/用户（防止批量脚本滥用）。
- 熔断：连续 5 次 5xx 触发短路（outlier detection），30 秒后半开探测恢复。
- 重试：幂等 GET 请求允许自动重试 2 次（指数退避），POST/PUT 类写请求默认不自动重试（避免重复提交，由客户端业务层控制幂等键）。

### 9.4 是否需要 MetalLB / HAProxy / Keepalived

- **MetalLB：必须**。局域网裸金属 K3s 集群没有云厂商 LoadBalancer，MetalLB 以 L2（ARP 广播）或 BGP 模式为 Envoy Gateway 的 Service 分配可路由的局域网 IP，是暴露入口的最简方案。
- **HAProxy + Keepalived（VIP 漂移）：可暂缓**。MVP/阶段二 MetalLB L2 模式的主备切换（约几秒 ARP 收敛）已可接受；仅当未来需要跨网段/更严格亚秒级切换 SLA 时，才评估在 MetalLB 前再加一层 HAProxy+Keepalived VIP，当前属于**潜在过度设计**。

---

## 10. K3s 节点规划

### 10.1 控制面 HA

| 项 | 规划 |
|---|---|
| 控制面节点数 | 3（奇数保证 etcd Raft 多数派） |
| etcd | K3s 内嵌 etcd，`--cluster-init` 启动 HA 模式，禁用默认 SQLite datastore |
| 单节点规格建议 | 4 vCPU / 8GB RAM / 100GB SSD（etcd 对磁盘 IO 延迟敏感，要求 SSD） |
| 部署约束 | 3 个控制面节点分布在不同物理机/不同机架 PDU，避免单一断电点同时打掉多数派 |

### 10.2 工作节点规划

| 节点池（Label: `pool=`） | 数量（MVP 起） | 单节点规格建议 | 承载 | Taint |
|---|---|---|---|---|
| `general` | 3 起 | 8 vCPU / 16GB RAM | auth/user/project/task/file/notification/report/audit/worker-service, Envoy Gateway | 无 |
| `media-cpu` | 2 起 | 16 vCPU / 32GB RAM | ffmpeg 转码、LibreOffice Headless、PaddleOCR CPU 推理、subtitle/office-converter/render-writer-service | `workload=media-cpu:NoSchedule`（对应 toleration） |
| `media-gpu` | 1 起（按 ASR/OCR 并发量弹性增加） | 16 vCPU / 64GB RAM / 1× GPU（显存 ≥16GB，如 RTX 4090/A4000 级别） | faster-whisper GPU 推理、PaddleOCR GPU 推理 | `workload=gpu:NoSchedule` |
| `data` | 3 起（PostgreSQL 用 3，Kafka 另起 3，视预算可复用同池不同节点） | 16 vCPU / 32GB RAM / NVMe SSD 独立盘 | PostgreSQL(CNPG)、Kafka broker、Valkey | `workload=data:NoSchedule` |

### 10.3 亲和性 / PDB / HPA

- **Affinity**：有状态组件（PostgreSQL/Kafka）使用 Pod 反亲和（`podAntiAffinity`）确保多副本分散到不同物理节点；媒体处理服务优先调度到对应 `media-cpu`/`media-gpu` 节点池（`nodeAffinity` + `toleration`）。
- **PodDisruptionBudget**：核心服务（auth/task/translation-core）设置 `minAvailable: 1`（或按副本数的 60% 取整），保证节点维护/滚动升级期间不会同时全部下线。
- **HPA**：`task-service`/`translation-core`/媒体处理服务基于 CPU 利用率（阈值 70%）+ 自定义指标（Kafka Consumer Lag，通过 KEDA 或 Prometheus Adapter 暴露）双维度自动扩缩容；数据层（PostgreSQL/Kafka）不做 HPA，采用手工规划的容量评估（见 §17）。

### 10.4 滚动升级与故障恢复

- K3s 版本升级：先升级控制面（逐台，保持多数派可用），再滚动升级工作节点（每次 cordon+drain 一台，等业务 Pod 在其他节点重新调度稳定后再继续下一台）。
- 应用滚动升级：Deployment `maxUnavailable: 0, maxSurge: 1`（先起新版本再下线旧版本，保证升级期间不丢容量）。
- 节点故障恢复：K3s 默认 `node-monitor-grace-period`（约 40s）后标记 NotReady，Pod 驱逐重新调度到健康节点；有状态组件（PostgreSQL/Kafka）依赖各自的副本/ISR 机制自动切主，无需人工介入（但需配置告警通知运维确认根因）。

---

## 11. Valkey 使用规范

### 11.1 允许用途

| 用途 | 示例 | Key 规范 | TTL |
|---|---|---|---|
| 会话缓存 | 登录 Session/Token 校验结果 | `sess:{token_hash}` | 与 Token 有效期一致，默认 2h |
| 限流计数 | Envoy Gateway/服务自身滑动窗口限流 | `rl:{scope}:{key}:{window}` | 与限流窗口一致（如 1s/1min） |
| 分布式锁 | 防止同一任务被多个 worker 并发处理（配合 Kafka Consumer Group 已有的独占分区语义，作为二次防线） | `lock:task:{task_id}` | 锁持有时长上限（如 5min），必须设置 TTL 防死锁 |
| 热数据缓存 | 术语库/TM 高频查询结果缓存（Cache-Aside，PostgreSQL 为准） | `cache:tm:{project_id}:{hash}` | 5–15min，变更时主动失效（消费 `project.events`） |
| 幂等去重 | Kafka 消费者幂等去重（配合业务表唯一约束兜底） | `dedup:{event_id}` | 24h |

### 11.2 明确禁止

- **禁止**把 Valkey 作为任何业务实体的唯一/权威数据源（如直接把术语库/任务状态只存 Valkey 不落 PostgreSQL）。
- **禁止**无 TTL 的永久 Key（除极少数配置类 Key 需经架构评审白名单登记）。
- **禁止**把 Valkey 当作消息队列的替代品做可靠消息投递（Valkey Pub/Sub 不持久化，与 Kafka 职责不可混用）。
- 所有写入 Valkey 的数据必须能够从 PostgreSQL 重建（缓存失效/清空不应造成数据丢失，最多造成一次性缓存穿透）。

### 11.3 部署形态

- 阶段一/二：Valkey 单实例 + 副本（主从），用于缓存类场景可接受短暂不可用（重新从 PostgreSQL 回源）。
- 阶段三：如限流/分布式锁场景对可用性要求提高，可升级为 Valkey Cluster 模式（分片+哨兵/集群自动故障转移）。

---

## 12. Harbor 镜像架构

### 12.1 项目/命名划分

| Harbor Project | 用途 | 镜像示例 |
|---|---|---|
| `cats-core` | 自研核心业务微服务镜像 | `harbor.internal/cats-core/task-service:1.4.2` |
| `cats-media` | 自研媒体处理微服务镜像 | `harbor.internal/cats-media/asr-service:0.9.0` |
| `cats-frontend` | Web 控制台/客户端构建产物容器（如 Next.js 镜像） | `harbor.internal/cats-frontend/web-console:2.1.0` |
| `cats-3rdparty` | 第三方基础镜像代理缓存（PostgreSQL/Kafka/Envoy/Prometheus 等） | `harbor.internal/cats-3rdparty/postgres:16.4` |

### 12.2 版本策略

- 自研镜像 Tag 规范：`{semver}`（如 `1.4.2`）为发布版本，`{semver}-{git_short_sha}` 为 CI 每次构建的可追溯版本，`latest`/`dev` 仅用于开发环境，**禁止在 prod 环境使用 `latest`**（Argo CD Application 强制指定精确 Tag，配合 CI 自动更新 Git 中的 Helm values 版本号完成"GitOps 触发部署"）。
- 第三方镜像：通过 Harbor 的"仓库代理（Proxy Cache）"或定时同步策略从公网拉取后缓存，集群内统一从 `cats-3rdparty` 拉取，避免直连境外镜像源不稳定。

### 12.3 扫描与回滚

- 集成 Trivy 扫描（Harbor 内置），镜像推送后自动扫描，发现 Critical/High 漏洞的镜像标记但不阻断推送（阻断策略视组织安全要求逐步收紧），生产部署前 CI 门禁校验镜像扫描结果。
- 回滚：Argo CD 保留每次同步的 Git commit 历史，回滚即 `git revert` 到上一个稳定 commit（Helm values 指向上一版本镜像 Tag），Argo CD 自动同步完成回滚，分钟级完成。

### 12.4 离线部署

- 提供"镜像打包脚本"：`harbor-offline-bundle.sh` 将一批指定 Tag 的镜像导出为 `.tar` 包，供无公网访问的局域网环境通过物理介质导入 Harbor（`docker save`/`docker load` 或 Harbor CLI 批量导入）。
- Harbor 自身以 HA 模式部署（≥2 副本 + 外置对象存储做镜像 Blob 后端），避免 Harbor 单点故障阻断全部部署（对应技术选型风险 R-11）。

---

## 13. 可观测性

### 13.1 Prometheus 指标清单（核心）

| 指标类别 | 示例指标 | 用途 |
|---|---|---|
| HTTP/gRPC | `http_requests_total{service,route,status}`、`http_request_duration_seconds_bucket` | 请求量/错误率/延迟分位 |
| Kafka Consumer | `kafka_consumergroup_lag{group,topic,partition}` | 消费积压，驱动 HPA 与告警 |
| PostgreSQL | `pg_stat_activity_count`、`pg_replication_slot_lag_bytes`（自定义采集）、慢查询计数 | 数据库健康与 CDC 复制槽积压 |
| Valkey | `redis_connected_clients`、`redis_memory_used_bytes`、`redis_evicted_keys_total` | 缓存层容量与驱逐 |
| 媒体处理 | `asr_job_duration_seconds`、`ocr_job_duration_seconds`、`render_job_failure_total` | 媒体管道各阶段耗时/失败率，驱动容量规划（见 §17） |
| K8s | 标准 `kube-state-metrics` + `node_exporter` 全量指标 | 节点/Pod 资源使用与调度健康 |

### 13.2 OpenTelemetry 统一 Trace 设计

- 所有服务（Rust/Python/Node.js/TypeScript）统一接入 OpenTelemetry SDK，导出到 OTel Collector（DaemonSet/Sidecar 均可，MVP 用集中式 Collector Deployment 即可），再转发至 Tempo。
- **同步调用**：HTTP/gRPC 请求头透传 W3C Trace Context（`traceparent`/`tracestate`），网关到各微服务自动传播。
- **异步 Kafka 消息**：生产者在消息 Header 中写入 `traceparent`（当前 Span 上下文），消费者读取 Header 并以此创建"链接 Span”（Span Link，而非父子 Span，因为异步消息处理不属于同一调用栈的父子关系，用 Link 语义更准确），从而在 Tempo 中把"任务创建 → Kafka 事件 → ASR 处理 → OCR 处理 → 渲染回写 → 通知推送"整条异步链路串成一条可查询的 Trace（通过统一 `trace_id`）。
- 每个媒体处理任务的 `task_id` 同时作为业务关联键写入 Span Attribute（`cats.task_id`），便于按业务 ID 反查 Trace，而不仅依赖 `trace_id`。

### 13.3 日志架构

- 应用日志统一 JSON 结构化输出（含 `trace_id`/`task_id`/`service`/`level`），stdout 输出由节点上的 Promtail（或 Vector）采集发送到 Loki。
- Loki 按 `namespace`/`service`/`level` 建标签索引（避免高基数标签如 `task_id` 直接做 Loki Label，改为日志正文字段供 LogQL 全文过滤，防止 Loki 索引膨胀）。
- 保留策略：应用日志 14 天，审计相关日志（登录、术语变更、导出）额外落 `audit_db`（结构化、长期保留，见 §5）。

---

## 14. 安全设计

| 项 | 设计 |
|---|---|
| K8s RBAC | 每个微服务的 ServiceAccount 仅授予自身命名空间内所需的最小权限（如仅能读自身 ConfigMap/Secret），禁止使用 `cluster-admin` 或跨命名空间的宽泛角色 |
| NetworkPolicy | 默认拒绝所有跨命名空间流量（default-deny），显式声明允许的服务间调用（如 `task-service` 允许被 Envoy Gateway 所在命名空间访问，`postgresql` 仅允许 `cats-core` 命名空间内已知服务 IP/标签访问） |
| Secret 管理 | 阶段一/二：K8s 原生 Secret + SealedSecrets（加密后可安全提交 Git，实现 GitOps 下的密钥管理）；阶段三视合规要求评估引入 HashiCorp Vault（开源版 Apache-2.0/MPL） |
| Kafka ACL | 每个消费者/生产者使用独立 SASL 凭据，ACL 限定"仅能生产/消费自己业务域相关 Topic"（如 `asr-service` 只能消费 `task.media.asr.requested`、生产 `task.media.asr.completed`，无权访问 `audit.events`） |
| Harbor RBAC | 按 Project 划分（见 §12.1），CI 账号仅有对应 Project 的 push 权限，只读账号（K3s 集群拉取用）仅有 pull 权限 |
| mTLS | 阶段一/二：服务间调用走 K8s 内部网络 + NetworkPolicy 隔离 + 应用层 JWT/Token 鉴权，暂不引入全量 mTLS 服务网格（见 §3 过度设计判定）；阶段三如合规要求提升，评估引入 Linkerd（轻量、CNCF 毕业，比 Istio 更适合当前规模）做透明 mTLS |
| 最小权限 | 数据库账号（§5.2）、K8s ServiceAccount、Kafka ACL、Harbor RBAC 均遵循"仅授予完成职责所需的最小权限集"，任何权限扩大需经架构评审登记 |
| 客户端安全 | Tauri 应用签名（Windows Authenticode / macOS Notarization），自动更新走签名校验的更新通道，防止篡改分发 |
| 敏感内容合规 | 延续 OFCAT fail-closed 原则：`translation-core` 对接的云端模型/ASR/OCR API，敏感项目强制路由本地部署模型，本地不可达时中止而非降级云端 |

---

## 15. CI/CD 与 GitOps

### 15.1 流水线

```
开发者提交代码（各微服务独立仓库或 Monorepo）
  → CI（Gitea Actions/Jenkins）：单测 → 构建镜像 → 推送 Harbor（打 {semver}-{git_sha} Tag）
  → CI 自动更新「环境仓库」（GitOps 配置仓，含各环境 Helm values）中 dev 环境的镜像 Tag，提交 PR
  → 自动合并到 dev 分支（dev 环境免人工审批）
  → Argo CD（dev Application）检测到 Git 变更，自动同步到 K3s dev 命名空间
  → dev 环境验证通过后，人工发起 PR 将变更提升到 test 分支
  → Argo CD（test Application）自动同步
  → test 环境验证通过后，人工发起 PR 提升到 prod 分支 —— 该 PR **强制要求至少 1 名架构师/负责人 Approve**（人工审批网关）
  → 合并后 Argo CD（prod Application）同步到生产命名空间（可配置为需要在 Argo CD UI 中额外点击 "Manual Sync" 确认，双重保险）
```

### 15.2 环境隔离

- dev/test/prod 使用**独立 K8s 命名空间**（`cats-core-dev`/`cats-core-test`/`cats-core-prod`），共享同一套 K3s 物理集群（阶段一/二，控制成本），阶段三如容量/隔离性要求提高可评估拆分独立集群。
- 各环境独立的 Helm values 文件（副本数、资源配额、外部依赖连接串均不同），存放于 GitOps 配置仓的 `envs/dev|test|prod/` 目录下，Argo CD 按目录路径分别对应各自 Application。

### 15.3 DB Migration 与应用版本协调

- 数据库 Schema 迁移使用向后兼容的**扩展式迁移**（Expand-Contract 模式）：先上线兼容新旧代码的 Schema 变更（如新增可空列），应用滚动升级完成后再上线收尾迁移（如删除废弃列），避免"新代码依赖新 Schema 但旧 Pod 尚在运行"造成的短暂不兼容。
- 迁移脚本作为 Argo CD `PreSync` Hook Job 在应用 Deployment 更新前执行，失败则整个同步失败并保留旧版本运行（Argo CD 自动判定同步失败不切流量）。
- 迁移工具：Flyway 或 Alembic（视各服务实现语言选择，Python 服务用 Alembic，其余可选 Flyway 统一），版本号纳入 Git 版本控制与代码同仓库管理。

### 15.4 回滚

- 应用回滚：`git revert` Helm values 变更 → Argo CD 自动/手动同步 → 分钟级恢复到上一版本运行时。
- 数据库回滚：由于采用 Expand-Contract 模式，绝大多数场景无需回滚 Schema；极端情况下依赖 PITR（§5.5）时间点恢复，RTO 视备份规模数十分钟到数小时不等（生产环境应急预案需提前演练估算实际耗时）。

---

## 16. 高可用与灾备

| 维度 | HA（高可用，应对单点故障） | DR（灾备，应对整体基础设施损毁） |
|---|---|---|
| 目标 | 单节点/单进程/单副本故障时业务不中断或分钟级自愈 | 整个机房/局域网集群不可用时，能否在可接受时间内于另一环境恢复服务 |
| 覆盖范围 | K3s 控制面 3 节点、PostgreSQL 1 主 2 备、Kafka 3 broker RF=3、核心服务多副本+PDB、Harbor 多副本 | 跨机房/异地灾备集群、异地备份存储 |
| 当前阶段结论 | **必须覆盖**（§10、§5.6、§6 已给出具体方案） | **当前阶段不建设独立异地 DR 集群**（局域网私有化 SaaS 场景，业务前提是单一局域网内部署，暂无跨机房诉求；见 §3 过度设计判定） |
| 已具备的灾备基础能力 | — | PostgreSQL WAL 异地归档（对象存储可配置跨机房复制）+ 定期全量备份可搬迁到备用场地手工恢复，作为"冷 DR"最低限度保障，RTO 以小时计 |
| 触发条件（未来是否建设热 DR） | — | 当客户合同/合规要求明确要求异地容灾 SLA 时，作为独立立项评估，不在本阶段默认范围内 |

---

## 17. 容量规划与压测方案

### 17.1 方法论

**容量规划由压测数据（RPS / P95 / P99 / 每请求 CPU 消耗）反推所需副本数，而非凭经验/团队规模拍脑袋决定。**

步骤：
1. 对每个核心服务在标准测试环境（与生产同规格节点）用 k6/Locust 做压测，得到单副本在给定资源配额下的最大稳定 RPS（P99 延迟仍在 SLO 内的临界点）。
2. 结合业务预估峰值 RPS（由用户规模 × 人均操作频率估算）。
3. 副本数 = ceil(预估峰值 RPS / 单副本最大稳定 RPS × 安全系数（建议 1.5，预留突发余量与滚动升级期间容量损失）)。

### 17.2 示例计算（以 task-service 为例）

| 项 | 数值 | 说明 |
|---|---|---|
| 目标并发用户数 | 3000 | 规划上限 |
| 人均高峰期操作频率 | 0.05 req/s（约每 20 秒一次交互，含轮询任务状态） | 经验估算，需实测校准 |
| 预估峰值 RPS | 3000 × 0.05 = **150 RPS** | |
| 压测得单副本（2 vCPU / 4GB 配额）最大稳定 RPS（P99 < 300ms） | 40 RPS | 压测实测值（示例假设） |
| 安全系数 | 1.5 | |
| 所需副本数 | ceil(150 / 40 × 1.5) = ceil(5.625) = **6 副本** | HPA `minReplicas` 可设 3（低谷），`maxReplicas` 设 8（留冗余） |

同样方法应用于 `translation-core`（CPU/内存消耗与模型调用延迟强相关，需单独压测流式翻译场景）、`asr-service`/`ocr-service`（GPU 资源画像需按"每小时可处理的媒体分钟数"这类业务吞吐指标压测，而非单纯 RPS）。

### 17.3 媒体处理服务容量估算要点

- ASR：faster-whisper 在参考 GPU（16GB 显存）上，`medium` 模型处理 1 分钟音频约耗时 5–10 秒（实时率 6–12x），据此估算给定并发媒体任务量所需 GPU 节点数。
- OCR：PaddleOCR 单张图片（含版面分析）CPU 推理约 200–500ms/张，视频抽帧场景需按"抽帧密度 × 视频时长"估算总处理量。
- 该类估算须在正式压测环境用真实媒体样本实测校准后写入运维手册，本文给出的是数量级参考起点，不作为最终 SLA 承诺依据。

### 17.4 数据层容量规划

- PostgreSQL：按 §5.3 连接数估算 + 表数据量增长趋势（TM/术语条目、任务记录）预留至少 1 年增长的存储余量，磁盘使用率超过 70% 触发扩容告警。
- Kafka：按 Topic Retention（§6.2）× 峰值消息速率估算所需磁盘，预留 30% 余量；Broker 数量以 RF=3 为下限，磁盘 I/O 吞吐是否成为瓶颈通过压测验证后决定是否需要增加 Broker 数分摊分区。

---

## 18. 迁移路线图

### 18.1 第一阶段：最小可行架构（MVP）

**目标**：验证核心业务闭环（文本翻译 SaaS 化 + 基础的 PDF/Office 翻译），支撑 50–200 并发用户。

- K3s：1 套集群，控制面 3 节点起步即按 HA 标准建（不留技术债），工作节点 `general` 池 3 台 + `media-cpu` 池 1 台（暂不建 `media-gpu` 池，ASR/OCR 阶段先用 CPU 推理或云端 API 过渡）。
- 服务范围：auth/user/project/task/file/notification/report/audit/worker-service + translation-core，共 10 个服务；媒体处理只先上线 `ocr-service`（PDF 扫描件）与 `office-converter-service`（docx/xlsx/pptx），`ingestion-service` 与 `render-writer-service` 同期上线打通最小闭环；`asr-service`/`subtitle-service`（音视频）与动图处理延后到阶段二。
- 消息机制：**不引入 Kafka**，用 `task_db` 内的状态字段 + PostgreSQL `LISTEN/NOTIFY` 触发轮询/推送，配合 worker-service 简单调度即可（对应 §6.1 判定结论）。
- 数据层：PostgreSQL 单主 1 备（CNPG），Valkey 单实例+1 副本，暂不建 Kafka/Debezium。
- 网关：Envoy Gateway + MetalLB，基础限流即可，灰度发布能力先具备但不高频使用。
- CI/CD：Argo CD 三环境骨架先搭建（哪怕 test/prod 初期只是最小规格），避免后期补建 GitOps 流程的返工成本。
- 可观测性：Prometheus+Grafana+Loki 基础监控必须从第一天具备（不属于可暂缓项，运维盲区风险高于额外部署成本）。

### 18.2 第二阶段：扩容架构

**目标**：支撑 500–1500 并发用户，补全全媒体能力（音视频/字幕/动图），引入 Kafka 事件驱动架构。

- 正式引入 **Kafka（KRaft）+ Debezium CDC + Outbox 模式**，替换阶段一的 DB 轮询机制（业务代码改造：写业务表同时写 Outbox 表，删除原有轮询逻辑）。
- 补齐 `asr-service`/`subtitle-service`，新增 `media-gpu` 节点池（至少 1 台起，视 ASR/OCR 并发量弹性扩容）。
- 动图（GIF/WebP）处理能力在 `render-writer-service`/`ocr-service` 内以复用逻辑方式补齐（无需新增独立服务，见技术选型 ADR-29）。
- PostgreSQL 升级为 1 主 2 备（同步+异步流复制各一），Valkey 视限流/锁场景压力评估是否升级 Cluster 模式。
- HPA 全面启用（基于压测数据校准的副本数区间，见 §17）。
- 引入 SealedSecrets 强化密钥 GitOps 管理。

### 18.3 第三阶段：高可用架构

**目标**：支撑 1500–3000+ 并发用户，补齐容灾与高级可观测性/安全能力。

- 视故障演练结果与合规要求，评估：Schema Registry（Karapace）、Vault 密钥管理、Linkerd mTLS 服务网格、跨机房冷 DR 能力建设，均按 §1.2 六问逐项评审后决定是否引入（不预先默认引入）。
- 容量：按 §17 方法论持续用最新压测数据校准各服务副本数区间与数据层扩容计划。
- 灾备：如业务/合规要求明确需要热 DR，另立项评估（见 §16）。

### 18.4 从当前 OFCAT 架构迁移的具体步骤

| 步骤 | 内容 |
|---|---|
| S1 | 搭建 K3s 集群与 §18.1 MVP 阶段基础设施（Harbor/Prometheus/Argo CD 先行） |
| S2 | 将 OFCAT 本地引擎（FastAPI 单体）内的 TM/术语/QA/编排（LangGraph）逻辑抽取为 `translation-core` 服务，SQLite 表结构迁移为 PostgreSQL `project_db` 对应表（DDL 转换 + 数据导入脚本，`sqlite-vec` 向量数据迁移为 `pgvector`） |
| S3 | 新建 auth/user/project/task/file/notification/report/audit/worker-service 骨架，把 OFCAT 原有「导入管道」（CMP-12）能力迁移为 worker-service 的一类异步任务 |
| S4 | 开发 Tauri 客户端，复用/重写原 Options/Popup UI（Svelte）的交互逻辑为 Tauri WebView 前端，客户端通过 REST/gRPC 对接新微服务而非直连本地 127.0.0.1 引擎 |
| S5 | 开发 Next.js Web 控制台，承接原本无对应能力的"多用户管理/项目管理/账单"等 SaaS 化管理界面 |
| S6 | 按 §18.1 上线 MVP 全部服务，与仍在使用 OFCAT 本地版的用户并行运行一段时间（双轨），验证核心翻译质量/TM 匹配结果与旧版一致 |
| S7 | 提供"本地 SQLite 数据导入 CATs 云端项目"的一次性迁移工具（复用 S2 的迁移脚本封装为用户自助导入向导），完成存量用户数据迁移 |
| S8 | 灰度切流验证稳定后，逐步引导 OFCAT 本地版用户迁移至 CATs 客户端，进入 §18.2 阶段二能力建设 |

---

## 19. 可降低复杂度的地方 / 潜在单点故障 / 当前方案最大的 10 个风险

| # | 风险/复杂度点 | 影响 | 缓解措施 |
|---|---|---|---|
| 1 | MVP 阶段共享单一 K3s 集群承载 dev/test/prod 三环境 | 中——环境间可能相互影响（资源争抢/误操作波及其他环境） | 命名空间级 ResourceQuota 隔离 + NetworkPolicy 隔离 + RBAC 限制跨环境操作权限；阶段三视预算拆分独立集群 |
| 2 | PostgreSQL 是单一权威存储，若集群级故障影响面大 | 高——所有服务依赖同一 PostgreSQL 集群 | 1 主 2 备 HA（§5.6）+ PITR 备份（§5.5）+ 季度恢复演练；避免"看似分布式实则单点"的假象 |
| 3 | Debezium/Kafka Connect 是 CDC 链路单点（若挂起，Outbox 事件无法转发） | 中——Outbox 表数据不会丢失（仍在 PostgreSQL），但事件消费方感知延迟 | Kafka Connect 以多副本+自动故障转移部署；监控复制槽积压告警（R-08）；PostgreSQL 侧数据本身不受影响，可延迟补偿 |
| 4 | Harbor 单点会阻断全部新部署/扩容（镜像拉取失败） | 高——影响故障恢复能力本身 | Harbor HA 部署 + 节点本地镜像缓存（`containerd` 已拉取过的镜像节点故障重启后仍可复用），见 §12.4 |
| 5 | media-gpu 节点池数量少，GPU 资源是媒体处理吞吐瓶颈 | 中——ASR/OCR 高峰期排队 | HPA/KEDA 基于 Kafka Consumer Lag 自动扩容（受限于物理 GPU 数量上限，需提前按 §17.3 估算容量） |
| 6 | LibreOffice Headless 进程稳定性历史上是已知痛点（可能僵死） | 中 | 转换服务加超时熔断+进程池回收（R-09），必要时降级走纯语言库路径（python-docx 等）跳过 LibreOffice |
| 7 | Kafka Topic/Consumer Group 数量随媒体处理服务增多而增长，运维认知负担上升 | 低–中 | 严格遵循命名规范（§6.2）+ 集中式 Topic 清单文档纳入 GitOps 配置仓 code review 流程 |
| 8 | Envoy Gateway/Gateway API 相对新，团队学习曲线 | 低 | 提供内部使用手册+模板化 Helm values（ADR-17 已识别，见技术选型书） |
| 9 | 客户端（Tauri）与 Web 控制台（Next.js）两套前端技术栈并存，重复开发/维护部分 UI 组件 | 低–中 | 共享 TypeScript API 契约包（OpenAPI 生成），核心视觉规范（Design Token）跨技术栈共用，非强制共享组件库 |
| 10 | 当前判定"不需要"的组件（Saga 引擎/服务网格 mTLS/独立向量库/Schema Registry）未来需求变化时的引入成本被低估 | 低–中 | 每次架构评审复审 §1.2/§3 判定结论是否仍成立，避免"当初图省事，后来技术债"，保持决策可追溯（本文档修订履历机制） |

---

## 20. 最终推荐拓扑图（文字化）

```
                        局域网用户（50–3000 并发）
        ┌───────────────────────┬──────────────────────────┐
   Tauis 客户端(Rust+Svelte)              浏览器 → Next.js Web 控制台
        └───────────────────────┴──────────────────────────┘
                              │ HTTPS/gRPC/WebSocket
                     MetalLB (L2, 局域网 VIP)
                              │
                     Envoy Gateway（K3s 边缘）
        ┌─────────────┬───────────────┬────────────────────┐
   HTTPRoute(REST)  GRPCRoute(内部)  灰度/限流/熔断策略层
        └─────────────┴───────────────┴────────────────────┘
                              │
   ┌──────────────────────────────────────────────────────────────┐
   │ K3s 工作节点池                                                   │
   │  general: auth/user/project/task/file/notification/           │
   │           report/audit/worker-service + translation-core       │
   │  media-cpu: ingestion/subtitle/office-converter/               │
   │             render-writer-service + PaddleOCR(CPU)/LibreOffice  │
   │  media-gpu: asr-service(faster-whisper) + PaddleOCR(GPU)        │
   │  data: PostgreSQL(CNPG 1主2备) / Kafka(KRaft 3broker) / Valkey  │
   └──────────────────────────────────────────────────────────────┘
                              │
   ┌──────────────────────────────────────────────────────────────┐
   │ 平台支撑（cats-platform 命名空间）                                 │
   │  Harbor(HA) │ Prometheus/Alertmanager/Grafana │ OTel+Tempo       │
   │  Loki+Promtail │ Argo CD(GitOps: dev→test→prod) │ Kafka Connect  │
   │  +Debezium                                                       │
   └──────────────────────────────────────────────────────────────┘
                              │
                K3s 控制面 × 3（HA，内嵌 etcd，跨机架部署）
```

数据流闭环：**用户请求 → Envoy Gateway → 核心业务服务（同步）→ PostgreSQL（唯一权威）→ Outbox → Debezium → Kafka → 媒体处理/通知/审计/报表（异步消费）→ 结果回写 PostgreSQL/file-service → 通知用户**，全链路由 OpenTelemetry 统一 Trace 串联，由 Prometheus+Alertmanager 统一告警，由 Argo CD 统一声明式部署与回滚。

---

## 21. 与详细设计的衔接

| 详细设计文档（计划） | 承接本书章节 |
|---|---|
| `03-详细设计/模块设计/` | §4 各微服务内部模块设计（translation-core 编排逻辑迁移、各媒体处理服务算法细节） |
| `03-详细设计/接口设计/` | §4.2、§9 各服务 REST/gRPC/事件契约完整定义（OpenAPI/Protobuf） |
| `03-详细设计/数据库设计/` | §5、§7 各逻辑库完整 DDL、索引、Outbox 表迁移脚本 |

本书为 CATs 微服务架构的基础设计基线，后续详细设计与实现均以本书章节结论为约束条件，如需偏离须先更新本书并走 §0 文档管理信息中的评审流程。
