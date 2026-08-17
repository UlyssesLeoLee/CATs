# CATs 技术选型书

**系统名称:** CATs — 全媒体 AI 辅助翻译 SaaS 平台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | CATs-TS-002 |
| 文档名 | 技术选型书（含选型决策记录 ADR，v2.0——微服务/全媒体版） |
| 版本 | 第 2.0 版（草稿） |
| 创建日 | 2026-08-17 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [OFCAT 技术选型书 v1.0（历史/旧架构参考）](./OFCAT_技术选型书_v1.0.md)、[CATs 微服务架构设计书 v1.0](../架构设计/CATs_微服务架构设计书_v1.0.md)、[CATs 命名变更说明](../架构设计/CATs_命名变更说明.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | （OFCAT）薄扩展+本地引擎技术栈选型，见历史文档 |
| 2.0 | 2026-08-17 | 架构师 | 全面重做：Rust 原生客户端 + Next.js 控制台 + K3s 微服务平台 + 全媒体处理管线技术选型 |

---

## 1. 前言

### 1.1 目的
记录 CATs 从「薄浏览器扩展 + 单机引擎」转型为「Rust 原生客户端 + Next.js 控制台 + K3s 微服务 SaaS 平台」后，各技术层的候选方案对比、评分与最终决策，作为 [CATs 微服务架构设计书](../架构设计/CATs_微服务架构设计书_v1.0.md) 的选型依据。

### 1.2 选型总原则
1. **开源免费、可商用**：全部候选须为 OSS 且许可证（Apache-2.0/MIT/MPL/BSD 等）允许商用，不含仅限非商用的模型/软件。
2. **适配局域网私有化部署**：优先选择可离线安装、不强依赖外网服务（如 Docker Hub、GitHub Container Registry 需可被内部镜像仓库如 Harbor 代理）的方案。
3. **不过度设计**：新增组件须能在 §2 的评估矩阵中说明「解决什么问题 / 当前规模是否需要 / 不用会怎样」。
4. **评估维度与权重同 v1.0**：适配性 30%、体验性能 25%、可维护性 20%、落地成本 15%、合规安全 10%，1–5 分制。

---

## 2. 选型总览

| 层 | 决策 | 主要替代 | 综合分 |
|---|---|---|---|
| 桌面/原生客户端 | **Tauri 2.x（Rust 核心 + Svelte 前端渲染于 WebView）** | egui/iced 纯 Rust GUI / Electron | 4.6 |
| 客户端内前端框架 | Svelte 5 + TypeScript（Tauri WebView 内） | React / SolidJS | 4.3 |
| Web 控制台/BFF | **Next.js 14+（App Router）** | Remix / SvelteKit | 4.4 |
| 容器编排 | **K3s（3 控制面 + N 工作节点）** | 标准 K8s（kubeadm）/ K0s | 4.5 |
| 服务网格/入口 | **Envoy Gateway（Kubernetes Gateway API）** | Nginx Ingress / Traefik | 4.4 |
| 核心翻译引擎语言 | Python（FastAPI，沿用 OFCAT 编排逻辑）+ Rust（渲染回写高性能路径） | 纯 Python / 纯 Rust | 4.3 |
| 主存储 | **PostgreSQL 16（CloudNativePG 管理）** | MySQL / CockroachDB | 4.6 |
| 缓存/会话/限流 | **Valkey（Redis 兼容 Fork）** | Redis（许可证变更）/ KeyDB | 4.5 |
| 消息队列 | **Kafka（KRaft 模式，无 ZooKeeper）** | NATS JetStream / RabbitMQ | 4.3 |
| CDC | **Debezium（Kafka Connect）** | 自研双写 | 4.4 |
| 镜像仓库 | **Harbor** | 自建 Registry / Nexus | 4.4 |
| 可观测性-指标 | **Prometheus + Alertmanager** | VictoriaMetrics | 4.5 |
| 可观测性-链路 | **OpenTelemetry + Tempo/Jaeger** | Zipkin | 4.4 |
| 日志 | **Loki + Promtail（或 Vector）** | ELK（重） | 4.2 |
| CI/CD | **Argo CD（GitOps）+ 现有 CI（Gitea Actions/Jenkins）** | Flux CD | 4.3 |
| ASR（语音转写） | **faster-whisper（CTranslate2 后端）** | openai-whisper 原版 / Vosk | 4.5 |
| 字幕/时间轴 | **自研 subtitle-service（基于 pysrt/webvtt-py + ffmpeg）** | Aegisub 库 | 4.2 |
| TTS/配音钩子 | **Coqui TTS / Piper（可插拔接口，MVP 仅留钩子不强依赖）** | 商用云 TTS | 3.8（延后） |
| 视频/音频处理 | **ffmpeg** | GStreamer | 4.7 |
| 通用 OCR | **PaddleOCR（中日英表格版面强）** | Tesseract | 4.3 |
| 手写/复杂版面 OCR 备选 | Tesseract 5（LSTM） | — | 3.9 |
| PDF 文字层解析 | **pdfplumber / PyMuPDF(fitz)** | pdf.js（仅浏览器渲染） | 4.3 |
| PDF 版面保留回写 | **PyMuPDF + ReportLab 混合重排** | LibreOffice 转换链 | 4.0 |
| Office 文档（docx/xlsx/pptx/ODF） | **LibreOffice Headless（soffice --headless）统一转换 + python-docx/openpyxl/python-pptx 做结构化读写** | 纯 OOXML SDK 自研 | 4.2 |
| 动图（GIF/WebP）帧处理 | **ffmpeg 抽帧 + PaddleOCR 逐帧 OCR + Pillow 重编码回写** | gifsicle（仅编辑不含 OCR） | 4.1 |
| TM 匹配 | RapidFuzz + bge-m3 嵌入（沿用 OFCAT） | 纯向量 | 4.5 |
| AI 网关 | LiteLLM + 自研合规路由（沿用 OFCAT） | 全自研 | 4.3 |
| 向量检索（TM 语义召回） | **PostgreSQL + pgvector 扩展** | 独立向量库（Qdrant/Milvus） | 4.3 |

---

## 3. 选型决策记录（ADR）

### ADR-14 桌面/原生客户端：Tauri 2.x（Rust 核心）

- **背景**：原 MV3 浏览器扩展无法承载 ffmpeg 解码、本地 OCR 预处理、长时间后台任务、本地文件系统深度访问；需要独立原生客户端。
- **候选**：
  1. **Tauri 2.x**：Rust 后端 + 系统原生 WebView（WKWebView/WebView2/WebKitGTK）渲染前端，前端可用任意现代框架。
  2. **egui / iced**：纯 Rust immediate-mode / Elm-架构 GUI 工具包，无 WebView，UI 完全用 Rust 代码绘制。
  3. **Electron**：Chromium + Node.js，生态成熟但资源占用大。
- **决策**：Tauri 2.x，内嵌 Svelte 5 + TypeScript 作为前端渲染层。
- **理由**：
  - 相比 Electron：Tauri 不内置完整 Chromium（复用系统 WebView），安装包体积（约 3–10MB 基座）远小于 Electron（约 80–120MB），内存占用低 3–5 倍，更适合"翻译工作台"这种需要长期挂在系统托盘的应用。
  - 相比纯 Rust GUI（egui/iced）：CATs 客户端需要展示富文本对照编辑、字幕时间轴、PDF/Office 版面预览等复杂交互 UI，用 Web 技术栈（HTML/CSS + 成熟组件库）实现的开发效率远高于用 immediate-mode Rust GUI 手绘；且 Svelte 前端团队/生态与 Web 控制台（Next.js）可复用大量 UI 组件设计语言与 TypeScript 类型定义（通过共享 API 契约包），降低维护成本。
  - Tauri 2.x 相比 1.x 新增移动端（iOS/Android）打包能力、更细粒度的权限系统（capabilities/permissions 模型），为未来移动客户端预留路径。
  - Rust 核心可直接调用/内嵌轻量原生库（如本地 ffmpeg 探测、文件监控、系统托盘、自动更新），比 Electron 的 Node 原生模块桥接更安全（内存安全）、启动更快。
  - 重计算任务（ASR/OCR/大文件转换）不在客户端本地执行，而是提交到后端微服务集群处理，客户端主要承担：采集/预览/编辑/进度展示/本地缓存，因此不需要 Rust GUI 承担繁重计算 UI，Tauri 的"胶水"定位完全够用。
- **取舍**：WebView 版本在不同操作系统上渲染细节有差异（尤其 Windows WebView2 需要预装或随包分发运行时）→ 打包时强制内嵌 WebView2 Evergreen Bootstrapper 离线包，避免局域网环境无法联网下载。
- **结论**：**Tauri 2.x + Rust 核心 + Svelte 5 WebView 前端**，作为 CATs 客户端唯一形态，替代原浏览器扩展。

### ADR-15 Web 控制台/BFF：Next.js（App Router）

- **候选**：Next.js（App Router）/ Remix / SvelteKit。
- **决策**：Next.js 14+，App Router，作为客户面 Web 控制台（用户自助管理、管理后台、认证登录页/OAuth 回调、账单与用量展示、项目/术语库管理 UI）与轻量 BFF（Backend-For-Frontend，聚合后端微服务 API、处理 Session/Cookie、SSR 首屏）。
- **理由**：
  - 生态与团队可招聘性最广，配套 Auth.js（NextAuth）可对接自建 auth-service 做 OIDC/Session；App Router 的 Server Components + Route Handlers 天然适合做 BFF 聚合层，减少客户端直连多个微服务的复杂度与 CORS 面。
  - 开源、Apache-2.0/MIT 双许可（Vercel 主导但代码开源，自托管不依赖 Vercel 云服务，符合局域网私有化部署要求）。
  - 明确定位：Next.js **只承担控制台/门户/BFF**，不承担翻译核心计算（ASR/OCR/翻译管道），避免把 Node.js 进程当作重任务执行体（Node 不适合 CPU 密集型媒体处理）。
- **取舍**：与 Tauri 客户端存在两套前端技术栈（Svelte in Tauri vs Next.js React）→ 可接受，二者面向不同用户群（客户端面向译员日常工作台，Web 控制台面向管理员/自助配置），并通过共享的 TypeScript API 类型包（从 OpenAPI/Protobuf 生成）保持契约一致。

### ADR-16 容器编排：K3s

- **候选**：标准 Kubernetes（kubeadm）/ K3s / K0s。
- **决策**：K3s，3 控制面节点（HA，嵌入式 etcd 或外置 PostgreSQL 作为 datastore）+ N 工作节点。
- **理由**：K3s 是 CNCF 认证的轻量级 Kubernetes 发行版，单二进制、默认打包 containerd/Flannel/CoreDNS/Traefik（Traefik 可禁用替换为 Envoy Gateway），资源占用远低于标准 K8s，非常适合局域网私有化的中小规模集群（本方案 50–3000 并发用户量级），同时 100% 兼容标准 K8s API，未来需要迁移到标准 K8s/托管 K8s 时零改造。
- **取舍**：K3s 默认 SQLite 单机数据存储不满足 HA → 生产环境需配置 3 控制面节点 + 外置 etcd 集群或使用 K3s 内嵌 etcd HA 模式（`--cluster-init`）。

### ADR-17 入口网关：Envoy Gateway（Kubernetes Gateway API）

- **候选**：Nginx Ingress Controller / Traefik / Envoy Gateway。
- **决策**：Envoy Gateway，基于标准 Gateway API（`Gateway`/`HTTPRoute`/`GRPCRoute`/`TCPRoute`）。
- **理由**：Gateway API 是 Ingress API 的官方后继标准，天然支持 HTTP/2、gRPC、WebSocket（客户端与后端的翻译流式推送）、灰度发布（`HTTPRoute` weight 分流）、更细粒度的流量策略（`BackendTrafficPolicy` 限流/熔断/重试）；Envoy 本身是云原生服务网格事实标准数据面，性能与可观测性成熟，社区活跃，Apache-2.0 许可。
- **取舍**：比 Nginx Ingress 学习曲线更陡（Gateway API 概念更多）→ 通过团队内部 Gateway API 使用手册与模板化 Helm values 降低门槛。

### ADR-18 主存储：PostgreSQL（唯一权威存储）

- **决策**：PostgreSQL 16，CloudNativePG（CNPG）Operator 管理，按微服务拆分逻辑库（`auth_db`/`user_db`/`project_db`/`task_db`/`report_db` 等），每库独立账号/最小权限。
- **候选与对比**：见架构设计书 §5.6 HA 方案对比（CloudNativePG vs Patroni vs Stolon）。
- **理由**：PostgreSQL 功能最全（JSONB、全文检索、pgvector 扩展支持向量召回、成熟的 PITR/流复制生态），是本方案**唯一权威数据存储**，杜绝"缓存/消息队列兼做存储"的反模式。CloudNativePG 是 CNCF Sandbox 项目，原生 K8s Operator 模式，比 Patroni（需要额外的 DCS 如 etcd/Consul 才能选主）在 K8s 环境下运维更简单，Apache-2.0 许可。
- **取舍**：CNPG 生态比 Patroni 年轻 → MVP 阶段用 CNPG；若后续遇到 CNPG 未覆盖的高级需求（如跨区域多活）可平滑切换 Patroni（同样基于标准 PostgreSQL，SQL 层无锁定）。

### ADR-19 缓存层：Valkey

- **决策**：Valkey（Linux Foundation 主导的 Redis 开源 Fork，BSD-3 许可）替代 Redis，用途严格限定为**缓存、会话、限流计数、分布式锁、热数据**，**明令禁止**作为任何业务数据的权威存储（详见架构设计书 §11）。
- **理由**：Redis 自 7.4 起改为 RSALv2/SSPL 双许可，对某些商用/云托管场景有限制；Valkey 是社区在许可证变更后 fork 的延续项目，协议/命令 100% 兼容 Redis，BSD-3 完全宽松可商用，是当前开源社区的主流迁移目标（AWS/Google/Oracle 等联合支持）。
- **取舍**：无。

### ADR-20 消息队列：Kafka（KRaft 模式）

- **候选**：Kafka（KRaft）/ NATS JetStream / RabbitMQ。
- **决策**：Kafka 3.x+，KRaft 模式（不部署 ZooKeeper），3 broker 起步。
- **理由**：Kafka 生态最成熟，与 Debezium CDC、Schema Registry、Kafka Connect 集成度最高，适合本方案"审计/事件溯源/多消费者广播"的核心诉求（`user.events`/`project.events`/`task.events`/`audit.events`/`notification.events` 等 Topic）；KRaft 模式移除 ZooKeeper 依赖，运维复杂度显著降低，是 Kafka 官方推荐的新部署形态（3.x 起生产可用，4.x 起 ZooKeeper 模式已移除）。
- **取舍**：Kafka 运维成本仍高于 NATS → 严格控制在架构设计书 §1 的"是否需要"判定标准下使用，MVP 阶段先只上线审计与跨服务通知两类 Topic，其余按需增量引入（见架构设计书 §18 路线图）。

### ADR-21 CDC：Debezium

- **决策**：Debezium（Kafka Connect 插件）监听 PostgreSQL 逻辑复制槽（`pgoutput` 插件），替代应用层手写"写 DB 后再写 Kafka"的双写模式。
- **理由**：彻底解决双写不一致问题（DB 写成功但 Kafka 写失败/反之）；结合 Outbox 表模式（业务事务内只写 `outbox_event` 表，由 Debezium 捕获变更并发布），保证「业务落库」与「事件发布」的原子性。Apache-2.0 许可，CNCF 生态广泛验证。
- **取舍**：引入 Kafka Connect 集群与逻辑复制槽会增加 PostgreSQL 主库 WAL 保留压力 → 需监控复制槽积压并设置告警阈值（详见架构设计书 §7）。

### ADR-22 镜像仓库：Harbor

- **决策**：Harbor 作为集群唯一私有 OCI 镜像仓库，所有第三方镜像（PostgreSQL/Kafka/Envoy/Prometheus 等）与自研服务镜像统一先拉取到 Harbor 缓存/推送，K3s 集群只从 Harbor 拉取镜像，不直连 Docker Hub/GHCR。
- **理由**：局域网环境可能无法稳定访问境外镜像仓库；Harbor 提供镜像扫描（Trivy 集成）、RBAC、多项目隔离、镜像签名（Notation/Cosign）、复制策略（可配置定时从公网同步到内网），是自托管 OCI 仓库的事实标准，Apache-2.0 许可。
- **取舍**：无，为局域网 SaaS 的强制要求。

### ADR-23 可观测性：Prometheus + Alertmanager + OpenTelemetry + Loki

- **决策**：指标用 Prometheus + Alertmanager（+ Grafana 展示），链路追踪用 OpenTelemetry SDK/Collector + Tempo（或 Jaeger），日志用 Loki + Promtail/Vector。
- **理由**：CNCF 毕业项目组合，是云原生可观测性事实标准，三者均开源免费；OpenTelemetry 提供统一的 Trace Context 传播规范，可跨越同步 HTTP/gRPC 调用与异步 Kafka 消息（通过在消息 Header 中传播 `traceparent`），实现全链路统一 Trace ID（详见架构设计书 §13）。
- **取舍**：组件数量较多（Prometheus/Alertmanager/Grafana/OTel Collector/Tempo/Loki/Promtail）→ MVP 阶段用 kube-prometheus-stack + Loki-stack 两个 Helm Chart 一次性拉起，避免逐个手工部署。

### ADR-24 CI/CD：GitOps（Argo CD）

- **决策**：Git 仓库（含 Helm values）→ CI（构建镜像、单测、推送 Harbor）→ Argo CD 监听 Git 变更自动/审批后同步到 K3s，dev/test/prod 用不同 Argo CD Application + 独立 Helm values 文件隔离。
- **理由**：GitOps 模式使集群实际状态与 Git 声明状态一致可追溯，回滚即 `git revert` + Argo CD 自动同步，比命令式 `kubectl apply`/`helm upgrade` 更可控；Argo CD 提供 Web UI 差异预览与手动审批网关（用于生产环境变更审批），Apache-2.0 许可。
- **取舍**：需要团队适应 GitOps 工作流（不能手工 kubectl 改集群）→ 制定"禁止手工改集群，一切通过 Git PR"的运维规范。

### ADR-25 ASR（语音转写）：faster-whisper

- **候选**：openai-whisper（原版 PyTorch 实现）/ faster-whisper（CTranslate2 重实现）/ Vosk。
- **决策**：faster-whisper。
- **理由**：相同 Whisper 模型权重下，faster-whisper 基于 CTranslate2 推理引擎，速度提升约 4 倍、显存占用降低约 2 倍，支持 CPU/GPU 混合部署，MIT 许可，模型权重（Whisper 系列）为 MIT 许可可商用；输出含词级时间戳，直接对接字幕生成。
- **取舍**：需要额外维护 CTranslate2 模型转换步骤 → 提供预转换模型下载/离线镜像脚本，纳入 Harbor 内网分发。

### ADR-26 视频/音频处理：ffmpeg

- **决策**：ffmpeg 作为所有音视频处理（抽帧、音轨提取、字幕烧录、格式转换、GIF/WebP 编解码）的统一底层工具，由 `ingestion-service`/`subtitle-service`/`render-writer-service` 以子进程方式调用。
- **理由**：事实标准，LGPL/GPL（视具体编译选项，采用 LGPL 编译版本以保持商用友好，不启用 GPL-only 组件如 x264 的商业限制场景时需评估许可，内部离线部署自用不涉及分发无许可风险），几乎支持所有媒体容器与编解码器。
- **取舍**：无。

### ADR-27 OCR：PaddleOCR（主）+ Tesseract（备选）

- **决策**：延续 OFCAT ADR-11，PaddleOCR 作为主 OCR 引擎（`ocr-service`），用于 PDF 扫描件、图片、视频帧、GIF/WebP 帧的文字检测识别；Tesseract 5 作为特定场景（如纯拉丁文手写体、PaddleOCR 未覆盖语种）的备选可插拔引擎。
- **理由**：PaddleOCR 中日英与表格/版面分析能力强，覆盖 CATs 目标语种；Apache-2.0 许可可商用；作为独立微服务部署，可按 GPU 资源独立扩缩容。

### ADR-28 Office 文档处理：LibreOffice Headless + 语言专用库

- **候选**：LibreOffice Headless 统一转换 / 纯语言 SDK（python-docx/openpyxl/python-pptx/odfpy）/ Aspose 等商业 SDK。
- **决策**：混合方案——**结构化读写**（保留样式、可编辑段落级替换）用 `python-docx`（docx）、`openpyxl`（xlsx）、`python-pptx`（pptx）、`odfpy`（ODF 系列：odt/ods/odp）；**格式互转与兜底渲染**（如老旧 doc/xls/ppt 二进制格式、复杂版面预览图生成）用 LibreOffice Headless (`soffice --headless --convert-to`) 统一转换为标准 OOXML/ODF 或 PDF 再处理。
- **理由**：结构化库可精确定位段落/单元格/文本框做译文回填，保留原格式与版式，是主路径；LibreOffice 作为地表最广泛的开源 Office 兼容套件（MPL-2.0），覆盖率最高，用作二进制老格式与异常文件的兜底转换器，避免自研解析器覆盖所有 Office 版本历史格式的高成本。二者均开源免费可商用，且均可离线安装于内网节点。
- **取舍**：LibreOffice Headless 进程较重、并发实例数需要限制 → `office-converter-service` 内置进程池 + 队列节流，避免突发大量转换请求拖垮节点。

### ADR-29 动图（GIF/WebP）处理

- **决策**：ffmpeg 抽帧为 PNG 序列 → PaddleOCR 逐帧识别文字区域 → 翻译回填后用 Pillow/ffmpeg 重新编码为 GIF（保留帧延迟/循环参数）或 WebP（保留动画容器）。
- **理由**：复用已选定的 ffmpeg + PaddleOCR + Python 图像库组合，无需引入额外专用组件；GIF/WebP 本质是"视频的低配版子集"，抽帧-识别-回写模式与视频字幕烧录管线高度复用（详见架构设计书 §4 render-writer-service）。

### ADR-30 向量检索：PostgreSQL + pgvector（替代 sqlite-vec）

- **候选**：pgvector（PostgreSQL 扩展）/ 独立向量库（Qdrant/Milvus）。
- **决策**：pgvector，与 TM 主表同库同事务。
- **理由**：架构设计原则是"PostgreSQL 唯一权威存储"，引入独立向量库会制造第二个"准权威存储"，违反 §1 不过度设计原则；pgvector 在 TM 量级（预估百万级句对，§17 容量规划已覆盖）下 HNSW 索引性能足够，且向量与结构化字段可在同一 SQL 事务/查询中联合过滤（按 project_id/语言对过滤 + 向量召回一步到位），比独立向量库的"先过滤 ID 再跨库查询"更简洁。
- **取舍**：超大规模（千万级以上）向量检索性能不及专用向量库 → 达到该量级前无需引入，届时可将 TM 语义索引拆出为独立读模型，不影响主架构。

---

## 4. 被否决方案与理由

| 方案 | 否决理由 |
|---|---|
| Electron 作为客户端框架 | 内置完整 Chromium，安装包/内存占用远大于 Tauri，且团队目标是原生化体验 |
| 纯 Rust GUI（egui/iced）做客户端 | 复杂富文本/时间轴/文档预览 UI 开发效率显著低于 Web 技术栈，非当前团队强项 |
| ZooKeeper 模式 Kafka | KRaft 已生产可用且是官方推荐方向，无需再运维一套 ZooKeeper |
| Redis（社区版新许可） | 许可证变更后对部分商用场景存在合规不确定性，Valkey 是宽松许可的等价替代 |
| 独立向量库（Qdrant/Milvus） | 当前 TM 量级下 pgvector 足够，避免制造第二个"准权威存储"，增加数据一致性维护成本 |
| 应用层手写 DB+Kafka 双写 | 无法保证原子性，天然存在消息丢失/重复窗口，用 Outbox+Debezium CDC 替代 |
| 标准 Kubernetes（kubeadm 全量部署） | 运维复杂度、资源开销均高于 K3s，在 50–3000 用户量级无必要 |
| 商业 Office SDK（Aspose 等） | 非开源、按并发/服务器收费，与"开源免费可商用"选型原则冲突 |
| Nginx Ingress Controller | 不基于标准 Gateway API，灰度/流量策略能力不如 Envoy Gateway 精细，gRPC/WebSocket 支持不如 Envoy 原生 |

---

## 5. 选型相关风险

| 风险 | 影响 | 对策 |
|---|---|---|
| R-06 Tauri 客户端 WebView2 依赖（Windows） | 中 | 打包内嵌 WebView2 离线安装包，避免局域网首次运行需联网下载 |
| R-07 K3s 控制面节点故障域集中 | 高 | 3 控制面节点跨物理机/机架部署，配置 PodDisruptionBudget 与反亲和性 |
| R-08 Kafka 复制槽（Debezium）积压拖垮 PostgreSQL WAL | 高 | Prometheus 监控复制槽 lag，超阈值告警+人工介入清理/扩容磁盘 |
| R-09 LibreOffice Headless 进程僵死/内存泄漏 | 中 | 转换服务设置超时熔断+进程池定期回收重启 |
| R-10 faster-whisper GPU 资源争抢 | 中 | K3s 节点打 GPU 污点+资源配额，ASR 任务走专用队列限流 |
| R-11 Harbor 单点故障阻断全部镜像拉取 | 高 | Harbor 自身以 HA 模式部署（多副本+外置对象存储），并保留本地节点镜像缓存 |
| R-12 微服务数量膨胀导致运维复杂度失控 | 中 | 严格执行架构设计书 §1 的"新增组件四问"判定标准，MVP 期控制在 5–10 个核心服务 |

---

## 6. 结论

CATs 采用「**Tauri 2.x 原生客户端 + Next.js Web 控制台 + K3s 微服务集群（Envoy Gateway 入口 + PostgreSQL 唯一权威存储 + Valkey 缓存 + Kafka(KRaft)/Debezium 事件与 CDC + Harbor 镜像仓库 + Prometheus/OTel/Loki 可观测性 + Argo CD GitOps）+ 全媒体处理管线（faster-whisper/PaddleOCR/ffmpeg/LibreOffice Headless）**」为技术基线，在延续 OFCAT 阶段已验证的 TM/术语/QA 确定性算法与合规 fail-closed 原则基础上，扩展为可支撑 50–3000 并发用户、覆盖文本/音频/视频/PDF/Office/动图全媒体类型的局域网私有化 SaaS 平台。各层均为开源免费可商用组件，具备清晰的分阶段落地路径（详见架构设计书 §18）。
