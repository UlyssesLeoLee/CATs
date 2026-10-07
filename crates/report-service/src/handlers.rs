//! HTTP handlers (report-service) — per ULYS-153 切片 C-1
//!
//! 端点:
//! - GET /healthz                                  — 健康检查
//! - GET /v1/reports/usage?org_id=&from=&to=       — 用量统计
//! - GET /v1/reports/translation-volume?project_id=&from=&to= — 翻译量
//! - GET /v1/reports/audit-summary?workspace_id=&from=&to=   — 审计摘要
//!
//! 错误信封: per 接口设计书 §1.3 — `{error, message, detail?}`

use crate::db;
use crate::models::{
    AuditSummaryResponse, ErrorBody, TranslationVolumeResponse, UsageReportItem,
    UsageReportResponse,
};
use crate::rbac;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use cats_rbac::RbacChecker;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub app: cats_common::AppMeta,
}

pub async fn healthz() -> impl Responder {
    healthz_response()
}

/// `healthz_response` 工厂 (per lib re-export 约定)
///
/// 形状 per BACKEND_STATUS §4.1m 统一后的全仓唯一形状：
/// `{"status":"ok","app":{"name":...,"version":...}}`
pub fn healthz_response() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: cats_common::AppMeta {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

/// `GET /readyz` — 就绪探针，**必须**在 DB 不可用时返回 503
///
/// 2026-10-07 新增。此前 k3s 的 `deploy/k3s/cats-core/report-service.yaml` 把
/// readinessProbe 指向了 `/healthz`，而 `/healthz` 恒返 200 且不查任何依赖 ——
/// 数据库挂了 Pod 照样 Ready，流量继续被派发进来，而每个请求都 500。
///
/// 三个报表 endpoint 全部走 report_db 聚合查询，DB 不可用时一个都跑不了，
/// 所以就绪只看数据库这一项。
pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    let db_ok = sqlx::query("SELECT 1")
        .execute(pool.get_ref())
        .await
        .is_ok();
    let body = cats_common::ReadyResponse::new(env!("CARGO_PKG_NAME"), db_ok);
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(body.status_code())
            .expect("status_code 只返回 200/503，都是合法状态码"),
    )
    .json(body)
}

/// **唯一的路由表** —— `main.rs` 与集成测试共用。
///
/// 2026-10-07 抽出。此前 4 条路由直接内联在 `main.rs` 的 `HttpServer::new`
/// 里，而 `tests/` 下只有 `smoke.rs`，于是"服务对外暴露什么"这件事**没有任何
/// 测试在验证**。共用这一个函数之后，集成测试挂的就是生产实际提供的路由。
///
/// `app_data` 也在这里注册（而不是留在调用方），否则"注册哪些共享状态"
/// 同样会变成调用方与测试各写一份。
///
/// 调用方注意：`HttpServer::new` 的闭包是 `Fn`，每个 worker 线程都会再调一次，
/// 闭包里要把两个 `web::Data` **先 clone 出来再 move 进去**。
pub fn configure_routes(
    cfg: &mut web::ServiceConfig,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
) {
    cfg.app_data(pool)
        .app_data(rbac_checker)
        .route("/healthz", web::get().to(healthz))
        .route("/readyz", web::get().to(readyz))
        // GET /v1/reports/usage — 用量统计（RBAC: Report Read）
        .route("/v1/reports/usage", web::get().to(usage_report))
        // GET /v1/reports/translation-volume — 翻译量（RBAC: Report Read）
        .route(
            "/v1/reports/translation-volume",
            web::get().to(translation_volume),
        )
        // GET /v1/reports/audit-summary — 审计摘要（RBAC: Report Read）
        .route("/v1/reports/audit-summary", web::get().to(audit_summary));
}

// =====================================================================
// 1. /v1/reports/usage
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    pub org_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// `GET /v1/reports/usage` (RBAC: Report Read)
pub async fn usage_report(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    q: web::Query<UsageQuery>,
) -> impl Responder {
    if let Err((status, body)) =
        rbac::enforce(rbac_checker.get_ref(), &req, "/v1/reports/usage", "GET").await
    {
        return HttpResponse::build(status).json(body);
    }
    if q.from >= q.to {
        return HttpResponse::BadRequest().json(ErrorBody::new(
            "invalid_request",
            "from must be strictly before to",
        ));
    }
    let items = match db::usage_by_org(pool.get_ref(), q.org_id, q.from, q.to).await {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ErrorBody::new("server_error", "usage query failed").with_detail(e));
        }
    };
    let total = match db::usage_total(pool.get_ref(), q.org_id, q.from, q.to).await {
        Ok(t) => t,
        Err(e) => {
            return HttpResponse::InternalServerError()
                .json(ErrorBody::new("server_error", "usage total query failed").with_detail(e));
        }
    };
    let items: Vec<UsageReportItem> = items
        .into_iter()
        .map(|r| UsageReportItem {
            action: r.action,
            resource_type: r.resource_type,
            event_count: r.event_count,
            distinct_actors: r.distinct_actors.unwrap_or(0),
        })
        .collect();
    HttpResponse::Ok().json(UsageReportResponse {
        org_id: q.org_id,
        from: q.from,
        to: q.to,
        total_events: total,
        items,
    })
}

