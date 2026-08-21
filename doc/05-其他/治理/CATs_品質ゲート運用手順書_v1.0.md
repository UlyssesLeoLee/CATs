# CATs 品質ゲート運用手順書 v1.0

> **文档编号**：CATs-IMPL-055  
> **フェーズ**：55/56 静的解析（SAST）/ コードレビュー（CR）  
> **关联任务**：150 任务 #55、#56、#128（QA Review）  
> **版本**：v1.0（评审会前草稿）  
> **创建日**：2026-08-20  
> **作者**：QA + 架构师 + Rust Lead

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| QA 负责人 | ☐ | — |
| 架构师 | ☐ | — |
| Rust Lead | ☐ | — |
| 前端 Lead | ☐ | — |
| PM | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| **v1.0** | **2026-08-20** | **QA** | **评审前草稿：SAST + CR + 评审 + 豁免** |

---

## 1. 目的

定义 CATs 项目的**质量门禁**（CR / SAST / 评审）的统一基线，作为：

- CI/CD 流水线的质量环节
- M1-S0 起的开发者规范
- 评审会、QA 评审（128）的依据
- 缺陷预防的第一道防线

---

## 2. 范围

| 维度 | 范围 |
|------|------|
| 静态分析 | Rust / TypeScript / SQL / IaC |
| 代码评审 | 所有 PR |
| 评审会议 | RD / BD / DD / ST 评审 |
| 豁免 | 漏洞 / 警告 / 规则 |
| 不在 | 性能 / 渗透（属 ST） |

---

## 3. 质量门禁总览

```
┌──────────┐
│  PR 提交 │
└────┬─────┘
     ↓
┌──────────┐     ┌──────────────┐
│  Lint   │ ←─→ │  Rust fmt    │  ← 失败 = 阻止合并
│         │     │  clippy      │
│         │     │  eslint      │
│         │     │  prettier    │
└────┬─────┘     └──────────────┘
     ↓
┌──────────┐     ┌──────────────┐
│  Test   │ ←─→ │  cargo test  │  ← 覆盖率 < 80% = 阻止
│         │     │  vitest      │
│         │     │  pytest      │
└────┬─────┘     └──────────────┘
     ↓
┌──────────┐     ┌──────────────┐
│  SAST   │ ←─→ │  semgrep     │  ← 高危 = 阻止
│         │     │  snyk        │
│         │     │  gitleaks    │
└────┬─────┘     └──────────────┘
     ↓
┌──────────┐
│  人工   │  ← 必须 1+ 评审通过
│  Review │
└────┬─────┘
     ↓
┌──────────┐
│  Merge  │
└──────────┘
```

---

## 4. 静态分析（SAST）

### 4.1 Rust

| 工具 | 用途 | 配置 |
|------|------|------|
| `cargo fmt --check` | 格式 | CI 必跑 |
| `cargo clippy -- -D warnings` | Lint | 严格模式 |
| `cargo audit` | 依赖漏洞 | 每次 PR |
| `cargo deny` | License + 依赖策略 | 每周 |
| `cargo-geiger` | unsafe 统计 | 每周 |
| `semgrep` | 模式扫描 | 每次 PR |

#### 强制规则

- `unsafe_code` 禁止（除特批）
- `unwrap_used` 禁止
- `expect_used` 禁止（生产代码）
- `panic` 禁止
- `print_stdout/print_stderr` 禁止（用 `tracing`）
- `todo`/`unimplemented` 禁止
- 公开 API 必须有文档

#### 配置（`clippy.toml`）

```toml
avoid-breaking-exported-api = false
msrv = "1.79"  # 最低支持版本，CI 实际用最新 stable（QA-024 决议）
```

#### 配置（`Cargo.toml`）

```toml
[lints.clippy]
pedantic = { level = "warn", priority = -1 }
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
print_stdout = "deny"
print_stderr = "deny"
todo = "deny"
unimplemented = "deny"
module_name_repetitions = "allow"
must_use_candidate = "warn"
missing_errors_doc = "warn"
missing_panics_doc = "warn"
```

### 4.2 TypeScript

| 工具 | 用途 | 配置 |
|------|------|------|
| `eslint` | Lint | 严格模式 |
| `prettier --check` | 格式 | CI 必跑 |
| `tsc --noEmit` | 类型 | 严格模式 |
| `pnpm audit` | 依赖漏洞 | 每次 PR |

#### 强制规则

- `any` 禁止
- `@typescript-eslint/no-explicit-any` error
- `@typescript-eslint/no-unused-vars` error
- `no-console` 禁止（用 logger）
- `prefer-const` error

#### 配置（`tsconfig.json`）

```json
{
  "compilerOptions": {
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "exactOptionalPropertyTypes": true
  }
}
```

### 4.3 SQL

| 工具 | 用途 | 配置 |
|------|------|------|
| `sqlfluff` | Lint | 每次 PR |
| `pgspot` | 模式扫描 | 每周 |

#### 强制规则

- 禁止 `SELECT *`
- 禁止字符串拼接
- 必须有表注释
- 必须有字段注释

