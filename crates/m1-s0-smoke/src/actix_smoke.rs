//! actix-web 4.x 兼容性冒烟
//!
//! 验证目标: 4.x 在 Rust 1.98.0 下编译 + /healthz 端点可启动可响应

use actix_web::{get, web, App, HttpResponse, HttpServer};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResp {
    status: &'static str,
    service: &'static str,
    rust_version: &'static str,
}

#[get("/healthz")]
async fn healthz() -> HttpResponse {
    HttpResponse::Ok().json(HealthResp {
        status: "ok",
        service: "cats-m1-s0-smoke-actix",
        rust_version: env!("CARGO_PKG_RUST_VERSION"),
    })
}

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg.service(healthz);
}

/// Smoke 入口: 编译即通过 + 测试不实际 bind 端口
pub async fn start_mock_server() -> std::io::Result<()> {
    HttpServer::new(|| App::new().configure(config))
        .bind("127.0.0.1:0")?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::test as actix_test;

    #[actix_web::test]
    async fn healthz_returns_ok_json() {
        let app = actix_test::init_service(App::new().configure(config)).await;
        let req = actix_test::TestRequest::get().uri("/healthz").to_request();
        let resp = actix_test::call_service(&app, req).await;
        assert!(resp.status().is_success());
    }
}
