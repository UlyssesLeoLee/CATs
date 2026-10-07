//! HTTP handlers (actix-web 4)
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/05-其他/管理/CATs_Baseline一览_v1.0.md §5.1
//! 引用: doc/05-其他/管理/CATs_错误码表_v1.0.md §3-§4 (error enum 复用)
//!
//! 端点 (per Sprint 1 拆解 v1.0+1 §2 T-02 完成判据):
//! - GET  /healthz
//! - POST /v1/users              — 创建 UserProfile (CRUD stub)
//! - GET  /v1/users/{id}         — 查询
//! - PUT  /v1/users/{id}         — 更新
//!
//! 错误码 (per 错误码表 v1.0):
//! - 200 成功
//! - 400 invalid_request (字段空 / 长度超限)
//! - 404 resource_not_found / user_not_found
//! - 409 email_conflict / username_conflict
//! - 500 server_error

use crate::db;
use crate::models::{CreateUserRequest, ErrorBody, GetUserResponse, UpdateUserRequest};
use actix_web::{web, HttpResponse, Responder};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    app: cats_common::AppMeta,
}

/// `GET /healthz` — 存活探针 + 启动探针复用
///
/// 响应形状 = 全仓统一后的唯一形状（per BACKEND_STATUS §4.1m）：
/// `{"status":"ok","app":{"name":...,"version":...}}`
///
/// 用 `env!("CARGO_PKG_NAME")` 而不是 `AppMeta::current()` —— 后者返回的是
/// **cats-common 自己**的包名，会让每个服务都自报 "cats-common"，监控分不出
/// 是谁应答的。这个坑见 asr-service/src/main.rs 里的同款注释。
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        app: cats_common::AppMeta {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

/// `POST /v1/users` — 创建 UserProfile
pub async fn create_user(
    pool: web::Data<PgPool>,
    body: web::Json<CreateUserRequest>,
) -> impl Responder {
    let req = body.into_inner();
    if req.display_name.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "display_name must not be empty".to_string(),
            detail: None,
        });
    }
    if req.display_name.len() > 100 {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "display_name must be ≤ 100 chars".to_string(),
            detail: None,
        });
    }
    let (profile, created) = match db::create(
        pool.get_ref(),
        req.user_id,
        &req.display_name,
        req.email.as_deref(),
        req.avatar_url.as_deref(),
        &req.locale,
        &req.timezone,
    )
    .await
    {
        Ok(p) => p,
        Err(e) => {
            // 唯一约束冲突 → 409 (per 错误码表 §3.4)
            let msg = format!("{}", e);
            if msg.contains("idx_user_profile_email_unique") {
                return HttpResponse::Conflict().json(ErrorBody {
                    error: "email_conflict".to_string(),
                    message: "email already registered".to_string(),
                    detail: None,
                });
            }
            return HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(msg),
            });
        }
    };
    let status = if created { 201 } else { 200 };
    HttpResponse::build(actix_web::http::StatusCode::from_u16(status).unwrap())
        .json(GetUserResponse::from(profile))
}

/// `GET /v1/users/{id}` — 查询 UserProfile (按主键 id)
pub async fn get_user(pool: web::Data<PgPool>, path: web::Path<String>) -> impl Responder {
    let id_str = path.into_inner();
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "id must be a valid UUID".to_string(),
                detail: None,
            });
        }
    };
    match db::find_by_id(pool.get_ref(), id).await {
        Ok(Some(p)) => HttpResponse::Ok().json(GetUserResponse::from(p)),
        Ok(None) => HttpResponse::NotFound().json(ErrorBody {
            error: "user_not_found".to_string(),
            message: "user not found".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorBody {
            error: "server_error".to_string(),
            message: "internal server error".to_string(),
            detail: Some(format!("{}", e)),
        }),
    }
}

