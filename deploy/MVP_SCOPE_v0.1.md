# CATs MVP SRE 落地范围 / 已知缺口 / Sprint 续做清单

**版本**: v0.1
**日期**: 2026-09-19
**作者**: 架构师(Mavis 接手 agent per DEC-008) per 8/27 19:39 守门
**状态**: 🟡 MVP W3-W4 部分落地,Sprint 2 末续做清单

---

## §0 元信息

### §0.1 本次实际落地(MVP SRE worktree `feat/mvp-sre` 已 commit)

| commit | 类型 | 内容 |
|---|---|---|
| `db173ab` | deploy(k3s) | namespaces + quotas + limitranges(cats-core/cats-edge/cats-data/cats-observe/cats-mock) |
| `b6548cd` | deploy(k3s) | postgres + pgvector(CloudNativePG 1 instance MVP) |
| 接续 commit | deploy(k3s) | kafka MVP 1-broker KRaft + 10 topics |

### §0.2 本次未能落地(原 10 项里 4-7 未做,worker 断网导致)

- ❌ **cats-core 9 services deployment**(auth/user/project/task/file/notification/report/audit/worker + translation-core)
- ❌ **cats-edge envoy 独立 deployment**(per 9/1 13:05 JST 用户偏好)
- ❌ **secrets 占位**(JWT/DB/AI keys 4 个)
- ❌ **monitoring**(prometheus + alertmanager 4 rules + grafana dashboard)
- ❌ **argocd Application CRD**
- ❌ **CI kubeconform check**(root session 接力完成)
- ❌ **MVP RUNBOOK**

---

## §1 已知缺口(MUST FIX 才能 MVP 上线)

| # | 缺口 | 优先级 | Sprint 2 末续做责任域 |
|---|---|---|---|
| 1 | cats-core 9 service deployment YAML 缺 | 🔴 P0 | SRE Lead(worker#5 续做 or root 接) |
| 2 | envoy edge 独立 deployment + EnvoyConfig 缺 | 🔴 P0 | SRE Lead |
| 3 | monitoring 4 alertmanager rules 缺(per 1d8926d) | 🔴 P0 | SRE Lead |
| 4 | secrets 占位 + 工作流(sealed-secrets / external-secrets) | 🟡 P1 | SRE Lead + 平台 Lead |
| 5 | 真实 CI 上跑 kubeconform gate | 🟡 P1 | SRE Lead + PM Lead |

---

## §2 9/1 用户偏好落实

- ✅ **所有 nginx 替换为 envoy**(9/1 13:03 JST)— 本次为 MVP 写 cats-edge/envoy-gateway.yaml 留下的"独立 deployment"路径,**不**用 istio sidecar(per 9/1 13:05 JST)
- ✅ **envoy 独立 deployment 模式**(9/1 13:05 JST)— Sprint 2 末续做时确认

---

## §3 全部 Sprint 2 末续做清单(per Sprint 2 节奏)

- [ ] cats-core/9 services deployment
- [ ] cats-edge/envoy-gateway(deployment 模式)
- [ ] secrets 占位 4 份
- [ ] monitoring/prometheus + alertmanager 4 rules(per 1d8926d 草稿)
- [ ] monitoring/grafana MVP dashboard
- [ ] argocd ApplicationSet(v2 阶段二)
- [ ] ci kubeconform 在真实 CI 跑通
- [ ] MVP_RUNBOOK.md(local-up / local-down)
- [ ] DDD Review 6 角色复审

---

**承认现状**:本次 Sprint 2 W3-W4 部分落地,但根 9/8 15:29 JST 第 7 次强化(Mavis 自驱不被动等指令),root session 已主动接力已落地的实物,**未落地的列入续做清单,不假装完成**。
