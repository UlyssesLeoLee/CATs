# CATs CI/CD 構築・運用手順書 v1.0

> **文档编号**：CATs-IMPL-053  
> **フェーズ**：53/57/58 開発環境構築 / ビルド / CI  
> **关联任务**：150 任务 #53、#57、#58、QA-041（性能基线）、QA-042（K3s HA）  
> **版本**：v1.0（评审会前草稿）  
> **创建日**：2026-08-20  
> **作者**：SRE + 架构师

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| SRE | ☐ | — |
| 架构师 | ☐ | — |
| QA | ☐ | — |
| PM | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-20** | **SRE** | **评审前草稿：CI/CD + 容器 + GitOps 整合** |

---

## 1. 目的

定义 CATs 项目的 **CI/CD 流水线、容器化、镜像仓库、环境管理**全部标准，作为：

- 开发者上手（M1-S0）的统一基线
- 自动化部署 / 灰度 / 回滚的实施依据
- 评审会、QA-042（K3s HA）讨论的统一基线

---

## 2. 范围

| 维度 | 范围 |
|------|------|
| **应用** | 15 微服务 + 浏览器扩展 + BFF + Worker |
| **环境** | Dev / Staging / Pre-prod / Prod |
| **工具** | GitHub Actions + 自建 Runner + Harbor + ArgoCD |
| **平台** | K3s（QA-042） |
| **不在** | 业务部署流程（见 105-108 リリース） |

---

## 3. 工具链总览

```
┌──────────┐    ┌──────────────┐    ┌──────────┐    ┌──────────┐
│  GitHub  │ →  │  GitHub      │ →  │  Harbor  │ →  │  ArgoCD  │
│  PR/MR   │    │  Actions     │    │  镜像    │    │  GitOps  │
│          │    │  CI Runner   │    │  仓库    │    │          │
└──────────┘    └──────────────┘    └──────────┘    └──────────┘
                                                            ↓
                                                     ┌──────────────┐
                                                     │   K3s 集群   │
                                                     │  Dev/Stg/PP/Prod │
                                                     └──────────────┘
```

| 类别 | 工具 | 用途 |
|------|------|------|
| 代码托管 | GitHub Enterprise | Git + PR + Code Review |
| CI | GitHub Actions | Lint / Test / Build / Scan / Push |
| 镜像仓库 | Harbor（私有） | 镜像存储 + 签名 + 扫描 |
| CD | ArgoCD | GitOps 部署 |
| 密钥 | Sealed Secrets + Vault | 密钥分发 |
| 监控 | Prometheus + Grafana | CI/CD 自身 + 应用 |

---

## 4. 開発環境構築

### 4.1 硬件

| 角色 | 设备 | 配置 |
|------|------|------|
| 开发者（Rust） | MacBook Pro M3 / Linux 工作站 | 32GB / 1TB |
| 开发者（前端） | MacBook Pro M3 | 32GB / 1TB |
| SRE | Linux 工作站 + 多 VM | 64GB / 2TB |
| DBA | Linux 工作站 | 32GB / 1TB |

### 4.2 基础工具

| 工具 | 版本 | 用途 |
|------|------|------|
| Git | 2.42+ | 版本控制 |
| Rust | 1.79+ (stable) | 后端开发 |
| cargo | 1.79+ | Rust 包管理 |
| Node.js | 20 LTS | 前端开发 |
| pnpm | 9+ | 前端包管理 |
| Docker | 24+ | 容器构建 |
| kind / minikube | latest | 本地 K8s |
| kubectl | 1.28+ | K8s 客户端 |
| k9s | latest | K8s TUI |
| Sops | latest | 密钥加密 |

### 4.3 IDE

- Rust：RustRover / VS Code + rust-analyzer
- 前端：VS Code + Volar
- SQL：DataGrip / DBeaver
- API：Insomnia / Bruno

### 4.4 本地启动

