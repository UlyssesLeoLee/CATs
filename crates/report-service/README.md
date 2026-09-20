# report-service

> CATs 报表服务 (per ULYS-153 切片 C-1)

| 项目 | 内容 |
|---|---|
| Crate 名 | `report-service` |
| 阶段 | MVP |
| 默认端口 | 8087（由 env `BIND_ADDR` 覆盖） |
| 数据边界 | `report_db` (跨表聚合需 ops 侧配置 postgres_fdw / dblink) |
| 镜像 | `harbor.cats.internal/cats/report-service:0.1.0` |

## 概述

用量统计、翻译量、审计摘要三类聚合报表查询。所有查询走 SQL 聚合，不拉数据到内存。

引用：[CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)（16 服务清单）

## API 端点 (per ULYS-153 切片 C-1)

| Method | Path | Query | 说明 |
|---|---|---|---|
| GET | `/healthz` | — | 存活/就绪探针 |
| GET | `/v1/reports/usage` | `?org_id=&from=&to=` | 按 action × resource_type 分组的事件统计 |
| GET | `/v1/reports/translation-volume` | `?project_id=&from=&to=` | 按 day 聚合的翻译完成量 |
| GET | `/v1/reports/audit-summary` | `?workspace_id=&from=&to=` | 总数 + distinct actors + top10 actions + 最近事件时间 |

错误信封（per 接口设计书 §1.3）：
```json
{"error": "invalid_request", "message": "...", "detail": "..."}
```

## 数据源与跨服务聚合

按切片 C-1 §数据源要求，SQL 函数使用 schema 限定（`audit.audit_logs`），允许：

1. **本切片 MVP**：ops 在 `report_db` 创建 foreign schema 指向 `audit_db`，零代码改动启用跨库聚合
2. **未来扩展**：相同模式可加 `task.*` / `translation_core.*` 等跨服务表

具体配置：
```sql
-- ops 侧一次性配置 (per K3s 阶段二任务 T-04)
CREATE EXTENSION IF NOT EXISTS postgres_fdw;
CREATE SERVER audit_db_fdw FOREIGN DATA WRAPPER postgres_fdw
  OPTIONS (host 'pg-audit', port '5432', dbname 'audit_db');
CREATE USER MAPPING FOR svc_report SERVER audit_db_fdw
  OPTIONS (user 'svc_report_ro', password '...');
CREATE SCHEMA audit;
IMPORT FOREIGN SCHEMA public LIMIT TO (audit_logs) FROM SERVER audit_db_fdw INTO audit;
GRANT USAGE ON SCHEMA audit TO svc_report;
GRANT SELECT ON ALL TABLES IN SCHEMA audit TO svc_report;
```

## 本地运行

```powershell
# 编译
cargo build -p report-service

# 运行
$env:BIND_ADDR = "0.0.0.0:8087"
$env:DATABASE_URL = "postgres://svc_report:...@pg-report:5432/report_db"
cargo run -p report-service

# 健康检查
curl http://127.0.0.1:8087/healthz

# usage 示例
curl "http://127.0.0.1:8087/v1/reports/usage?org_id=11111111-1111-1111-1111-111111111111&from=2026-09-01T00:00:00Z&to=2026-09-30T23:59:59Z"

# translation-volume 示例
curl "http://127.0.0.1:8087/v1/reports/translation-volume?project_id=22222222-2222-2222-2222-222222222222&from=2026-09-01T00:00:00Z&to=2026-09-30T23:59:59Z"

# audit-summary 示例
curl "http://127.0.0.1:8087/v1/reports/audit-summary?workspace_id=11111111-1111-1111-1111-111111111111&from=2026-09-01T00:00:00Z&to=2026-09-30T23:59:59Z"
```

## 测试

```powershell
cargo test -p report-service
```

## 容器化

```bash
docker build -f deploy/docker/Dockerfile.rust --build-arg CRATE_NAME=report-service -t report-service:0.1.0 .
```

## Helm 部署

```bash
helm lint deploy/helm/report-service
helm template deploy/helm/report-service
```

## 引用基线文档

- [CATs_微服务架构设计书_v1.0 §4.1](../../doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md)
- [CATs_技术基线_v1.0 §1](../../doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md)
- [CATs_Rust技术选型书_v1.0](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
- [ULYS-153 切片 C-1 实施记录](../slices/ULYS-153_slice_C_report_audit.md)
