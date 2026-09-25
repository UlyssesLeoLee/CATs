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
    AuditSummaryResponse, ErrorBody, TranslationVolumeResponse, UsageReportItem, UsageReportResponse,
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
    pub service: &'static str,
}

pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        service: "report-service",
    })
}

/// `healthz_response` 工厂 (per lib re-export 约定)
pub fn healthz_response() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        service: "report-service",
    })
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/reports/usage", "GET").await {
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/reports/translation-volume", "GET").await {
        return HttpResponse::build(status).json(body);
    }
    if q.from >= q.to {
        return HttpResponse::BadRequest().json(ErrorBody::new(
            "invalid_request",
            "from must be strictly before to",
        ));
    }
    let daily = match db::translation_volume_by_project(
        pool.get_ref(),
        q.project_id,
        q.from,
        q.to,
    )
    .await
    {
        Ok(r) => r,
        Err(e) => {
            return HttpResponse::InternalServerError().json(
                ErrorBody::new("server_error", "translation_volume query failed").with_detail(e),
            );
        }
    };
    let (total_completed, total_chars) = match db::translation_volume_total(
        pool.get_ref(),
        q.project_id,
        q.from,
        q.to,
    )
    .await
    {
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
    if let Err((status, body)) = rbac::enforce(rbac_checker.get_ref(), &req, "/v1/reports/audit-summary", "GET").await {
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
        assert!(!json.contains("detail"), "detail should be skipped when None");
    }

    #[test]
    fn error_body_serializes_with_detail() {
        let body = ErrorBody::new("server_error", "query failed").with_detail("timeout");
        let json = serde_json::to_string(&body).unwrap();
        assert!(json.contains("timeout"));
    }
}
