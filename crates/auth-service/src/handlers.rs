//! HTTP handlers (actix-web 4)
//!
//! 引用: api/openapi/cats-openapi-v1.yaml
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §3 (REST + gRPC)
//! 引用: doc/05-其他/管理/CATs_M1_Sprint1_任务拆解_v1.0.md §2 T-01 (refresh 轮换 + logout + 错误码)

use crate::audit::AuditSink;
use crate::auth::{issue_jwt, verify_jwt, verify_password};
use crate::db;
use crate::models::{
    AuditEvent, AuditOutcome, Claims, ErrorBody, LoginRequest, LoginResponse, LogoutRequest,
    LogoutResponse, MeResponse, RefreshRequest, RefreshResponse,
};
use actix_web::{web, HttpRequest, HttpResponse, Responder};
use chrono::Utc;
use serde_json::json;
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

/// AppState: PgPool + AuditSink (dyn) + 配置
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub audit: Arc<dyn AuditSink>,
}

impl AppState {
    /// 构造 AppState, 用 DbAuditSink (生产默认)
    pub fn new(pool: PgPool) -> Self {
        let audit_sink: Arc<dyn AuditSink> = Arc::new(crate::audit::DbAuditSink::new(pool.clone()));
        Self {
            pool,
            audit: audit_sink,
        }
    }

    /// 测试构造: 用 InMemoryAuditSink 替代 DB
    #[cfg(any(test, debug_assertions))]
    pub fn new_with_sink(pool: PgPool, sink: std::sync::Arc<dyn AuditSink>) -> Self {
        Self { pool, audit: sink }
    }
}

// =====================================================================
// 审计辅助: 构造 AuditEvent 并 emit
// =====================================================================

/// 同步构造 audit 事件并 emit (失败仅 log, 不阻塞主流程)
///
/// 选择 await 而非 spawn 的理由:
/// - audit sink 都是本地操作 (DB ~ms / InMemory 立即), 不会显著增加响应时延
/// - 同步保证 e2e 立即可见 (避免 actix_rt::spawn 在 init_service 中不被驱动)
/// - 生产 audit 失败要 log 但不能影响主流程 → 已 try log
async fn build_audit(
    state: &AppState,
    user_id: Option<Uuid>,
    event_type: &str,
    outcome: AuditOutcome,
    detail: Option<serde_json::Value>,
    req: &HttpRequest,
) -> AuditEvent {
    let event = AuditEvent {
        event_id: Uuid::new_v4(),
        user_id,
        event_type: event_type.to_string(),
        outcome,
        detail,
        source_ip: req
            .connection_info()
            .realip_remote_addr()
            .map(|s| s.to_string()),
        user_agent: req
            .headers()
            .get("User-Agent")
            .and_then(|h| h.to_str().ok())
            .map(|s| s.to_string()),
        occurred_at: Utc::now(),
    };
    if let Err(err) = state.audit.emit(&event).await {
        tracing::error!(event_type = %event.event_type, "audit emit failed: {err}");
    }
    event
}

// =====================================================================
// /healthz
// =====================================================================

/// GET /healthz
pub async fn healthz() -> impl Responder {
    HttpResponse::Ok().json(json!({"status": "ok", "service": "auth-service"}))
}

// =====================================================================
// POST /v1/auth/login
// =====================================================================

/// POST /v1/auth/login
pub async fn login(
    state: web::Data<AppState>,
    body: web::Json<LoginRequest>,
    req: HttpRequest,
) -> impl Responder {
    let req_body = body.into_inner();
    if req_body.username.is_empty() || req_body.password.is_empty() {
        return HttpResponse::BadRequest().json(ErrorBody {
            error: "invalid_request".to_string(),
            message: "username and password required".to_string(),
            detail: None,
        });
    }
    let user = match db::find_by_username(&state.pool, &req_body.username).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            // 审计: login_failed (用户不存在)
            build_audit(
                state.get_ref(),
                None,
                "login_failed",
                AuditOutcome::Failure,
                Some(json!({"reason": "user_not_found", "username": &req_body.username})),
                &req,
            )
            .await;
            return unauthorized();
        }
        Err(e) => return server_error(&format!("db query failed: {e}")),
    };
    if !user.is_active {
        build_audit(
            state.get_ref(),
            Some(user.id),
            "login_failed",
            AuditOutcome::Failure,
            Some(json!({"reason": "user_inactive"})),
            &req,
        )
        .await;
        return unauthorized();
    }
    if !verify_password(&req_body.password, &user.password_hash) {
        build_audit(
            state.get_ref(),
            Some(user.id),
            "login_failed",
            AuditOutcome::Failure,
            Some(json!({"reason": "wrong_password"})),
            &req,
        )
        .await;
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

    // 审计: login success
    build_audit(
        state.get_ref(),
        Some(user.id),
        "login",
        AuditOutcome::Success,
        Some(json!({"method": "password"})),
        &req,
    )
    .await;

    HttpResponse::Ok().json(LoginResponse {
        access_token,
        refresh_token,
        expires_in: access_exp,
        token_type: "Bearer".to_string(),
        user_id: user.id.to_string(),
        username: user.username,
    })
}

