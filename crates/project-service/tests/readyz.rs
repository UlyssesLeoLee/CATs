//! project-service 的 `/readyz` 必须**在 DB 不可用时返回 503**
//!
//! # 为什么要单独钉这条
//!
//! 2026-10-07 之前，本服务只有 `/healthz`，而 k3s 的
//! `deploy/k3s/cats-core/project-service.yaml` 把 readinessProbe 指向了
//! `/healthz`。`/healthz` 恒返 200 且不查任何依赖 —— 于是**数据库挂了，
//! Pod 依然被判定 Ready，流量继续被派发进来**，而每个请求都会 500。
//!
//! 同类缺陷在 audit-service / worker-service 上真实发生过：它们的 `/readyz`
//! 一度把 `db: "fail"` 写进 body 却仍返回 **200**。k8s 的 readinessProbe
//! **只看状态码，不看 body**，所以那等于骗调度器。
//!
//! 这两条用例不需要真 PostgreSQL：`connect_lazy` 到一个必然拒绝连接的地址，
//! 查询立刻失败 —— 这正是"DB 不可用"的最短路径。
//!
//! 真正"DB 可用 → 200"那一半需要真 PG，标记 `#[ignore]`，由 CI 的
//! `e2e (real PostgreSQL)` job 执行。

use actix_web::{http::StatusCode, test as actix_test, web, App};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;
use std::sync::Arc;
use std::time::Duration;

use cats_rbac::RbacChecker;
use project_service::handlers;

/// 指向一个必然连不上的地址（127.0.0.1:1 没有监听者）。
///
/// `connect_lazy` 不会立刻建连，所以构造本身是同步且不会挂起；
/// 真正查询时才会失败。`acquire_timeout` 给测试封一个上界，避免极端
/// 情况下把套件挂住。
async fn unreachable_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_millis(500))
        .connect_lazy("postgres://nobody:nothing@127.0.0.1:1/nothing")
        .expect("connect_lazy 不建连，不应失败")
}

/// 装配与生产一致：`configure_routes` 收 pool + rbac 两个 app_data，
/// 与 `main.rs` 同形（`App::configure` 的单参形式套不上，用闭包）。
async fn call_readyz(pool: sqlx::PgPool) -> (StatusCode, Value) {
    let pool_data = web::Data::new(pool);
    let rbac_data = web::Data::new(Arc::new(RbacChecker::new()));
    let app =
        actix_test::init_service(App::new().configure(move |c| {
            handlers::configure_routes(c, pool_data.clone(), rbac_data.clone())
        }))
        .await;
    let req = actix_test::TestRequest::get().uri("/readyz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    let status = resp.status();
    let body: Value = actix_test::read_body_json(resp).await;
    (status, body)
}

#[actix_web::test]
async fn readyz_returns_503_when_db_is_unreachable() {
    let pool = unreachable_pool().await;
    let (status, body) = call_readyz(pool).await;

    assert_eq!(
        status,
        StatusCode::SERVICE_UNAVAILABLE,
        "DB 连不上时 readinessProbe 必须失败。把 503 改成 200 会让 k8s 在数据库 \
         故障期间继续往这个 Pod 派流量 —— 这就是 2026-10-07 修掉的那个缺陷"
    );
    assert_eq!(body["status"], "not_ready", "body 也必须说 not_ready");
    assert_eq!(body["db"], "fail");
    assert_eq!(body["service"], "project-service");
}

#[actix_web::test]
async fn readyz_shape_is_stable_and_shared_across_services() {
    let pool = unreachable_pool().await;
    let (_status, body) = call_readyz(pool).await;

    let obj = body.as_object().expect("顶层必须是对象");
    let mut keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec!["db", "service", "status"],
        "顶层键必须与 cats_common::ReadyResponse 一致，\
         否则各服务形状会漂移，运维看板就废了"
    );
}

/// `/healthz` 在同样条件下**仍必须 200** —— 存活与就绪是两回事。
///
/// 这条是反向用例：`/healthz` 查依赖会让 Pod 在数据库故障时被**重启**，
/// 而重启解决不了数据库的问题，只会把故障放大成 CrashLoop。
#[actix_web::test]
async fn healthz_stays_200_even_when_db_is_unreachable() {
    let pool = unreachable_pool().await;
    let pool_data = web::Data::new(pool);
    let rbac_data = web::Data::new(Arc::new(RbacChecker::new()));
    let app =
        actix_test::init_service(App::new().configure(move |c| {
            handlers::configure_routes(c, pool_data.clone(), rbac_data.clone())
        }))
        .await;
    let req = actix_test::TestRequest::get().uri("/healthz").to_request();
    let resp = actix_test::call_service(&app, req).await;
    assert_eq!(
        resp.status(),
        StatusCode::OK,
        "存活探针不查依赖：DB 挂了应该摘流量（/readyz 503），不该重启进程（/healthz）"
    );
}

/// DB 可用 → 200。需要真 PostgreSQL。
#[actix_web::test]
#[ignore = "需要真 PostgreSQL（CI 的 e2e (real PostgreSQL) job 会跑）"]
async fn readyz_returns_200_when_db_is_reachable() {
    let pool = project_service::db::build_pool()
        .await
        .expect("build_pool failed（检查 DATABASE_URL）");
    let (status, body) = call_readyz(pool).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ready");
    assert_eq!(body["db"], "ok");
}
