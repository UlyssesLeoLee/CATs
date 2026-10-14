//! `/healthz` 只能有一处注册 —— 本文件是这个约束的门禁。
//!
//! 背景：本 crate 曾经在 `src/main.rs` 里注册 `/healthz`，而 `api/mod.rs` 的
//! `configure_routes()` 里也注册了一次。actix-web 对同一路径注册两次时
//! **先注册的赢**，于是 `api::healthz_handler` 运行时永远收不到请求；但它自己
//! 的测试 `healthz_returns_ok` 单独 mount `configure_routes`、看不到 main.rs 的
//! 那次注册，所以一直是绿的 —— 一条假绿。
//!
//! 两个用例分别钉住这个事实的两半：
//! 1. `first_registration_wins` —— 钉住 actix 的语义（上面那句"先注册的赢"
//!    不是猜的，是这里实测的）。actix-web 若改语义，这个用例会失败，届时
//!    `src/main.rs` 里那条注释也要一起改。
//! 2. `main_rs_registers_healthz_nowhere` —— 钉住"main.rs 不再自己注册"，
//!    这样单一归属 `configure_routes` 不会被悄悄破坏。

use actix_web::{http::StatusCode, web, App, HttpResponse, Responder};

/// 钉住 actix-web 的重复路径语义：先注册的赢。
#[actix_web::test]
async fn first_registration_wins() {
    async fn registered_first() -> impl Responder {
        HttpResponse::build(StatusCode::OK).json(serde_json::json!({"from": "registered_first"}))
    }
    async fn registered_second() -> impl Responder {
        HttpResponse::build(StatusCode::OK).json(serde_json::json!({"from": "registered_second"}))
    }

    let app = actix_web::test::init_service(
        App::new()
            .route("/healthz", web::get().to(registered_first))
            .route("/healthz", web::get().to(registered_second)),
    )
    .await;
    let req = actix_web::test::TestRequest::get()
        .uri("/healthz")
        .to_request();
    let body: serde_json::Value = actix_web::test::call_and_read_body_json(&app, req).await;

    assert_eq!(
        body.get("from").and_then(|v| v.as_str()),
        Some("registered_first"),
        "actix-web 的重复路径语义变了：现在不是先注册的赢。若 main.rs 重新自己注册 \
         /healthz，请一并复核 crates/cats-ai-gateway/src/main.rs 里关于本语义的注释。实际 body={body}"
    );
}

/// 钉住"`/healthz` 的唯一归属是 `api::configure_routes()`"。
#[test]
fn main_rs_registers_healthz_nowhere() {
    let main_rs = include_str!("../src/main.rs");
    // 只看代码，不看注释 —— main.rs 里关于本约束的注释本身就写着 `/healthz`，
    // 按整行扫会把注释当成注册。行尾 `//` 之后的全部丢掉。
    let offenders: Vec<String> = main_rs
        .lines()
        .enumerate()
        .filter_map(|(i, l)| {
            let code = l.split("//").next().unwrap_or("");
            if code.contains("/healthz") {
                Some(format!("src/main.rs:{}: {}", i + 1, l.trim()))
            } else {
                None
            }
        })
        .collect();
    assert!(
        offenders.is_empty(),
        "src/main.rs 不应再出现 /healthz —— 它由 api::configure_routes() 注册。\n\
         再加一次就是重复注册，而先注册的赢意味着其中一半永远收不到请求。\n\
         命中行：\n  {}",
        offenders.join("\n  ")
    );
}

/// `configure_routes()` 确实拥有 `/healthz`（上面那条删的是"另一处"，
/// 不是"唯一那处"）—— 防止把约束修过头。
#[actix_web::test]
async fn configure_routes_serves_healthz() {
    let app =
        actix_web::test::init_service(App::new().configure(cats_ai_gateway::api::configure_routes))
            .await;
    let req = actix_web::test::TestRequest::get()
        .uri("/healthz")
        .to_request();
    let body: serde_json::Value = actix_web::test::call_and_read_body_json(&app, req).await;

    assert_eq!(
        body.get("status").and_then(|v| v.as_str()),
        Some("ok"),
        "configure_routes() 必须仍然注册 /healthz；实际 body={body}"
    );
    assert_eq!(
        body.pointer("/app/name").and_then(|v| v.as_str()),
        Some(env!("CARGO_PKG_NAME")),
        "app.name 必须是本 crate 的包名，不是 cats-common 的（AppMeta::current() 的坑）。实际 body={body}"
    );
}
