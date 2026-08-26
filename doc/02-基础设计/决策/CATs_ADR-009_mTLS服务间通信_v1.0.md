# CATs ADR-009：服务间 mTLS 通信

> **文档编号**：CATs-ADR-009
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师(worker 代签 per DEC-008)
> **状态**：已接受
> **取代**：—

---

## 1. 背景

ADR-002 已落定服务间通信走 gRPC，但"传输层安全"未在 ADR-002 中定。15 微服务在 Kubernetes 内部东西向流量当前为明文 gRPC(应用层 TLS 由调用方各自处理)，存在：

- **窃听风险**：集群内任意 Pod 可嗅探未加密 gRPC 流量(集群被突破即泄露翻译原文 / TM 语料)
- **伪造风险**：恶意 Pod 可伪造源服务身份调用关键域(如 audit / billing)
- **合规压力**：合规项目(政府 / 金融行业翻译)要求传输层双向认证
- **多租户隔离**：跨租户流量需在网络层强隔离，不能仅靠应用层 JWT(参见 ADR-008)

QA-074 已登记为 Open 项，要求服务间通信加密与双向认证明确方案。L-7 风险登记册"东西向流量零信任"对应此 ADR。

要决策：

- **mTLS 实施层**：应用层 vs sidecar vs 完整服务网格
- **证书管理**：自建 CA / cert-manager / 服务网格托管
- **隔离维度**：仅 mTLS / mTLS + NetworkPolicy / 完整零信任

## 2. 选项

| 选项 | 描述 | 优势 | 劣势 |
|------|------|------|------|
| **A. 应用层 mTLS(自管 cert-manager)** ✅ | Rust 服务用 `rustls` / tonic-tls，Node 服务用 `grpc-js` + 自签证书 | 控制精细、无 sidecar 开销、与 Rust 性能预算兼容 | 证书分发逻辑需自研或用 cert-manager，团队负担中等 |
| **B. Linkerd sidecar** | 轻量服务网格自动注入 sidecar mTLS | 一键启用、零代码改动 | 增加每 Pod ~50MB 内存 + 一跳代理延迟，Rust 性能收益被吃 |
| **C. Istio 全网格** | 完整服务网格 | 能力最全(流量管理 / 可观测 / 安全) | 运维重、SRE 团队 2 人无法独立运维(参见 NFR-OP-010)、CRD 复杂度高 |
| **D. 仅 NetworkPolicy(无 mTLS)** | 仅靠 K8s NetworkPolicy 隔离 | 最轻 | 不解决"集群内嗅探"与"伪造源服务"两个核心威胁 |

补充说明：

- 选项 B / C 与 SRE 团队能力不匹配(L-7 风险登记册中已识别 NFR-OP-010：2 SRE ≤ 20 人·天/周)
- 选项 D 不能满足合规要求，排除
- 本 ADR 决策"应用层 mTLS + NetworkPolicy 双重"，对应选项 A

## 3. 决策

**采用 A + NetworkPolicy 双重：应用层 mTLS 走自管 cert-manager(短期) / Linkerd sidecar(中期评估)；叠加 K8s NetworkPolicy 做网络层隔离；不引入完整服务网格。**

实施细则：

1. **应用层 mTLS**
   - **证书签发**：cert-manager + 内置 ClusterIssuer(短期方案)
   - **证书生命周期**：每服务一张证书，TTL 24h，前 12h 自动续期
   - **SAN 包含**：服务 K8s Service DNS(`<svc>.<ns>.svc.cluster.local`)
   - **Rust 侧**：tonic 启用 TLS，`common-grpc` 封装 `mtls_config()` 共享配置(参见 ADR-001 / ADR-002 共享库)
   - **Node 侧**：grpc-js 加载 `tls.crt` / `tls.key` / `ca.crt`，由 init container 挂载
2. **mTLS 强制策略**
   - 服务间 gRPC 必须走 mTLS，明文端口 50051 默认不暴露
   - 临时调试例外通过 `MeshException` CRD(短期手写 YAML)申请，SRE 双人复核
3. **NetworkPolicy 叠加**
   - 默认 deny-all，按 namespace + label 显式放行
   - 跨域调用显式声明 `IngressSource` / `EgressTarget` 规则
   - 租户隔离：跨租户 Pod 不能直接通信(参见 ADR-005 多租户)
4. **中期评估 Linkerd**
   - M3 末根据"应用层 mTLS 运维负担"数据决定是否切到 Linkerd sidecar
   - 评估触发：续期失败率 > 0.1% / 月 或 SRE 团队扩张到 ≥ 4 人
5. **审计与可观测**
   - mTLS 握手失败事件写 `audit` 表
   - Prometheus 指标 `mtls_handshake_total{result="success|failure",service="..."}`

## 4. 影响

- **正面**：
  - 传输层零信任达成，满足合规项目硬性要求
  - 与 ADR-002 gRPC / ADR-005 多租户 / ADR-008 JWT 形成"网络层 + 身份层"双层防御
  - 不引入 sidecar → Rust 服务性能预算不受影响
  - cert-manager 已是 K8s 生态事实标准，团队熟悉
- **负面**：
  - 服务启动依赖 cert-manager 可用 → 启动顺序与 readiness probe 需调整
  - 调试链路略复杂(grpcurl 需带客户端证书)，已在 53 任务规划排查手册
  - 短期方案自管证书分发，团队需承担 cert-manager 运维
- **风险**：
  - cert-manager 单点故障 → 已在选型 024 中规划 HA 部署
  - 跨集群(multi-cluster)场景证书信任链复杂 → 当前架构单集群，暂不评估
  - 中期切 Linkerd 时应用代码已用 `common-grpc` 抽象，迁移成本可控但非零 → M3 末评审必查

## 5. 关联

- **上游**：ADR-002(gRPC 通信)、ADR-005(Keycloak + 多租户)、ADR-008(JWT)
- **下游**：可观测性平台设计 v1.0(mTLS 指标)、53 任务(CI/CD 含 cert 巡检)
- **阻塞项**：QA-074(mTLS 服务间通信)