// =====================================================================
// POST /v1/auth/refresh (T-01: 轮换 - 旧 jti 撤销, 新 jti 签发)
// =====================================================================

/// POST /v1/auth/refresh (T-01: 带 jti 轮换的 refresh)
///
/// 流程:
/// 1. 验证 refresh_token JWT 签名 + 类型
/// 2. 检查 jti 是否已撤销 (轮换检查)
/// 3. 撤销旧 jti (reason='rotated')
/// 4. 签发新 access_token + 新 refresh_token (新 jti)
/// 5. 审计: refresh success + refresh_revoked
pub async fn refresh(
    state: web::Data<AppState>,
    body: web::Json<RefreshRequest>,
    req: HttpRequest,
) -> impl Responder {
    let req_body = body.into_inner();
    let claims: Claims = match verify_jwt(&req_body.refresh_token) {
        Ok(c) => c,
        Err(e) => {
            // 审计: refresh_failed (token 签名/格式错)
            build_audit(
                state.get_ref(),
                None,
                "refresh_failed",
                AuditOutcome::Failure,
                Some(json!({"reason": "invalid_token"})),
                &req,
            )
            .await;
            return HttpResponse::Unauthorized().json(e);
        }
    };
    if claims.token_type != "refresh" {
        build_audit(
            state.get_ref(),
            None,
            "refresh_failed",
            AuditOutcome::Failure,
            Some(json!({"reason": "wrong_token_type", "got": claims.token_type})),
            &req,
        )
        .await;
        return HttpResponse::Unauthorized().json(ErrorBody {
            error: "invalid_token_type".to_string(),
            message: "expected refresh token".to_string(),
            detail: None,
        });
    }
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => return unauthorized_token("invalid_subject"),
    };
    let old_jti = match Uuid::parse_str(&claims.jti) {
        Ok(j) => j,
        Err(_) => return unauthorized_token("invalid_jti"),
    };

    // 关键: 检查 jti 是否已被撤销 (二次使用必须 401)
    match db::jti_is_revoked(&state.pool, old_jti).await {
        Ok(true) => {
            build_audit(
                state.get_ref(),
                Some(user_id),
                "refresh_failed",
                AuditOutcome::Failure,
                Some(json!({"reason": "jti_revoked", "jti": old_jti.to_string()})),
                &req,
            )
            .await;
            return HttpResponse::Unauthorized().json(ErrorBody {
                error: "token_revoked".to_string(),
                message: "refresh token has been revoked".to_string(),
                detail: None,
            });
        }
        Ok(false) => {} // OK, 未撤销
        Err(e) => return server_error(&format!("jti_is_revoked query failed: {e}")),
    }

    let user = match db::find_by_id(&state.pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => {
            build_audit(
                state.get_ref(),
                Some(user_id),
                "refresh_failed",
                AuditOutcome::Failure,
                Some(json!({"reason": "user_not_found"})),
                &req,
            )
            .await;
            return unauthorized();
        }
        Err(e) => return server_error(&format!("db query failed: {e}")),
    };
    if !user.is_active {
        build_audit(
            state.get_ref(),
            Some(user.id),
            "refresh_failed",
            AuditOutcome::Failure,
            Some(json!({"reason": "user_inactive"})),
            &req,
        )
        .await;
        return unauthorized();
    }

    // 撤销旧 jti
    if let Err(e) = db::revoke_jti(&state.pool, old_jti, user.id, "rotated").await {
        return server_error(&format!("revoke_jti failed: {e}"));
    }

    // 签发新 access + 新 refresh (新 jti)
    let (access_token, access_exp) = match issue_jwt(user.id, &user.username, "access") {
        Ok(p) => p,
        Err(e) => return server_error(&format!("jwt issue failed: {e}")),
    };
    let (refresh_token, _) = match issue_jwt(user.id, &user.username, "refresh") {
        Ok(p) => p,
        Err(e) => return server_error(&format!("jwt refresh issue failed: {e}")),
    };

    // 审计: refresh success + refresh_revoked
    build_audit(
        state.get_ref(),
        Some(user.id),
        "refresh",
        AuditOutcome::Success,
        Some(json!({"old_jti": old_jti.to_string(), "rotated": true})),
        &req,
    )
    .await;
    build_audit(
        state.get_ref(),
        Some(user.id),
        "refresh_revoked",
        AuditOutcome::Success,
        Some(json!({"jti": old_jti.to_string(), "reason": "rotated"})),
        &req,
    )
    .await;

    HttpResponse::Ok().json(RefreshResponse {
        access_token,
        refresh_token,
        expires_in: access_exp,
        token_type: "Bearer".to_string(),
    })
}