```bash
# 1. 克隆仓库
git clone https://github.com/cats-org/cats.git
cd cats

# 2. 安装依赖
./scripts/bootstrap.sh

# 3. 启动本地 K8s（kind）
./scripts/kind-up.sh

# 4. 部署开发环境
./scripts/dev-deploy.sh

# 5. 启动开发服务
cargo run -p bff          # 后端
pnpm --filter web dev     # 前端

# 6. 打开浏览器
open http://localhost:3000
```

详细见 `CATs_開発者ガイド_v1.0.md`。

---

## 5. コンテナビルド

### 5.1 基础镜像

| 镜像 | 来源 | 用途 |
|------|------|------|
| `distroless/cc-debian12` | Google | Rust 运行时 |
| `gcr.io/distroless/nodejs20-debian12` | Google | Node 运行时 |
| `alpine:3.19` | Docker Hub | 工具/脚本 |
| `debian:12-slim` | Docker Hub | LLM 推理 |

### 5.2 Dockerfile（Rust 示例）

```dockerfile
# ---- Build Stage ----
FROM rust:1.79-slim AS builder
WORKDIR /app

# 缓存依赖
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main(){}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

COPY . .
RUN cargo build --release --locked

# ---- Runtime Stage ----
FROM gcr.io/distroless/cc-debian12:nonroot
COPY --from=builder /app/target/release/bff /bff
EXPOSE 8080
USER nonroot:nonroot
ENTRYPOINT ["/bff"]
```

### 5.3 多服务构建

```bash
# 一次性构建所有服务
./scripts/build-all.sh

# 单服务构建
./scripts/build.sh bff v1.0.0
```

### 5.4 镜像标签策略

| 标签 | 用途 | 示例 |
|------|------|------|
| `<git-sha>` | 不可变追踪 | `abc1234567` |
| `<service>:<git-sha>` | 显式 | `bff:abc1234567` |
| `<service>:latest` | 默认（**禁用 prod**） | — |
| `<service>:v<semver>` | 稳定版本 | `bff:v1.2.3` |

---

## 6. 镜像仓库（Harbor）

### 6.1 部署

- 4 节点分布式（QA-042 HA 评估）
- 私有部署（不暴露公网）
- 与 K3s 集群同 VPC

### 6.2 项目结构

```
harbor.cats.local/
├── cats/                  # 公开
│   ├── bff
│   ├── tm-service
│   ├── term-service
│   ├── ...
│   └── workers
├── cats-internal/         # 内部
│   ├── ops-tools
│   ├── ci-runners
│   └── dev-images
└── cats-lts/              # 长期支持
    └── postgres
```

### 6.3 安全

- 镜像扫描：Trivy（每次 push）
- 镜像签名：cosign
- 漏洞白名单：CVE 评分 + 业务影响
- 访问控制：OIDC（与 GitHub 集成）

---

## 7. CI Pipeline（GitHub Actions）

### 7.1 工作流总览

```
┌────────┐
│  PR     │
└────┬───┘
     ↓
┌────────────┐
│  Lint     │ (clippy, eslint, ruff, sqlfluff)
└────┬───────┘
     ↓
┌────────────┐
│  Test     │ (cargo test, pytest, vitest)
└────┬───────┘
     ↓
┌────────────┐
│  Build    │ (cargo build, tsc, pnpm build)
└────┬───────┘
     ↓
┌────────────┐
│  SAST     │ (clippy strict, semgrep, snyk)
└────┬───────┘
     ↓
┌────────────┐
│  SBOM     │ (cargo-cyclonedx, syft)
└────┬───────┘
     ↓
┌────────────┐
│  Scan     │ (trivy fs + image)
└────┬───────┘
     ↓
┌────────────┐
│  Sign     │ (cosign)
└────┬───────┘
     ↓
┌────────────┐
│  Push     │ (Harbor)
└────────────┘
```

### 7.2 触发条件