### 4.4 IaC / Dockerfile

| 工具 | 用途 |
|------|------|
| `hadolint` | Dockerfile |
| `kube-linter` | K8s YAML |
| `checkov` | Terraform / K8s |
| `trivy config` | IaC 扫描 |

### 4.5 Secret 扫描

| 工具 | 时机 |
|------|------|
| `gitleaks` | 每次 PR + pre-commit |
| `trufflehog` | 每周全量 |

---

## 5. 严重度与门禁

| 级别 | 工具 | 处理 | 门禁 |
|------|------|------|------|
| **Critical** | SAST / SCA | 必须立即修复 | 阻止合并 |
| **High** | SAST / SCA | 必须修复或豁免（需审批） | 阻止合并 |
| **Medium** | SAST | 1 周内修复 | 警告 |
| **Low** | Lint | 立即修复 | 警告 |

### 5.1 漏洞分级

| CVSS 3.1 | 等级 | SLA |
|----------|------|-----|
| 9.0-10.0 | Critical | 24h |
| 7.0-8.9 | High | 7d |
| 4.0-6.9 | Medium | 30d |
| 0.1-3.9 | Low | 90d |

---

## 6. 代码评审（CR）

### 6.1 PR 流程

```
开发者 → 推送分支 → 创建 PR → CI 自动 → 人工评审 → 批准 → 合并
   ↑                                          │
   └──────────── 变更请求（评审不通过）─────────┘
```

### 6.2 评审者

| 类型 | 要求 |
|------|------|
| 普通 PR | 1+ 评审者（CODEOWNERS 至少 1） |
| 关键模块 | 2+ 评审者 |
| 安全相关 | 1 架构师 + 1 SRE |
| 数据库 | 1 DBA |
| 发布分支 | 2 评审者 + PM 批准 |

### 6.3 CODEOWNERS

```gitignore
# 默认
*                               @cats-org/all-devs

# 架构
/02-基础设计/                   @cats-org/architects
/CATALOG.md                     @cats-org/architects

# 核心服务
/services/bff/                  @cats-org/bff-team
/services/tm-service/           @cats-org/tm-team
/services/auth-svc/             @cats-org/auth-team
/services/audit-svc/            @cats-org/audit-team

# 数据库
/migrations/                    @cats-org/dba-team

# 安全
/security/                      @cats-org/security-team
```

### 6.4 评审清单

#### 功能性

- [ ] 实现需求（关联 issue / doc）
- [ ] 无破坏性变更（或已沟通）
- [ ] 错误处理完整
- [ ] 日志规范（无敏感信息）
- [ ] 边界条件覆盖

#### 质量

- [ ] 符合编码规范
- [ ] 无新增 lint 警告
- [ ] 测试覆盖率达标
- [ ] 测试有断言
- [ ] 命名清晰

#### 安全

- [ ] 无密钥泄露
- [ ] 无 SQL 注入风险
- [ ] 无 XSS 风险
- [ ] 无 unsafe（除特批）
- [ ] 权限检查到位

#### 性能

- [ ] 无 N+1 查询
- [ ] 无全表扫描
- [ ] 缓存使用合理
- [ ] 资源（连接/内存）释放

#### 可维护性

- [ ] 文档更新
- [ ] 注释合理
- [ ] 公共 API 有文档
- [ ] 依赖合理（无未用）

#### 部署

- [ ] 迁移脚本完整
- [ ] 特性开关（如需）
- [ ] 监控指标（如新增）
- [ ] 文档变更说明

### 6.5 评审 SLA

| 阶段 | SLA |
|------|-----|
| 首次响应 | 4 工作小时 |
| 评审完成 | 1 工作日 |
| 安全 PR | 4 工作小时 |
| 紧急 PR | 1 工作小时 |

### 6.6 评审礼仪

- 礼貌、专业、建设性
- 区分 must / should / nit
- 解释为什么
- 同意时给 LGTM
- 必须修改时给 Request Changes

### 6.7 评审评论前缀

| 前缀 | 含义 |
|------|------|
| `MUST:` | 必须修改才能合并 |
| `SHOULD:` | 强烈建议修改 |
| `NIT:` | 小问题，可不改 |
| `Q:` | 提问 |
| `PRAISE:` | 表扬 |

---

## 7. 豁免机制

### 7.1 适用

- SAST 高危误报
- 漏洞（短期无法升级）
- 强制规则违反（特批）
- 评审要求（紧急 PR）

### 7.2 流程

```
发现需豁免 → 填写豁免单 → 评审（架构师/QA）→ 批准/拒绝 → 记录 → 实施
```

### 7.3 豁免单（GitHub Issue 模板）

```markdown
## 豁免类型
- [ ] SAST 警告
- [ ] SCA 漏洞
- [ ] 强制规则违反
- [ ] 其他

## 详情
- 工具 / 规则 ID：
- 文件 / 行号：
- 警告内容：

## 原因
<!-- 为什么需要豁免 -->

## 影响
<!-- 不豁免的影响 / 豁免的风险 -->

## 缓解
<!-- 临时缓解措施 -->

## 关闭条件
<!-- 何时正式修复 -->

## 评审
- 架构师：@xxx 同意/拒绝
- QA：@xxx 同意/拒绝
- 决策日：
```