// =====================================================================
// 2. /v1/reports/translation-volume
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct TranslationVolumeQuery {
    pub project_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// `GET /v1/reports/translation-volume` (RBAC: Report Read)
pub async fn translation_volume(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    q: web::Query<TranslationVolumeQuery>,
) -> impl Responder {
    if let Err((status, body)) = rbac::enforce(
        rbac_checker.get_ref(),
        &req,
        "/v1/reports/translation-volume",
        "GET",
    )
    .await
    {
        return HttpResponse::build(status).json(body);
    }
    if q.from >= q.to {
        return HttpResponse::BadRequest().json(ErrorBody::new(
            "invalid_request",
            "from must be strictly before to",
        ));
    }
    let daily = match db::translation_volume_by_project(pool.get_ref(), q.project_id, q.from, q.to)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(
                ErrorBody::new("server_error", "translation_volume query failed").with_detail(e),
            );
        }
    };
    let (total_completed, total_chars) =
        match db::translation_volume_total(pool.get_ref(), q.project_id, q.from, q.to).await {
            Ok(t) => t,
            Err(e) => {
                return HttpResponse::InternalServerError().json(
                    ErrorBody::new("server_error", "translation_volume total query failed")
                        .with_detail(e),
                );
            }
        };
    HttpResponse::Ok().json(TranslationVolumeResponse {
        project_id: q.project_id,
        from: q.from,
        to: q.to,
        total_completed,
        total_chars,
        daily,
    })
}

// =====================================================================
// 3. /v1/reports/audit-summary
// =====================================================================

#[derive(Debug, Deserialize)]
pub struct AuditSummaryQuery {
    pub workspace_id: Uuid,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
}

/// `GET /v1/reports/audit-summary` (RBAC: Report Read)
pub async fn audit_summary(
    req: HttpRequest,
    pool: web::Data<PgPool>,
    rbac_checker: web::Data<Arc<RbacChecker>>,
    q: web::Query<AuditSummaryQuery>,
) -> impl Responder {
    if let Err((status, body)) = rbac::enforce(
        rbac_checker.get_ref(),
        &req,
        "/v1/reports/audit-summary",
        "GET",
    )
    .await
    {
        return HttpResponse::build(status).json(body);
    }
    if q.from >= q.to {
        return HttpResponse::BadRequest().json(ErrorBody::new(
            "invalid_request",
            "from must be strictly before to",
        ));
    }
    let (total_events, distinct_actors, last_event_at, top_actions) =
        match db::audit_summary_by_workspace(pool.get_ref(), q.workspace_id, q.from, q.to).await {
            Ok(t) => t,
            Err(e) => {
                return HttpResponse::InternalServerError().json(
                    ErrorBody::new("server_error", "audit_summary query failed").with_detail(e),
                );
            }
        };
    HttpResponse::Ok().json(AuditSummaryResponse {
        workspace_id: q.workspace_id,
        from: q.from,
        to: q.to,
        total_events,
        distinct_actors,
        top_actions,
        last_event_at,
    })
}

// =====================================================================
// 4. 单元测试 (不依赖 DB, 验证 query 反序列化 + 边界条件)
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn usage_query_construction() {
        // 验证 Query DTO 构造正确 (URL 反序列化由 actix-web Query extractor 保证)
        let q = UsageQuery {
            org_id: Uuid::parse_str("11111111-1111-1111-1111-111111111111").unwrap(),
            from: Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
            to: Utc.with_ymd_and_hms(2026, 9, 30, 23, 59, 59).unwrap(),
        };
        assert_eq!(q.from.to_rfc3339(), "2026-09-01T00:00:00+00:00");
        assert!(q.from < q.to);
    }

    #[test]
    fn translation_volume_query_deserialize() {
        let q = TranslationVolumeQuery {
            project_id: Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap(),
            from: Utc.with_ymd_and_hms(2026, 9, 1, 0, 0, 0).unwrap(),
            to: Utc.with_ymd_and_hms(2026, 9, 30, 23, 59, 59).unwrap(),
        };
        assert_ne!(q.project_id, Uuid::nil());
    }

    #[test]
    fn error_body_serializes_without_detail() {
        let body = ErrorBody::new("invalid_request", "from must be < to");
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("invalid_request"));
        assert!(
            !json.contains("detail"),
            "detail should be skipped when None"
        );
    }

    #[test]
    fn error_body_serializes_with_detail() {
        let body = ErrorBody::new("server_error", "query failed").with_detail("timeout");
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("timeout"));
    }
}