/// `PUT /v1/users/{id}` — 更新 UserProfile
pub async fn update_user(
    pool: web::Data<PgPool>,
    path: web::Path<String>,
    body: web::Json<UpdateUserRequest>,
) -> impl Responder {
    let id_str = path.into_inner();
    let id = match Uuid::parse_str(&id_str) {
        Ok(u) => u,
        Err(_) => {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "id must be a valid UUID".to_string(),
                detail: None,
            });
        }
    };
    let req = body.into_inner();
    // display_name 长度校验 (避免空字符串覆盖)
    if let Some(d) = &req.display_name {
        if d.is_empty() || d.len() > 100 {
            return HttpResponse::BadRequest().json(ErrorBody {
                error: "invalid_request".to_string(),
                message: "display_name must be 1..=100 chars".to_string(),
                detail: None,
            });
        }
    }
    match db::update(
        pool.get_ref(),
        id,
        req.display_name.as_deref(),
        req.email.as_deref(),
        req.avatar_url.as_deref(),
        req.locale.as_deref(),
        req.timezone.as_deref(),
    )
    .await
    {
        Ok(Some(p)) => HttpResponse::Ok().json(GetUserResponse::from(p)),
        Ok(None) => HttpResponse::NotFound().json(ErrorBody {
            error: "user_not_found".to_string(),
            message: "user not found".to_string(),
            detail: Some(format!("id: {id}")),
        }),
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("idx_user_profile_email_unique") {
                return HttpResponse::Conflict().json(ErrorBody {
                    error: "email_conflict".to_string(),
                    message: "email already registered".to_string(),
                    detail: None,
                });
            }
            HttpResponse::InternalServerError().json(ErrorBody {
                error: "server_error".to_string(),
                message: "internal server error".to_string(),
                detail: Some(msg),
            })
        }
    }
}

// =====================================================================
// **唯一路由表** —— `main.rs` 与集成测试都从这里接进来
// =====================================================================
//
// 2026-10-07。这 4 条原本内联在 `main.rs`，而 `tests/e2e_t02.rs` 的
// `make_app` 逐条抄了一份（4 = 4，逐条核对完全一致，所以改之前不是假绿）。
// 与 `auth-service` / `task-service` / `audit-service` 一样收敛到单一事实来源。
//
// 这里的 app_data 是 `web::Data<PgPool>`（不是自定义 AppState）：三个 handler
// 直接取连接池。`main.rs` 的 HttpServer 闭包是 `Fn`，每个 worker 线程各调一次，
// 所以要**先 clone 再 move**（`web::Data` 内封 Arc，clone 廉价）。
pub fn configure_routes(cfg: &mut web::ServiceConfig, pool: web::Data<PgPool>) {
    cfg.app_data(pool)
        .route("/healthz", web::get().to(healthz))
        .route("/v1/users", web::post().to(create_user))
        .route("/v1/users/{id}", web::get().to(get_user))
        .route("/v1/users/{id}", web::put().to(update_user));
}

// =====================================================================
// 测试辅助: 单元测试覆盖字段校验逻辑
// =====================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_create_user(display_name: &str) -> CreateUserRequest {
        CreateUserRequest {
            user_id: Uuid::new_v4(),
            display_name: display_name.to_string(),
            email: None,
            avatar_url: None,
            locale: "ja-JP".to_string(),
            timezone: "Asia/Tokyo".to_string(),
        }
    }

    #[test]
    fn create_request_display_name_validation() {
        let empty = make_create_user("");
        assert!(empty.display_name.is_empty());
        let long = make_create_user(&"a".repeat(101));
        assert!(long.display_name.len() > 100);
    }

    #[test]
    fn default_locale_and_timezone() {
        let req = CreateUserRequest {
            user_id: Uuid::new_v4(),
            display_name: "alice".to_string(),
            email: None,
            avatar_url: None,
            locale: crate::models::default_locale(),
            timezone: crate::models::default_timezone(),
        };
        assert_eq!(req.locale, "ja-JP");
        assert_eq!(req.timezone, "Asia/Tokyo");
    }
}
