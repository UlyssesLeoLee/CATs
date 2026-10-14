# CATs DB 账号注入 runbook v0.1

**版本**: v0.1
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008)

---

## §1 角色矩阵 (per DB 设计 v2.0 §2)

| 逻辑库 | 运行时角色 (CRUD) | 迁移角色 (DDL) | 只读角色 |
|---|---|---|---|
| auth_db | `svc_auth` | `migrator_auth` | — |
| user_db | `svc_user` | `migrator_user` | — |
| project_db | `svc_project` + `svc_translation` | `migrator_project` | — |
| task_db | `svc_task` | `migrator_task` | `svc_report_ro` |
| file_db | `svc_file` | `migrator_file` | — |
| notification_db | `svc_notify` | `migrator_notify` | — |
| report_db | `svc_report` | `migrator_report` | — |
| audit_db | `svc_audit` | `migrator_audit` | — |

**原则**: 运行时角色不具备 DDL 权限(per 架构 v1.0 §5.2)。

## §2 创建角色 DDL (per 逻辑库)

```sql
-- per 逻辑库
CREATE ROLE migrator_<svc> WITH LOGIN CREATEDB;
CREATE ROLE svc_*;

-- 项目权限
GRANT CONNECT ON DATABASE <db> TO svc_*;
GRANT USAGE ON SCHEMA public TO svc_*;
GRANT SELECT, INSERT, UPDATE, DELETE ON ALL TABLES IN SCHEMA public TO svc_*;
GRANT USAGE, SELECT ON ALL SEQUENCES IN SCHEMA public TO svc_*;

-- 默认权限 (新表自动授权)
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES TO svc_*;
ALTER DEFAULT PRIVILEGES IN SCHEMA public
  GRANT USAGE, SELECT ON SEQUENCES TO svc_*;
```

## §3 Secret 注入 (K8s + Sealed Secrets)

### 3.1 Sealed Secrets 流程

```bash
# 1. 安装 sealed-secrets controller (per 架构 v1.0 §14)
helm repo add sealed-secrets https://bitnami-labs.github.io/sealed-secrets
helm install sealed-secrets sealed-secrets/sealed-secrets -n kube-system

# 2. 用 kubeseal 加密 secret (offline 模式)
echo -n 'supersecret' | kubeseal --raw \
  --from-file=/dev/stdin \
  --namespace cats-core \
  --name cats-db-credentials \
  --controller-name=sealed-secrets \
  > sealed-secret.yaml

# 3. 部署到 K8s
kubectl apply -f sealed-secret.yaml
```

### 3.2 External Secrets (Vault / AWS Secrets Manager)

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: cats-db-credentials
  namespace: cats-core
spec:
  secretStoreRef:
    name: vault-backend
    kind: ClusterSecretStore
  target:
    name: cats-db-credentials
  data:
    - secretKey: auth-db-url
      remoteRef:
        key: cats/postgres
        property: auth-url
```

### 3.3 占位 fallback (MVP)

MVP 阶段用 `deploy/k3s/secrets/secrets-placeholder.yaml`(per Sprint 2 W3-W4 SRE 落地)即可,实际值填 `PLACEHOLDER`。**生产环境必须替换**。

## §4 密码轮换策略

| Secret | 轮换周期 | 来源 |
|---|---|---|
| JWT secret | 90 天 | per ADR-008 JWT 密钥轮换策略 |
| DB password | 180 天 | per 架构 v1.0 §14 |
| API keys (OpenAI 等) | 365 天 或 触发式 | per vendor 政策 |

## §5 已知缺口

- ❌ 真实 secret 注入 (MVP 用占位, Sprint 末续做)
- ❌ Vault 后端配置 (Sprint 3)
- ❌ 密码轮换自动化 (Sprint 3)

## §6 签批

| 角色 | 姓名 | 签字日 | 结论 |
|---|---|---|---|
| DBA | 架构师(Mavis 接手 agent per DEC-008) | 2026-09-19 | ✅ Runbook 就绪, MVP 走占位,Sprint 3 接真实 Vault |

> 永久代签 per 守门 #14 v3 + 9/8 15:19 强化。