//! `cats-bff` 入口
//!
//! 启动流程：
//! 1. 初始化 tracing
//! 2. 从 env 读 [`Config`]
//! 3. 构造 [`AppState`]（含 reqwest 透传 client + tonic gRPC channel）
//! 4. 起 actix-web HttpServer，注册全部路由
//!
//! 端点清单见 [`crate::routes::configure`]。

use actix_web::{web, App, HttpServer};
use tracing::info;

use cats_bff::{config::Config, AppState};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();

    let cfg = Config::from_env();
    info!(
        bind_addr = %cfg.bind_addr,
        auth_service_base = %cfg.auth_service_base,
        project_service_base = %cfg.project_service_base,
        translation_core_grpc = %cfg.translation_core_grpc,
        "starting cats-bff"
    );

    // 注意：上游 URL 仅打印，不打印任何 secret / password（per 8/27 11:06 JST 守门 #5）

    let state = AppState::from_config(&cfg)
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, format!("app state init: {e}")))?;
    let state_data = web::Data::new(state);

    HttpServer::new(move || cats_bff::build_app(state_data.clone()))
        .bind(&cfg.bind_addr)?
        .run()
        .await
}