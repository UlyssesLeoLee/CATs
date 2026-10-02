# CATs MVP 本地快速起停 RUNBOOK

**版本**: v1.0
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008)
**状态**: 🟢 MVP 商业版就绪

---

## 1. 准备 (一次性, 5 分钟)

### 1.1 前置依赖
- Docker Desktop + WSL2 (Windows) **或** Docker (Linux/macOS)
- kubectl + k3s (K8s 集群)
- helm 3.x (可选)
- kustomize 5.x

### 1.2 拉镜像
```bash
# MVP 阶段 image 用本地构建, imagePullPolicy=Never, 所以镜像需要手动 build
# (CI 推到 harbor 留给 Sprint 3)

# 在 monorepo 根目录:
cargo build --release -p auth-service -p user-service -p project-service -p task-service
cargo build --release -p file-service -p notification-service -p report-service
cargo build --release -p audit-service -p worker-service -p translation-core
cargo build --release -p cats-ai-gateway -p cats-bff

# 或一次过 (慢但稳):
cargo build --release --workspace

# 各 service binary 在 target/release/<service-name>
```

### 1.3 准备 secrets
```bash
# 占位 secrets 已经部署 (per deploy/k3s/secrets/secrets-placeholder.yaml)
# 真实密码走 sealed-secrets 或 external-secrets, 见 sprint 末续做
```

## 2. 起 (一键, 30 秒 - 2 分钟)

### 2.1 启 K3s 集群 (k3d 本地模式)
```bash
k3d cluster create cats-mvp --servers 1 --agents 2 --port "8080:80@loadbalancer"
```

### 2.2 部署 namespace + 边缘层 + PG + Kafka
```bash
kubectl apply -f deploy/k3s/namespaces.yaml
kubectl apply -f deploy/k3s/postgres/
kubectl apply -f deploy/k3s/kafka/cats-kafka-mvp.yaml
kubectl apply -f deploy/k3s/kafka/cats-kafka-topics-mvp.yaml
kubectl apply -f deploy/k3s/secrets/secrets-placeholder.yaml
```

### 2.3 启 cats-core 11 服务
```bash
kubectl apply -k deploy/k3s/cats-core/
```

### 2.4 启 cats-edge envoy
```bash
kubectl apply -f deploy/k3s/cats-edge/envoy-deployment.yaml
```

### 2.5 启 monitoring (prometheus + alertmanager + grafana)
```bash
kubectl apply -f monitoring/prometheus-scrape.yaml
kubectl apply -f monitoring/alertmanager-rules.yaml
# grafana dashboard 通过 ConfigMap 导入
```

### 2.6 启 GitOps (可选)
```bash
kubectl apply -f deploy/argocd/cats-mvp-application.yaml
```

## 3. 验证 (3 分钟)

### 3.1 Pod ready 状态
```bash
kubectl get pods -A -l 'app.kubernetes.io/part-of=cats'
# 期望: 全部 Running, 11/11 cats-core + 2 envoy + 1 postgres + 1 kafka + monitoring 全部 OK
```

### 3.2 Health checks
```bash
# 找 LoadBalancer IP (envoy)
LB_IP=$(kubectl get svc envoy-edge -n cats-edge -o jsonpath='{.status.loadBalancer.ingress[0].ip}')
curl http://$LB_IP/healthz
# 期望: ok

# auth-service (内网)
kubectl port-forward svc/auth-service -n cats-core 8080:8080 &
curl http://localhost:8080/healthz
# 期望: {"status":"ok","service":"auth-service"}
```

### 3.3 端到端 smoke
```bash
# login
curl -X POST http://localhost:8080/v1/auth/login -H 'Content-Type: application/json' \
  -d '{"email":"demo@cats.local","password":"demo123"}'
# 期望: {"access_token":"...","refresh_token":"..."}

# 创建项目 (带 JWT)
TOKEN="..."
curl -X POST http://localhost:8080/v1/projects -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"name":"Demo Project","org_id":"..."}'
# 期望: {"id":"...","name":"Demo Project",...}

# 翻译一段 (走 TM 命中)
curl -X POST http://localhost:8080/v1/translate/lookup -H "Authorization: Bearer $TOKEN" \
  -H 'Content-Type: application/json' -d '{"source_text":"Hello, world!","source_lang":"en-US","target_lang":"zh-CN"}'
# 期望: 1 条 100% 命中 (mock TM)
```

### 3.4 跑 mock smoke 脚本 (per cats-mock 收纳约定)
```bash
bash crates/cats-mock/scripts/smoke-test.sh
```

## 4. 停

```bash
# 干净停
k3d cluster delete cats-mvp

# 或者保留集群, 只停 cats 应用
kubectl delete -k deploy/k3s/cats-core/
kubectl delete -f deploy/k3s/cats-edge/envoy-deployment.yaml
```

## 5. 常见问题

| 问题 | 排查 |
|---|---|
| Pod Pending | `kubectl describe pod <pod> -n cats-core`, 通常 PVC 或资源不足 |
| CrashLoopBackOff | `kubectl logs <pod>`, 通常 env 没设或 DB 连不上 |
| envoy 502 | 检查 upstream service 的 service name + namespace 是否匹配 |
| JWT 401 | 检查 cats-jwt-secret 是否 deploy, auth-service 是否能读 |
| 5xx burst | 看 alertmanager rules 是否触发, 查 monitoring/prometheus |

## 6. Sprint 末续做清单 (per Sprint 2 末 W3+W4 落地)

- [ ] 真实镜像 build + push (CI 接 harbor)
- [ ] sealed-secrets 实际 secret 注入 (替换 placeholder)
- [ ] AI 网关接真 provider (OpenAI/Anthropic)
- [ ] translation-core 接真 project_db
- [ ] DDD Review 6 角色复审 (本轮 30 commit + 9 commit final)
- [ ] UI/UX 4 原则 落地
- [ ] 真卡内 mTLS + NetworkPolicy

---

**承认现状**:MVP 商业版 95% 就绪, 真实流量/真 provider/真 mTLS 留 Sprint 3。