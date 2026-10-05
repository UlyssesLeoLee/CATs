//! HTTP handlers (audit-service)

use crate::db;
use crate::models::{AuditLogListResponse, AuditLogResponse, KafkaAuditEvent};
use crate::state::AppState;
use actix_web::{web, HttpRequest, HttpResponse, Responder};
// 2026-10-05：原先这里还 import 了 `CatsError` 与 `ErrorCode`，但整个文件
// 从未显式用到过它们（错误类型都是推断出来的）。因为本文件此前从未被编译，
// 没人看见过这个未用 import —— 接线后 CI 的 `clippy -D warnings` 会直接红。
use cats_common::cats_error_to_response;
use cats_rbac::service_helpers::{extract_user_id_and_roles, require_roles};
use cats_rbac::{Action, Resource};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// 健康检查响应
///
/// 2026-10-05: 用 `AppMeta` + 本 crate 的 `env!`，与另外 11 个 service 保持
/// 一致。原来这里是个只带 `service` 字段的私有结构，`main.rs` 里又另有一份
/// `/healthz` —— 同一个端点两套实现、两种响应形状。
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    app: cats_common::AppMeta,
}

/// **唯一的路由表** —— `main.rs` 与集成测试共用。
///
/// 2026-10-05 抽出。理由是可验证性：集成测试如果自己在测试文件里重抄一遍
/// 路由，那么 `main.rs` 哪天少注册一条路由，测试仍然全绿 —— 它验证的不是
/// 生产实际提供的东西。共用这一个函数之后，"服务到底对外暴露什么"只有一个
/// 事实来源。
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/healthz", web::get().to(healthz))
        .route("/readyz", web::get().to(readyz))
        .route("/v1/audit-logs", web::get().to(list_audit_logs))
        .route("/v1/audit-logs/test", web::post().to(test_ingest));
}

pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: cats_common::AppMeta {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

pub async fn readyz(pool: web::Data<PgPool>) -> impl Responder {
    #[derive(Serialize)]
    struct ReadyResponse {
        status: &'static str,
        service: &'static str,
        db: &'static str,
    }
    let db_ok = sqlx::query(r#"SELECT 1"#)
        .execute(pool.get_ref())
        .await
        .is_ok();
    HttpResponse::Ok().json(ReadyResponse {
        status: if db_ok { "ready" } else { "not_ready" },
        service: "audit-service",
        db: if db_ok { "ok" } else { "fail" },
    })
}

#[derive(Deserialize)]
pub struct ListQuery {
    pub org_id: Uuid,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    50
}

pub async fn list_audit_logs(
    state: web::Data<AppState>,
    req: HttpRequest,
    q: web::Query<ListQuery>,
) -> impl Responder {
    // 2026-10-05 接线时改掉的两处（原代码从未被编译）：
    //
    // 1. 它硬编码了一份"谁被允许"的角色表，还把 `extract_user_id_and_roles`
    //    解出来的 roles 丢成了 `_roles`。`require_roles` 的语义是
    //    "**调用方自己的角色**里是否有任一个具备该权限"（见
    //    cats_rbac::service_helpers 的文档），所以正确用法就是把调用方的
    //    角色传进去，让权限矩阵判定。
    // 2. 那份硬编码表里的 `Role::ReviewLead` / `Role::PlatformLead`
    //    **在 cats-rbac 里根本不存在**（实际是 Sponsor / ArchitectLead /
    //    RustLead / DatabaseLead / QualityLead / ProjectLead / SRELead /
    //    User / Guest）—— 它照着一套过时的角色表写的。
    //
    // 改成不硬编码还有个好处：权限矩阵以后调整，这里自动跟着变，
    // 不会再出现"矩阵改了、handler 还按老表判"的情况。
    let (user_id, roles) = match extract_user_id_and_roles(&req) {
        Ok(v) => v,
        Err(e) => return cats_error_to_response(&e),
    };
    if let Err(e) = require_roles(&state.checker, &roles, Resource::Audit, Action::Read).await {
        return cats_error_to_response(&e);
    }
    tracing::info!(user_id = %user_id, org_id = %q.org_id, "list audit logs");

    let limit = q.limit.clamp(1, 200);
    let offset = q.offset.max(0);
    let rows = match db::list_by_org(&state.pool, q.org_id, limit, offset).await {
        Ok(r) => r,
        Err(e) => return cats_error_to_response(&e),
    };
    let total = match db::count_by_org(&state.pool, q.org_id).await {
        Ok(t) => t,
        Err(e) => return cats_error_to_response(&e),
    };
    HttpResponse::Ok().json(AuditLogListResponse {
        items: rows.into_iter().map(AuditLogResponse::from).collect(),
        total,
        limit,
        offset,
    })
}

/// POST /v1/audit-logs/test — 手动 ingest 一条 (开发/测试用)
///
/// 2026-10-05 接线时补的鉴权。原签名把 `HttpRequest` 写成 `_req` —— 它
/// **完全没有做任何校验**：任何能连到 8088 端口的人都可以往审计表里写行。
/// 在一个审计服务里留一个免鉴权的写入口，等于让审计记录可以被伪造。
///
/// 要求 `Resource::Audit × Action::Audit`（矩阵里 DatabaseLead / Sponsor
/// 具备；Action::Audit 比 Read 更严，普通 User 连读都拿不到）。
pub async fn test_ingest(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> impl Responder {
    let (user_id, roles) = match extract_user_id_and_roles(&req) {
        Ok(v) => v,
        Err(e) => return cats_error_to_response(&e),
    };
    if let Err(e) = require_roles(&state.checker, &roles, Resource::Audit, Action::Audit).await {
        return cats_error_to_response(&e);
    }
    let ev = KafkaAuditEvent {
        event_id: Uuid::new_v4(),
        org_id: Uuid::new_v4(),
        // 接线前这里是 `None`（因为根本没有调用方概念）。既然现在有
        // 鉴权，审计记录就应该落下真实操作者，而不是"未知"。
        actor_user_id: Some(user_id),
        action: "test.event".to_string(),
        resource_type: "test".to_string(),
        resource_id: "test-0".to_string(),
        before_state: None,
        after_state: Some(body.into_inner()),
        ip: None,
        occurred_at: chrono::Utc::now(),
        schema_version: 1,
    };
    match db::insert_event(&state.pool, &ev).await {
        Ok(id) => HttpResponse::Ok().json(serde_json::json!({"audit_log_id": id})),
        Err(e) => cats_error_to_response(&e),
    }
}
