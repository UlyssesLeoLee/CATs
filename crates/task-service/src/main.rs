//! `task-service` 入口 (per ULYS-151 切片 B-2)
//!
//! 引用: doc/05-其他/MVP商业版/_slice_b2_task.md
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//!
//! M1 阶段 (切片 B-2): 5 个业务 endpoint + SSE 进度推送 + RBAC inline 校验
//! 引用 ULYS-45 子任务 A (af8abb6) 既有 SSE 实现, 沿用内存事件总线模式
//!
//! 业务配置:
//! - `BIND_ADDR` — 默认 `0.0.0.0:8084`
//! - `DATABASE_URL` — task_db 连接 URL, **必填** (per §5.2 注入规范; 缺则 fail-fast)
//! - `RBAC_DISABLED=1` — 关闭 RBAC 检查 (本地/CI 调试用; 生产严禁)
//! - `EVENT_BUFFER` — 事件总线 broadcast buffer (默认 1024)

use actix_web::{web, App, HttpServer};
use std::env;
use std::sync::Arc;
use tracing::info;
use tracing_subscriber::EnvFilter;

/// 业务配置 (M1 阶段)
#[derive(Debug, Clone)]
struct Config {
    bind_addr: String,
    event_buffer: usize,
    rbac_disabled: bool,
}

impl Config {
    fn from_env() -> Self {
        let event_buffer = env::var("EVENT_BUFFER")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(1024);
        let rbac_disabled = env::var("RBAC_DISABLED")
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8084".to_string()),
            event_buffer,
            rbac_disabled,
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // tracing 初始化
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    // DATABASE_URL 必须设置 (per 安全约束: 不打印值)
    if env::var("DATABASE_URL").is_err() {
        eprintln!("ERROR: DATABASE_URL env var not set");
        std::process::exit(1);
    }

    let cfg = Config::from_env();
    info!(
        bind_addr = %cfg.bind_addr,
        event_buffer = cfg.event_buffer,
        rbac_disabled = cfg.rbac_disabled,
        "starting task-service"
    );

    // 构造连接池 (lazy, 不实际连 DB)
    let pool = match task_service::db::build_pool().await {
        Ok(p) => p,
        Err(e) => {
            eprintln!("ERROR: db pool build failed: {e}");
            std::process::exit(1);
        }
    };

    // 构造 AppState (默认 EventBus buffer; rbac_disabled 走 no-op checker)
    let state = if cfg.rbac_disabled {
        warn_no_rbac();
        task_service::AppState::new_with_buffer(pool, cfg.event_buffer)
    } else {
        task_service::AppState::new_with_buffer(pool, cfg.event_buffer)
    };

    let state_data = web::Data::new(state);
    let cfg_for_log = cfg.clone();
    HttpServer::new(move || {
        let cfg_log = cfg_for_log.clone();
        App::new()
            .app_data(state_data.clone())
            .route("/healthz", web::get().to(task_service::handlers::healthz))
            .route(
                "/v1/tasks",
                web::post().to(task_service::handlers::create_task),
            )
            .route(
                "/v1/tasks",
                web::get().to(task_service::handlers::list_tasks),
            )
            .route(
                "/v1/tasks/{id}",
                web::get().to(task_service::handlers::get_task),
            )
            .route(
                "/v1/tasks/{id}/status",
                web::patch().to(task_service::handlers::update_task_status),
            )
            .route(
                "/v1/tasks/{id}/events",
                web::get().to(task_service::handlers::task_events_sse),
            )
            .route(
                "/internal/v1/tasks/{id}/stage-progress",
                web::post().to(task_service::handlers::stage_progress),
            )
            .app_data(web::Data::new(Arc::new(cfg_log)))
    })
    .bind(&cfg.bind_addr)?
    .run()
    .await
}

/// 仅在 RBAC_DISABLED=1 时输出强警告 (生产路径不应触发)
fn warn_no_rbac() {
    eprintln!(
        "WARN: RBAC_DISABLED=1 — task-service is running without role checks. \
         This MUST NOT be used in production."
    );
}