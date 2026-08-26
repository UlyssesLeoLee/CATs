# CATs ADR-001：微服务架构选型

> **文档编号**：CATs-ADR-001
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师
> **状态**：已接受
> **取代**：—

---

## 1. 背景

CATs（AI 增强型 CAT 浏览器工作台）需要支撑：

- 15 个核心服务（翻译引擎 / TM / 术语 / LLM 网关 / 协作 / 项目管理 / 权限等）
- 多语言大规模 TM（10万+ 条），向量检索
- CRDT 协同编辑（Yjs + WebSocket）
- LLM 流式翻译
- 多租户隔离

单体架构在 ① 横向扩展粒度 ② 故障隔离 ③ 团队并行开发 ④ 技术异构（Rust 核心 + Node 协同网关）四点上不达标。

## 2. 选项

| 选项 | 描述 | 优势 | 劣势 |
|------|------|------|------|
| **A. 单体（Modular Monolith）** | 单进程多模块 | 部署简单、调用零开销 | 单点故障、扩展粒度粗、技术栈锁定 |
| **B. 微服务（15 服务 + 4 共享库）** ✅ | 服务按领域拆分，独立部署 | 横向扩展粒度细、故障隔离、团队并行、技术异构 | 分布式复杂度（事务/观测/部署） |
| C. Serverless（Fn Project） | 函数级拆分 | 极致弹性 | 冷启动延迟、状态管理难、不适合长连接（Yjs WS） |

## 3. 决策

**采用 B：微服务架构（15 服务 + 4 共享库）**。

服务边界（DD 评审 v1.0 §2.1 已锁定）：
- **核心域 5**：tm / term / llm-gateway / translate-orchestrator / qa-engine
- **支撑域 6**：project / document / collab-ws / collab-persistence / notify / audit
- **平台域 4**：auth / billing / gateway-bff / admin
- **共享库 4**：common-types / common-auth / common-observability / common-grpc

## 4. 影响

- **正面**：M1 启动即可 4 团队并行开发；故障域隔离；Rust 核心 + Node 协同网关异构可行
- **负面**：
  - 需 Outbox 模式保障分布式事务（L-7 已识别）
  - 需统一可观测性栈（OpenTelemetry + Jaeger + Prometheus + Loki）
  - 部署复杂度提升 → 已要求 53-58 任务出 CI/CD 構築運用手順書
- **风险**：
  - QA-012 16 微服务与 SRE 团队能力匹配 → 需 8-25 评审会决议
  - 共享库版本治理 → 需建立 internal-registry

## 5. 关联

- 上游：`CATs_微服务架构设计书_v1.0.md`
- 下游：ADR-002（通信协议）、ADR-003（数据存储）、DD 评审 v1.0 §2.1
- 阻塞项：QA-012