| 触发 | 工作流 | 范围 |
|------|--------|------|
| PR 创建/更新 | `pr-check` | 改动服务 + 依赖服务 |
| 推 main | `main-build` | 全部 + 镜像 + 部署 staging |
| 推送 tag | `release-build` | 全部 + 镜像 + 部署 pre-prod |
| 手动 | `manual-deploy` | 自选环境 + 服务 |

### 7.3 关键检查

- [ ] Lint 全过
- [ ] UT 全过 + 覆盖率 ≥ 80%
- [ ] Build 成功
- [ ] SAST 0 高危
- [ ] 镜像扫描 0 严重
- [ ] SBOM 生成
- [ ] 镜像签名成功
- [ ] 镜像推送成功

### 7.4 Runner 配置

- GitHub-hosted：基础（lint / test）
- Self-hosted：K3s + 内部网络（build / push / deploy）
- Self-hosted ARM：Rust 交叉编译（少量）

---

## 8. CD Pipeline（ArgoCD + GitOps）

### 8.1 GitOps 模型

```
GitOps Repo (cats-deploy)         K3s 集群
┌─────────────────────────┐        ┌──────────────┐
│  apps/                  │   →    │              │
│  ├── bff/              │ ArgoCD  │  bff Pods    │
│  │   ├── base/          │ reconcile│              │
│  │   └── overlays/      │        │              │
│  │       ├── dev/       │        │              │
│  │       ├── staging/   │        │              │
│  │       ├── pre-prod/  │        │              │
│  │       └── prod/      │        │              │
│  └── ...                │        │              │
└─────────────────────────┘        └──────────────┘
```

### 8.2 部署策略

| 环境 | 策略 | 触发 |
|------|------|------|
| Dev | Auto（PR 合并即部署） | GitHub |
| Staging | Auto（main 推送） | GitHub |
| Pre-prod | 手动 + 审批 | 手动 |
| Prod | 手动 + CAB + Canary | 手动 + Argo Rollouts |

### 8.3 蓝绿 / Canary（Prod）

- 工具：Argo Rollouts
- 阶段：5% → 25% → 50% → 100%
- 自动回滚：成功率 < 99% 或 错误率 > 1%
- 中间等待：10 分钟
- 监控：metrics / traces / logs

### 8.4 回滚

```bash
# ArgoCD 同步到上一版本
argocd app sync bff --revision <previous-sha>

# 或
argocd app rollback bff
```

---

## 9. 环境管理

### 9.1 环境清单

| 环境 | 用途 | 规模 | 数据 | 访问 |
|------|------|------|------|------|
| **Dev** | 开发者 | 1 节点 K3s | 脱敏测试集 | 开发者 |
| **Staging** | IT 测试 | 3 节点 | 脱敏副本 | QA + 开发者 |
| **Pre-prod** | ST / UAT | 6 节点（缩配） | 生产脱敏 | QA + 客户 |
| **Prod** | 生产 | 6+ 节点 | 真实 | 受控 |

### 9.2 配置差异

| 维度 | Dev | Staging | Pre-prod | Prod |
|------|-----|---------|----------|------|
| 副本数 | 1 | 2 | 2 | 3+ |
| 资源 | 0.5/1G | 1/2G | 1/2G | 2/4G |
| 域名 | dev.cats.local | stg.cats.local | pp.cats.local | cats.example.com |
| 证书 | 自签 | 自签 | 内部 CA | 公共 CA |
| 监控 | 基础 | 全 | 全 + 告警 | 全 + 告警 + 值班 |

### 9.3 命名空间

| 命名空间 | 用途 |
|----------|------|
| `cats-system` | 系统组件（监控 / 备份 / ArgoCD） |
| `cats-dev` | Dev 应用 |
| `cats-staging` | Staging 应用 |
| `cats-preprod` | Pre-prod 应用 |
| `cats-prod` | Prod 应用 |
| `cats-batch` | Worker / 批处理 |

---

