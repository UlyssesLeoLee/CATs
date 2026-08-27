//! `auth-service` 入口
//!
//! M1: 业务 endpoint 上线
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1

use actix_web::{web, App, HttpServer};
use std::env;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // tracing 初始化
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // DATABASE_URL 必须设置, 否则 fail-fast (per 安全约束: 不打印值)
    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        std::process::exit(1);
    }
    if env::var("JWT_SECRET").is_err() {
        eprintln!("ERROR: JWT_SECRET env var not set");
        std::process::exit(1);
    }

    let bind_addr = env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8081".to_string());

    // 构造连接池 (lazy, 不实际连 DB)
    let pool = match auth_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    // 启动时尝试检测并创建种子用户 (per 任务规范)
    // 仅在 SEED_USER 和 SEED_PASSWORD env var 都设置时
    if let (Ok(seed_user), Ok(seed_pass)) = (env::var("SEED_USER"), env::var("SEED_PASSWORD")) {
        let seed_email = env::var("SEED_EMAIL").ok();
        match auth_service::db::ensure_seed_user(
            &pool,
            &seed_user,
            &seed_pass,
            seed_email.as_deref(),
        )
        .await
        {
            Ok(true) => info!(user = %seed_user, "seed user created"),
            Ok(false) => info!(user = %seed_user, "seed user already exists"),
            Err(e) => eprintln!("ERROR: seed user create failed: {e}"),
        }
    }

    info!(bind_addr = %bind_addr, "starting auth-service");

    let state = auth_service::handlers::AppState::new(pool);
    let state_data = web::Data::new(state);
    HttpServer::new(move || {
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(auth_service::handlers::healthz))
            .route(
                "/v1/auth/login",
                web::post().to(auth_service::handlers::login),
            )
            .route(
                "/v1/auth/refresh",
                web::post().to(auth_service::handlers::refresh),
            )
            .route(
                "/v1/auth/logout",
                web::post().to(auth_service::handlers::logout),
            )
            .route("/v1/auth/me", web::get().to(auth_service::handlers::me))
    })
    .bind(&bind_addr)?
    .run()
    .await
}