### 7.4 豁免登记册

每月汇总，纳入 QA 月报。

---

## 8. 评审会议

### 8.1 类型

| 会议 | 时点 | 范围 | 主持 |
|------|------|------|------|
| **RD Review** | 要件定义末 | 10-21 全部 | 架构师 |
| **BD Review** | 基本设计末 | 22-41 全部 | 架构师 |
| **DD Review** | 详细设计末 | 42-52 全部 | 架构师 |
| **CR Review** | PR 时持续 | 全部 PR | 评审者 |
| **ST Gate** | ST 完成 | 76-89 全部 | QA |
| **UAT Gate** | UAT 完成 | 90-95 全部 | QA + 客户 |
| **Go-Live** | 发布前 | 102-108 | PM + Sponsor |

### 8.2 评审会议通用流程

```
T-7d   发出评审材料
T-3d   评审者预读
T-1d   提交问题清单
T-0    会议（3h）
        - 0:00-0:10  开场
        - 0:10-2:30  逐项过
        - 2:30-2:50  决议
        - 2:50-3:00  下一步
T+5d   关闭所有问题
```

### 8.3 评审产出

- 评审纪要
- 决议（通过 / 附条件 / 不通过）
- 待办清单（带责任人 + 关闭日）

### 8.4 已建评审纪要

| 评审 | 文档 | 状态 |
|------|------|------|
| RD Review | `CATs_要件承認決議書 v1.0` | ✅ |
| BD Review | `CATs_BD 评审纪要 v1.0` | ✅ |
| DD Review | `CATs_DD 评审纪要 v1.0` | ⚪ P2 待补 |
| ST Gate | `CATs_系统测试报告 v1.0` | ⚪ M3 待补 |
| UAT Gate | `CATs_UAT 报告 v1.0` | ⚪ M3 待补 |

---

## 9. 度量

### 9.1 PR 指标

| 指标 | 目标 |
|------|------|
| PR 首次响应时间 | < 4h |
| PR 完成时间 | < 1d |
| PR 合并率 | > 80% |
| 评审轮次 | < 3 |
| PR 规模 | < 400 行 |

### 9.2 CR 指标

| 指标 | 目标 |
|------|------|
| 评审参与率 | 100%（PR 必有评审） |
| 评论密度 | 5-15 / PR |
| Must 修改率 | > 60%（说明问题被发现） |

### 9.3 SAST 指标

| 指标 | 目标 |
|------|------|
| 高危新增 | 0 |
| 中危新增 | < 5 / 周 |
| Lint 警告 | < 10 / 服务 |
| 漏洞修复 SLA | 100% |

### 9.4 月报

每月汇总，纳入 QA 月报 + Sponsor 报告。

---

## 10. 工具栈

| 工具 | 用途 | 部署 |
|------|------|------|
| GitHub | Git + PR + CODEOWNERS | SaaS |
| GitHub Actions | CI | SaaS + Self-hosted |
| SonarQube | SAST / 代码质量 | Self-hosted |
| Semgrep | SAST | SaaS / CLI |
| Snyk | SCA | SaaS |
| Trivy | 容器 + IaC + 漏洞 | CLI / CI |
| Gitleaks | Secret 扫描 | CI + pre-commit |
| Dependabot | 依赖更新 | GitHub |

---

## 11. 集成 CI

详见 `CATs_CI_CD_構築運用手順書 v1.0` §7。

---

## 12. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 55 静的解析 | 本文 §4 |
| 56 コードレビュー | 本文 §6 |
| 127 品質計画 | 顶层 |
| 128 品質レビュー | §8 评审 |
| 20 要件レビュー | §8 RD |
| 41 基本設計レビュー | §8 BD |
| 52 詳細設計レビュー | §8 DD |
| QA-101 跨团队契约 | CODEOWNERS |

---

## 13. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_CI_CD 構築運用手順書 v1.0 | `05-其他\治理\` |
| CATs_開発者ガイド v1.0 | `05-其他\治理\` |
| CATs_要件承認決議書 v1.0 | `05-其他\评审记录\` |
| CATs_BD 评审纪要 v1.0 | `05-其他\评审记录\` |
| CATs_Rust 技术选型书 v1.0 | `02-基础设计\技术选型\` |
| CATs_安全要件定义书 v1.0 §9 | `05-其他\安全\` |
| CATs_测试设计书 v1.0 §10 | `04-测试\测试设计书\` |

---

## 14. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | CODEOWNERS 完善 | 架构师 | M1-S0 |
| OI-2 | SonarQube 部署 | SRE | M1-S0 |
| OI-3 | pre-commit hooks 模板 | Rust Lead | M1-S0 |
| OI-4 | Semgrep 规则集 | QA | M1-S0 |
| OI-5 | 豁免登记册建立 | QA | M1-S0 |
| OI-6 | 月报模板 | QA | M1-S0 |

---

**文档结束**