## 10. 密钥管理

### 10.1 原则

- 禁止明文密钥入库
- GitOps repo 存 Sealed Secrets
- Vault 存运行时密钥
- 密钥轮转：90 天

### 10.2 工具

| 场景 | 工具 |
|------|------|
| Git 仓库 | Sealed Secrets（公钥加密） |
| 运行时 | HashiCorp Vault / 云 KMS |
| 数据库 | Vault Dynamic Secrets |
| API 调用 | Vault Transit |

### 10.3 流程

```
密钥生成 → Vault 存储 → Sealed Secret 加密 → Git 仓库
                                          ↓
                                  ArgoCD 同步到 K8s
                                          ↓
                                  K8s Secret（运行时解密）
```

---

## 11. 监控与告警

### 11.1 CI/CD 自身

| 指标 | 告警阈值 |
|------|----------|
| Workflow 失败率 | > 5% |
| Workflow 时长 P95 | > 30min |
| Runner 队列 | > 5 个等待 |
| 镜像构建失败 | 立即 |
| 镜像扫描严重漏洞 | 立即 |

### 11.2 应用

详见 `CATs_可热插拔部署与运维设计_v1.0.md` §9。

---

## 12. 故障处理

### 12.1 CI 失败

| 现象 | 处理 |
|------|------|
| Lint 失败 | 看报告 + 修代码 |
| Test 失败 | 看报告 + 修代码 + 补用例 |
| Build 失败 | 看日志 + 检查依赖 |
| SAST 失败 | 修复 / 豁免（需审批） |
| 镜像扫描失败 | 修复 / 替换基础镜像 / 豁免 |
| 推送失败 | 检查 Harbor 凭据 / 网络 |

### 12.2 CD 失败

| 现象 | 处理 |
|------|------|
| 同步失败 | ArgoCD 重试 / 检查 Git |
| 健康检查失败 | 自动回滚 |
| Canary 异常 | Argo Rollouts 自动暂停 + 告警 |
| 部署超时 | 检查 K8s 资源 + 日志 |

### 12.3 Runner 故障

- Self-hosted：自动重启 + 通知
- 磁盘满：自动清理
- 网络断：重试 + 告警

---

## 13. 与 150 任务 / QA 关联

| 任务 / QA | 关联 |
|-----------|------|
| 53 開発環境構築 | 本文 §4 |
| 57 ビルド | 本文 §5-6 |
| 58 CI | 本文 §7 |
| 104 本番環境構築 | 衍生 |
| 105 本番デプロイ | §8 CD |
| 106 稼働確認 | §8 健康检查 |
| QA-041 PG 性能基线 | §11 监控 |
| QA-042 K3s HA | §9 环境 + §6 Harbor HA |

---

## 14. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_開発者ガイド v1.0 | `05-其他\治理\` |
| CATs_品質ゲート運用手順書 v1.0 | `05-其他\治理\` |
| CATs_可热插拔部署与运维设计 v1.0 | `02-基础设计\架构设计\` |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` |
| CATs_Rust 技术选型书 v1.0 §3.3 CI | `02-基础设计\技术选型\` |
| CATs_测试设计书 v1.0 §10 性能 | `04-测试\测试设计书\` |
| CATs_安全要件定义书 v1.0 §10.2 镜像 | `05-其他\安全\` |

---

## 15. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | Runner 资源规模（GitHub vs Self） | SRE | M1-S0 |
| OI-2 | Harbor HA 部署（4 节点） | SRE | M1-S0 |
| OI-3 | ArgoCD + Argo Rollouts 部署 | SRE | M1-S0 |
| OI-4 | Sealed Secrets 密钥对管理 | SRE | M1-S0 |
| OI-5 | Vault 部署模式（自建 / 云） | SRE + 架构 | M1-S0 |
| OI-6 | QA-042 K3s HA 决议影响 | 架构 + SRE | 评审会 D+2 |

---

**文档结束**
