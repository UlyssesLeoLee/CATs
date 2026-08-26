# CATs ADR-005：认证、授权与多租户隔离

> **文档编号**：CATs-ADR-005
> **版本**：v1.0
> **创建日**：2026-08-26
> **作者**：架构师 + 安全负责人
> **状态**：已接受
> **取代**：—

---

## 1. 背景

CATs 是多租户 SaaS（企业 / 团队 / 个人三层），需要：

- 身份认证（SSO + 密码 + MFA）
- 细粒度授权（RBAC + ABAC 混合）
- 租户隔离（数据 + 计算 + 网络）
- 合规（GDPR + 日本 APPI + 行业 NDA）

## 2. 选项

### 2.1 身份认证

| 选项 | 优势 | 劣势 | 决策 |
|------|------|------|------|
| 自建（Rust + JWT） | 可控、无外部依赖 | 重复造轮子、合规自证成本 | 否决 |
| **Keycloak** ✅ | 开源、OIDC/SAML/MFA 开箱即用、社区版成熟 | 需运维 | 采纳 |
| Auth0 / Okta | 托管、省心 | 成本高、数据出境、定制受限 | 暂不（评估中） |

### 2.2 授权模型

| 选项 | 决策理由 |
|------|----------|
| **RBAC（角色）+ ABAC（属性）混合** ✅ | 项目级角色 + 资源属性条件（如 NDA 等级 / 项目保密级别）|

角色矩阵（已在 33 任务占位，本 ADR 锁定）：
- 平台角色：super-admin / support / billing
- 租户角色：tenant-admin / project-manager / linguist / reviewer / guest
- 资源属性：project.security_level / document.nda / segment.lock_state

### 2.3 多租户隔离

| 策略 | 描述 | 决策 |
|------|------|------|
| 共享库 + 共享 DB + 租户列 | 成本最低、风险最高 | ❌ |
| 共享 DB + 独立 Schema | 中等成本、中等隔离 | ✅ **PG schema 隔离** |
| 每租户独立 DB | 高隔离、高成本 | 留给 enterprise tier（>500 seats） |
| 每租户独立 namespace（K8s） | 计算隔离 | ✅ 配合 PG schema 双重隔离 |

## 3. 决策

```
                    ┌──────────────┐
                    │   Keycloak   │  OIDC Provider
                    └──────┬───────┘
                           │ JWT (RS256)
              ┌────────────┴────────────┐
              ▼                         ▼
     ┌────────────────┐        ┌────────────────┐
     │  gateway-bff   │        │  admin 服务    │
     │ (Token 校验)   │        │ (Token 校验)   │
     └────────┬───────┘        └────────┬───────┘
              │ gRPC + tenant_id        │
              ▼                         ▼
     ┌────────────────────────────────────────┐
     │       应用服务（按 tenant_id 路由）      │
     │  + PG schema 隔离 + ABAC 中间件         │
     └────────────────────────────────────────┘
```

**关键约束**：

1. 每个请求必须携带 `tenant_id` + `user_id` + `project_id`（如有）
2. 任何 SQL 都必须经 tenant 中间件注入 `WHERE tenant_id = $1`
3. 跨租户调用禁止（除 super-admin 显式操作，且记录审计）
4. K8s 命名空间按 tier 划分（free/pro/enterprise）

## 4. 影响

- **正面**：
  - Keycloak 减重复造轮；OIDC 标准便于未来接 IdP
  - schema 隔离 + namespace 隔离双重保险
- **负面**：
  - Keycloak 集群需 SRE 运维（已纳入 109-117 运维）
  - tenant 中间件是性能热点（需用 Async trait + 上下文传播）
- **风险**：
  - 跨租户数据泄漏是 P0 安全风险 → 强制 LINT（rust-qs）
  - GDPR 数据导出/删除 → 已在 134 课题表登记

## 5. 关联

- 上游：CATs_安全要件定义书_v1.0
- 下游：CATs_权限矩阵（待整合）、L-8 RBAC+ABAC 实现（DD 评审 §4.2）
- 阻塞项：QA-008（GDPR 合规清单）
