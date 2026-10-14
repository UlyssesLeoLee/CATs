//! worker-service HTTP handlers

use crate::state::AppState;
use actix_web::{web, HttpResponse, Responder};
use cats_common::error::{cats_error_to_response, CatsError, ErrorCode};
use cats_rbac::service_helpers::extract_user_id_and_roles;
use cats_rbac::Role;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    app: cats_common::AppMeta,
}

/// **唯一的路由表** —— `main.rs` 与集成测试共用。
///
/// 2026-10-07 抽出。理由与 audit-service / user-service 等同一条：集成测试如果
/// 自己在测试文件里重抄一遍路由，那么 `main.rs` 哪天少注册一条路由，测试仍然
/// 全绿 —— 它验证的不是生产实际提供的东西。
///
/// `app_data` 也一并在这里注册（而不是留在调用方），否则"注册哪些共享状态"
/// 又会变成调用方与测试各写一份。
///
/// 调用方注意：`HttpServer::new` 的闭包是 `Fn`（每个 worker 线程都会再调一次），
/// 闭包里把两个 `web::Data` **先 clone 出来再 move 进去**。
pub fn configure_routes(
    cfg: &mut web::ServiceConfig,
    state: web::Data<AppState>,
    pool: web::Data<PgPool>,
) {
    cfg.app_data(state)
        .app_data(pool)
        .route("/healthz", web::get().to(healthz))
        .route("/readyz", web::get().to(readyz))
        .route("/v1/worker/tick", web::post().to(manual_tick));
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
    let db_ok = sqlx::query(r#"SELECT 1"#)
        .execute(pool.get_ref())
        .await
        .is_ok();
    // 2026-10-07 行为变更：DB 连不上时原来仍返回 **200**，只在 body 里写 `db:"fail"`。
    // 而 k8s 的 readinessProbe 只看状态码 —— `deploy/k3s/cats-core/worker-service.yaml:33`
    // 正是拿 `/readyz` 当 readinessProbe，于是数据库挂掉的 Pod 照样被判 Ready
    // 继续接流量。就绪探针说谎等于故障静默放大，所以失败时必须给 503。
    //
    // 形状与状态码判定已收敛到 `cats_common::ReadyResponse`：此前本文件与
    // audit-service 各有一份**逐字相同**的本地结构体，两份副本会各自漂移，
    // 而没有任何检查能发现。
    let body = cats_common::ReadyResponse::new(env!("CARGO_PKG_NAME"), db_ok);
    HttpResponse::build(
        actix_web::http::StatusCode::from_u16(body.status_code())
            .expect("status_code 只返回 200/503，都是合法状态码"),
    )
    .json(body)
}

/// 允许手动触发 tick 的角色（显式白名单）。
///
/// 2026-10-07 新增。原先这个列表是**内联**传给 `require_roles` 的，而
/// `require_roles` 的第二个参数语义是「**调用者**的角色」，不是「允许的角色」——
/// 循环问的是「角色 Sponsor 有没有 Task:Update」（有，第一个迭代就 Ok），
/// 调用者是谁根本不影响结果。实测 `X-Cats-Roles: User` 与 `Guest` 都能穿过
/// RBAC 进入 `tick()`。
///
/// **为什么这里不用权限矩阵判定**：`default_permissions()` 里 `Task:Update`
/// 目前**只有 Sponsor 持有**（RustLead 只对 `Service` 域全权，SRELead 无 Update）。
/// 若改成 `require_roles(&checker, &caller_roles, Task, Update)`，RustLead 与
/// SRELead 会被矩阵挡掉，与本 handler「这三个角色可 tick」的原意相反。
/// 白名单才是这里真正想表达的东西。
const TICK_ROLES: [Role; 3] = [Role::Sponsor, Role::RustLead, Role::SRELead];

/// POST /v1/worker/tick — 手动触发一次 tick (开发/测试用)
pub async fn manual_tick(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> impl Responder {
    let (_user_id, caller_roles) = match extract_user_id_and_roles(&req) {
        Ok(v) => v,
        Err(e) => return cats_error_to_response(&e),
    };

    // 调用者必须真的持有 TICK_ROLES 之一。判定依据是**请求头解出来的角色**，
    // 不是那个曾经被误当成「允许列表」的字面量数组。
    if !caller_roles.iter().any(|r| TICK_ROLES.contains(r)) {
        return cats_error_to_response(&CatsError::business(
            ErrorCode::OperationNotPermitted,
            format!(
                "operation_not_permitted: required_role=Sponsor|RustLead|SRELead \
                 actual_role={caller_roles:?} resource=Task action=Update"
            ),
        ));
    }

    match crate::scheduler::tick(&state.pool).await {
        Ok(()) => HttpResponse::Ok().json(serde_json::json!({"status": "ticked"})),
        Err(e) => cats_error_to_response(&e),
    }
}
