//! HTTP handlers (actix-web 4)
//!
//! 引用: api/openapi/cats-openapi-v1.yaml
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3 (REST + gRPC)

use crate::auth::{issue_jwt, verify_jwt, verify_password};
use crate::db;
use crate::models::{
    Claims, ErrorBody, LoginRequest, LoginResponse, MeResponse, RefreshRequest, RefreshResponse,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use serde_json::json;
use sqlx::PgPool;

/// POST /v1/auth/login
pub async fn login(pool: web::Data<PgPool>, body: web::Json<LoginRequest>) -> impl Responder {
    let req = body.into_inner();
    if req.username.is_empty() || req.password.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "username and password required".to_string(),
            detail: None,
        });
    }
    let user = match db::find_by_username(pool.get_ref(), &req.username).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized(),
        Err(e) => return server_error(&format!("db query failed: {e}")),
    };
    if !user.is_active {
        return unauthorized();
    }
    if !verify_password(&req.password, &user.password_hash) {
        return unauthorized();
    }
    let (access_token, access_exp) = match issue_jwt(user.id, &user.username, "access") {
        Ok(p) => p,
        Err(e) => return server_error(&format!("jwt issue failed: {e}")),
    };
    let (refresh_token, _refresh_exp) = match issue_jwt(user.id, &user.username, "refresh") {
        Ok(p) => p,
        Err(e) => return server_error(&format!("jwt refresh issue failed: {e}")),
    };
    HttpResponse::Ok().json(LoginResponse {
        access_token,
        refresh_token,
        expires_in: access_exp,
        token_type: "Bearer".to_string(),
        user_id: user.id.to_string(),
        username: user.username,
    })
}

/// POST /v1/auth/refresh
pub async fn refresh(pool: web::Data<PgPool>, body: web::Json<RefreshRequest>) -> impl Responder {
    let req = body.into_inner();
    let claims: Claims = match verify_jwt(&req.refresh_token) {
        Ok(c) => c,
        Err(e) => return HttpResponse::Unauthorized().json(e),
    };
    if claims.token_type != "refresh" {
        return HttpResponse::Unauthorized().json(ErrorBody {
            error: "invalid_token_type".to_string(),
            message: "expected refresh token".to_string(),
            detail: None,
        });
    }
    let user_id = match uuid::Uuid::parse_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => return unauthorized(),
    };
    let user = match db::find_by_id(pool.get_ref(), user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized(),
        Err(e) => return server_error(&format!("db query failed: {e}")),
    };
    if !user.is_active {
        return unauthorized();
    }
    let (access_token, access_exp) = match issue_jwt(user.id, &user.username, "access") {
        Ok(p) => p,
        Err(e) => return server_error(&format!("jwt issue failed: {e}")),
    };
    let (refresh_token, _) = match issue_jwt(user.id, &user.username, "refresh") {
        Ok(p) => p,
        Err(e) => return server_error(&format!("jwt refresh issue failed: {e}")),
    };
    HttpResponse::Ok().json(RefreshResponse {
        access_token,
        refresh_token,
        expires_in: access_exp,
        token_type: "Bearer".to_string(),
    })
}

/// GET /healthz
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok", "service": "auth-service"}))
}

/// GET /v1/auth/me
///
/// 接受 `Authorization: Bearer <access_token>`, 返回当前用户信息 (per OpenAPI v1 §3.2)
///
/// 错误:
/// - 401 缺 header / token 无效 / 过期 / 用户不存在 / 用户未激活
pub async fn me(req: HttpRequest, pool: web::Data<PgPool>) -> impl Responder {
    // 1. 提取 Bearer token
    let token = match req.headers().get("Authorization") {
        Some(h) => match h.to_str() {
            Ok(s) => s,
            Err(_) => return unauthorized_token("invalid_authorization_header"),
        },
        None => return unauthorized_token("missing_authorization"),
    };
    let token = match token
        .strip_prefix("Bearer ")
        .or_else(|| token.strip_prefix("bearer "))
    {
        Some(t) => t.trim(),
        None => return unauthorized_token("invalid_authorization_scheme"),
    };
    if token.is_empty() {
        return unauthorized_token("missing_token");
    }

    // 2. 验证 JWT (复用 auth::verify_jwt)
    let claims: Claims = match verify_jwt(token) {
        Ok(c) => c,
        Err(e) => {
            return HttpResponse::Unauthorized().json(e);
        }
    };

    // 3. 解析 user_id
    let user_id = match uuid::Uuid::parse_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => return unauthorized_token("invalid_subject"),
    };

    // 4. 查 DB (复用 db::find_by_id, 等价于按 user_id 查询)
    let user = match db::find_by_id(pool.get_ref(), user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized_token("user_not_found"),
        Err(e) => return server_error(&format!("db query failed: {e}")),
    };

    // 5. 检查 is_active
    if !user.is_active {
        return unauthorized_token("user_inactive");
    }

    HttpResponse::Ok().json(MeResponse {
        user_id: user.id.to_string(),
        username: user.username,
        email: user.email.unwrap_or_default(),
    })
}

fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized().json(ErrorBody {
        error: "invalid_credentials".to_string(),
        message: "invalid username or password".to_string(),
        detail: None,
    })
}

fn server_error(detail: &str) -> HttpResponse {
    HttpResponse::InternalServerError().json(ErrorBody {
        error: "server_error".to_string(),
        message: "internal server error".to_string(),
        detail: Some(detail.to_string()),
    })
}

/// 401 错误 (用于 token 解析 / 验证失败)
fn unauthorized_token(reason: &str) -> HttpResponse {
    HttpResponse::Unauthorized().json(ErrorBody {
        error: "invalid_token".to_string(),
        message: format!("token validation failed: {reason}"),
        detail: None,
    })
}
