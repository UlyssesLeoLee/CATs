# cats-bff — BFF 聚合服务 (M1 阶段)

> CATs 客户端 → 后端的桥接层 (per 微服务架构书 v1.0 §4.1)

## 8 业务 endpoint (per OpenAPI v1.0.1 §paths)

| Method | Path | 上游 | 鉴权 | RBAC | 备注 |
|---|---|---|---|---|---|
| POST | `/v1/auth/login` | auth-service | — | — | 转发 + 注入 JWT |
| POST | `/v1/auth/refresh` | auth-service | — | — | 转发 |
| POST | `/v1/auth/logout` | auth-service | Bearer | — | 转发 |
| GET  | `/v1/auth/me` | auth-service | Bearer | — | 聚合 (per §3) |
| GET  | `/v1/projects` | project-service | Bearer | Read | 转发 + RBAC |
| POST | `/v1/projects` | project-service | Bearer | Create | 转发 + RBAC + Idempotency-Key |
| POST | `/v1/tasks` | task-service | Bearer | Create | 转发 + RBAC |
| GET  | `/healthz` | 本地 | — | — | ✅ 已有 |

## 配置 (env)

```
SERVICE_BIND_ADDR   default 0.0.0.0:8097
AUTH_SERVICE_URL    default http://localhost:8081
USER_SERVICE_URL    default http://localhost:8082
PROJECT_SERVICE_URL default http://localhost:8083
TASK_SERVICE_URL    default http://localhost:8084
TRANSLATION_CORE_URL default http://localhost:8086
UPSTREAM_TIMEOUT_SECS default 5
```

## 错误信封 (per 接口设计书 v2.0 §1.3 + OpenAPI v1.0.1 ErrorBody schema)

```json
{
  "error": "missing_authorization",
  "message": "authentication required",
  "detail": null
}
```

3 字段 (error + message + detail), 跟 `crates/auth-service/src/models.rs` ErrorBody 一致。

## 引用

- 父: ULYS-125 (功能点罗列)
- 切片 spec: `doc/05-其他/MVP商业版/_slice_a_bff.md`
- OpenAPI: `api/openapi/cats-openapi-v1.0.1.yaml`
- 接口设计: `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md`
- 微服务架构: `doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md` §4.1
- 权限矩阵: `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` §3
- 错误码表: `doc/05-其他/管理/CATs_错误码表_v1.0.md` §3 (28 条 snake_case)

## 作者署名

架构师 (Mavis 接手 agent per DEC-008 + 守门 #14 v3) <architecture@mavis.local>