// =====================================================================
// POST /v1/auth/logout (T-01 新增)
// =====================================================================

/// POST /v1/auth/logout (T-01 新增)
///
/// 撤销 refresh_token (jti), 写 audit_log + emit AuditSink
pub async fn logout(
    state: web::Data<AppState>,
    body: web::Json<LogoutRequest>,
    req: HttpRequest,
) -> impl Responder {
    let req_body = body.into_inner();
    let claims: Claims = match verify_jwt(&req_body.refresh_token) {
        Ok(c) => c,
        Err(e) => {
            build_audit(
                state.get_ref(),
                None,
                "logout_failed",
                AuditOutcome::Failure,
                Some(json!({"reason": "invalid_token"})),
                &req,
            )
            .await;
            return HttpResponse::Unauthorized().json(e);
        }
    };
    if claims.token_type != "refresh" {
        return HttpResponse::Unauthorized().json(ErrorBody {
            error: "invalid_token_type".to_string(),
            message: "expected refresh token".to_string(),
            detail: None,
        });
    }
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => return unauthorized_token("invalid_subject"),
    };
    let jti = match Uuid::parse_str(&claims.jti) {
        Ok(j) => j,
        Err(_) => return unauthorized_token("invalid_jti"),
    };

    let now = Utc::now();
    if let Err(e) = db::revoke_jti(&state.pool, jti, user_id, "logout").await {
        return server_error(&format!("revoke_jti failed: {e}"));
    }

    build_audit(
        state.get_ref(),
        Some(user_id),
        "logout",
        AuditOutcome::Success,
        Some(json!({"jti": jti.to_string()})),
        &req,
    )
    .await;

    HttpResponse::Ok().json(LogoutResponse {
        revoked: true,
        revoked_at: now,
    })
}

// =====================================================================
// GET /v1/auth/me
// =====================================================================

/// GET /v1/auth/me
///
/// 接受 `Authorization: Bearer <access_token>`, 返回当前用户信息 (per OpenAPI v1 §3.2)
pub async fn me(state: web::Data<AppState>, req: HttpRequest) -> impl Responder {
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

    let claims: Claims = match verify_jwt(token) {
        Ok(c) => c,
        Err(e) => {
            build_audit(
                state.get_ref(),
                None,
                "me_failed",
                AuditOutcome::Failure,
                Some(json!({"reason": "invalid_token"})),
                &req,
            )
            .await;
            return HttpResponse::Unauthorized().json(e);
        }
    };

    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(u) => u,
        Err(_) => return unauthorized_token("invalid_subject"),
    };

    let user = match db::find_by_id(&state.pool, user_id).await {
        Ok(Some(u)) => u,
        Ok(None) => return unauthorized_token("user_not_found"),
        Err(e) => return server_error(&format!("db query failed: {e}")),
    };

    if !user.is_active {
        return unauthorized_token("user_inactive");
    }

    // 审计: me 访问成功 (best-effort, 不阻塞响应)
    build_audit(
        state.get_ref(),
        Some(user.id),
        "me_access",
        AuditOutcome::Success,
        None,
        &req,
    )
    .await;

    HttpResponse::Ok().json(MeResponse {
        user_id: user.id.to_string(),
        username: user.username,
        email: user.email.unwrap_or_default(),
    })
}

// =====================================================================
// 错误响应辅助
// =====================================================================

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
