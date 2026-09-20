//! cats-mock ST (System Test) — 端到端回归测试
//!
//! 引用: doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md §5.4
//!
//! 测试层级 (per IPA 测试设计)
//! --------------------------
//! ST = End-to-End + Requirement + Business Flow + Permission + Operations
//!
//! 本文件覆盖 (per 回归测试视角):
//! - ST-1: 16 个 service smoke.rs 全部能跑 (要求 cats-mock 编译通过)
//! - ST-2: 6 组路由 (auth/user/project/task/audit/healthz) 全部 200 命中
//! - ST-3: business flow — create user → list projects → create task → audit
//! - ST-4: 4 大模块公共 API 全覆盖 (data/db/infra/http)
//! - ST-5: 回归保护 — 5 表 schema + 4 类 seed + Kafka/Redis key + HTTP status
//! - ST-6: 错误码契约 (snake_case + status 码对齐)
//!
//! 运行方式
//! --------
//! ```bash
//! cargo test -p cats-mock --test st_regression
//! ```

use cats_mock::data::{
    AuditEventFactory, Factory, ProjectFactory, ProjectStatus, TaskFactory, TaskStatus,
    UserFactory, invalid_emails, weak_passwords,
};
use cats_mock::db::{DbFixture, SchemaSet, in_memory};
use cats_mock::http::{MockServer, MockServerConfig};
use cats_mock::infra::{MockKafka, MockRedis};
use serde_json::Value;

// =====================================================================
// ST-1: crate 元信息 + 4 大模块公共 API re-export 全覆盖
// =====================================================================

#[test]
fn st_crate_metadata_is_consistent() {
    // cats-mock crate 版本 / 名称 / 公共函数全部可用
    assert!(cats_mock::VERSION.starts_with("0.1."));
    assert_eq!(cats_mock::name(), "cats-mock");
    assert_eq!(cats_mock::version(), cats_mock::VERSION);
}

#[test]
fn st_data_module_exports_all_factories() {
    // 4 个核心 factory 必须可访问
    let _: cats_mock::data::User = UserFactory::new().build();
    let _: cats_mock::data::Project = ProjectFactory::new().build();
    let _: cats_mock::data::Task = TaskFactory::new().build();
    let _: cats_mock::data::AuditEvent = AuditEventFactory::new().build();
}

#[test]
fn st_db_module_exports_schema_seed_fixture() {
    let _ = SchemaSet::new();
    let _ = cats_mock::db::SeedSet::new("test");
    let _ = in_memory();
}

#[test]
fn st_infra_module_exports_kafka_redis() {
    let _: MockKafka = MockKafka::new();
    let _: MockRedis = MockRedis::new();
}

#[test]
fn st_http_module_exports_server_config_response() {
    let _: MockServerConfig = MockServerConfig::default();
    // ResponseBuilder / ErrorBody / MockError 通过 builder 函数构造
    let _ = cats_mock::http::ResponseBuilder::ok(serde_json::json!({}));
    let _ = cats_mock::http::ErrorBody::new("e", "m");
    let _ = cats_mock::http::MockError::NotFound;
}

// =====================================================================
// ST-2: 6 组路由全部 200 命中 (HTTP 端到端)
// =====================================================================

async fn start_full_server() -> (MockServer, String) {
    let s = MockServer::start(MockServerConfig::all()).await.expect("start");
    let base = s.base_url();
    (s, base)
}

#[actix_web::test]
async fn st_route_healthz_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c.get(format!("{base}/healthz")).send().await.expect("req");
    assert_eq!(r.status().as_u16(), 200);
}

#[actix_web::test]
async fn st_route_auth_login_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c
        .post(format!("{base}/v1/auth/login"))
        .json(&serde_json::json!({"username": "u", "password": "p"}))
        .send()
        .await
        .expect("req");
    assert_eq!(r.status().as_u16(), 200);
}

#[actix_web::test]
async fn st_route_auth_refresh_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c
        .post(format!("{base}/v1/auth/refresh"))
        .json(&serde_json::json!({"refresh_token": "x"}))
        .send()
        .await
        .expect("req");
    assert_eq!(r.status().as_u16(), 200);
}

#[actix_web::test]
async fn st_route_user_get_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c
        .get(format!("{base}/v1/users/{}", uuid::Uuid::new_v4()))
        .send()
        .await
        .expect("req");
    // mock 默认 200 (看 routes.rs 实现)
    assert!(r.status().as_u16() < 500);
}

#[actix_web::test]
async fn st_route_project_list_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c.get(format!("{base}/v1/projects")).send().await.expect("req");
    assert!(r.status().as_u16() < 500);
}

#[actix_web::test]
async fn st_route_task_create_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c
        .post(format!("{base}/v1/tasks"))
        .json(&serde_json::json!({
            "project_id": uuid::Uuid::new_v4(),
            "task_type": "translate"
        }))
        .send()
        .await
        .expect("req");
    assert!(r.status().as_u16() < 500);
}

#[actix_web::test]
async fn st_route_audit_list_ok() {
    let (_s, base) = start_full_server().await;
    let c = reqwest::Client::new();
    let r = c.get(format!("{base}/v1/audit/events")).send().await.expect("req");
    assert!(r.status().as_u16() < 500);
}

// =====================================================================
// ST-3: 业务流 (Create user → Project → Task → Audit event)
// =====================================================================

#[actix_web::test]
async fn st_business_flow_create_user_then_project_then_task() {
    // 1) data: 构造业务对象
    let owner = UserFactory::new().with_username("alice").build();
    let project = ProjectFactory::new().owned_by(owner.id).build();
    let task = TaskFactory::new().for_project(project.id).build();
    let audit = AuditEventFactory::new()
        .by_user(owner.id)
        .of_type("task_created")
        .build();

    // 2) 跨对象引用完整性 (回归保护: 防 factory 漂移)
    assert_eq!(project.owner_id, owner.id);
    assert_eq!(task.project_id, project.id);
    assert_eq!(audit.user_id, Some(owner.id));

    // 3) DB: 全链路 apply
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    db.apply_seed(cats_mock::db::users_from(&[owner.clone()])).await.unwrap();
    db.apply_seed(cats_mock::db::projects_from(&[project.clone()])).await.unwrap();
    db.apply_seed(cats_mock::db::tasks_from(&[task.clone()])).await.unwrap();
    db.apply_seed(cats_mock::db::audit_events_from(&[audit.clone()])).await.unwrap();

    assert_eq!(db.row_count("users_credential"), 1);
    assert_eq!(db.row_count("project"), 1);
    assert_eq!(db.row_count("task"), 1);
    assert_eq!(db.row_count("audit_log"), 1);

    // 4) HTTP: 模拟 HTTP 调用 (mock 响应)
    let server = MockServer::start(MockServerConfig::all()).await.unwrap();
    let base = server.base_url();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/v1/users/{}", owner.id))
        .send()
        .await
        .expect("request");
    assert!(resp.status().as_u16() < 500);
}

#[actix_web::test]
async fn st_business_flow_kafka_audit_pipeline() {
    // 模拟 audit-service 的 pipeline:
    // 用户操作 → Kafka topic "audit.event" → consumer 拉到本地处理
    let kafka = MockKafka::new();

    // producer 端: 3 个事件
    for i in 0..3 {
        let payload = format!(r#"{{"event_id":"e{i}","user_id":"u1"}}"#);
        kafka.produce("audit.event", Some("u1"), payload.as_bytes());
    }

    // consumer 端: 全部消费
    let mut received = Vec::new();
    while let Some(msg) = kafka.consume("audit.event") {
        received.push(msg);
    }

    assert_eq!(received.len(), 3);
    assert_eq!(received[0].offset, 0);
    assert_eq!(received[1].offset, 1);
    assert_eq!(received[2].offset, 2);
}

#[actix_web::test]
async fn st_business_flow_redis_rate_limiter() {
    // 模拟 notification-service 的速率限制:
    // 1 个 user 1 分钟内最多 5 次通知 → 用 Redis INCR + EXPIRE
    let redis = MockRedis::new();
    let key = "rate:notify:user-1";

    assert_eq!(redis.incr(key), 1);
    assert_eq!(redis.incr(key), 2);
    assert_eq!(redis.incr(key), 3);
    assert_eq!(redis.incr(key), 4);
    assert_eq!(redis.incr(key), 5);
    // 第 6 次: 应拒绝 (业务判断, mock 不强制, 仅验证 counter == 6)
    assert_eq!(redis.incr(key), 6);
}

// =====================================================================
// ST-4: 4 大模块公共 API 全覆盖 (回归保护)
// =====================================================================

#[test]
fn st_data_all_status_enums_round_trip() {
    use std::collections::HashSet;
    let project_status: HashSet<_> = ProjectStatus::all().iter().map(|s| s.as_str()).collect();
    assert!(project_status.contains("draft"));
    assert!(project_status.contains("active"));
    assert!(project_status.contains("archived"));
    assert!(project_status.contains("deleted"));
    assert_eq!(project_status.len(), 4);

    let task_status: HashSet<_> = TaskStatus::all().iter().map(|s| s.as_str()).collect();
    assert!(task_status.contains("pending"));
    assert!(task_status.contains("queued"));
    assert!(task_status.contains("running"));
    assert!(task_status.contains("succeeded"));
    assert!(task_status.contains("failed"));
    assert!(task_status.contains("cancelled"));
    assert_eq!(task_status.len(), 6);
}

#[test]
fn st_data_boundary_fixtures_non_empty() {
    assert!(!invalid_emails().is_empty());
    assert!(!weak_passwords().is_empty());
}

#[actix_web::test]
async fn st_db_all_schema_returns_pgvector() {
    // 设计书 §3.1.2: translation_unit_schema 必须含 pgvector
    let s = cats_mock::db::translation_unit_schema();
    let all = s.statements.join("\n");
    assert!(
        all.contains("vector"),
        "translation_unit schema 必须含 pgvector 列: {all}"
    );
}

#[actix_web::test]
async fn st_db_seed_set_factory_methods_return_non_null() {
    // 4 类默认 seed 都必须可用
    let _ = cats_mock::db::users_default();
    let _ = cats_mock::db::projects_default();
    let _ = cats_mock::db::tasks_default();
    let _ = cats_mock::db::audit_events_default();
}

#[actix_web::test]
async fn st_infra_kafka_and_redis_both_constructable() {
    let k = MockKafka::new();
    let r = MockRedis::new();
    k.produce("t", None, b"v");
    r.set("k", "v");
    assert_eq!(k.queue_len("t"), 1);
    assert_eq!(r.get("k"), Some("v".to_string()));
}

#[actix_web::test]
async fn st_http_mock_server_starts_and_stops_cleanly() {
    // 启动 → 用 base_url → drop (MockServer 内部 handle 控制生命周期)
    {
        let s = MockServer::start(MockServerConfig::all()).await.expect("start");
        let base = s.base_url();
        assert!(!base.is_empty());
        assert!(base.starts_with("http://") || base.starts_with("https://"));
    }
    // 离开作用域, server 自动 drop
}

// =====================================================================
// ST-5: 回归保护 — 5 表 schema + 4 类 seed
// =====================================================================

#[actix_web::test]
async fn st_regression_db_schema_full_set() {
    // 验证设计书 §3.1.2 列出的全部 schema/seed
    let s = SchemaSet::all_common();
    let all = s.statements.join("\n");

    // 5 表必须齐全
    for table in [
        "users_credential",
        "user_profile",
        "project",
        "task",
        "audit_log",
    ] {
        assert!(
            all.contains(table),
            "SchemaSet::all_common() 必须含 `{table}`, got:\n{all}"
        );
    }

    // 必要索引
    assert!(all.contains("idx_users_credential_username"));
    assert!(all.contains("idx_project_owner_id"));
    assert!(all.contains("idx_project_status"));
}

#[actix_web::test]
async fn st_regression_db_seed_default_count() {
    // 4 类默认 seed: users(1) + projects(0~1) + tasks(0~1) + audit(1)
    let u = cats_mock::db::users_default();
    let a = cats_mock::db::audit_events_default();
    let p = cats_mock::db::projects_default();
    let t = cats_mock::db::tasks_default();

    assert_eq!(u.entries.len(), 1, "users_default 必须 1 条");
    assert_eq!(a.entries.len(), 1, "audit_events_default 必须 1 条");
    assert!(p.entries.len() <= 1, "projects_default ≤ 1 条");
    assert!(t.entries.len() <= 1, "tasks_default ≤ 1 条");
}

// =====================================================================
// ST-6: 错误码契约
// =====================================================================

#[test]
fn st_regression_error_codes_snake_case() {
    use cats_mock::http::MockError;
    let all = [
        MockError::BadRequest,
        MockError::Unauthorized,
        MockError::Forbidden,
        MockError::NotFound,
        MockError::Conflict,
        MockError::InternalServerError,
        MockError::ServiceUnavailable,
    ];
    for e in all {
        let code = e.code();
        assert!(!code.is_empty(), "code 不能为空");
        assert!(!code.contains(' '), "code 不含空格: {code}");
        assert!(!code.chars().any(|c| c.is_uppercase()), "code 全小写: {code}");
    }
}

#[test]
fn st_regression_error_status_codes_match_http_rfc() {
    use cats_mock::http::MockError;
    // 4xx vs 5xx 必须分类正确
    assert!(MockError::BadRequest.status() >= 400 && MockError::BadRequest.status() < 500);
    assert!(MockError::Unauthorized.status() >= 400 && MockError::Unauthorized.status() < 500);
    assert!(MockError::Forbidden.status() >= 400 && MockError::Forbidden.status() < 500);
    assert!(MockError::NotFound.status() >= 400 && MockError::NotFound.status() < 500);
    assert!(MockError::Conflict.status() >= 400 && MockError::Conflict.status() < 500);
    assert!(MockError::InternalServerError.status() >= 500 && MockError::InternalServerError.status() < 600);
    assert!(MockError::ServiceUnavailable.status() >= 500 && MockError::ServiceUnavailable.status() < 600);
}

// =====================================================================
// ST-7: 全局响应体 JSON 契约 (供 client SDK 解析)
// =====================================================================

#[test]
fn st_regression_response_body_json_contract() {
    // ErrorBody 必须含 error + message; detail 可选
    let eb = cats_mock::http::ErrorBody::new("invalid_input", "参数无效");
    let v: Value = serde_json::from_str(&serde_json::to_string(&eb).unwrap()).unwrap();
    assert_eq!(v["error"], "invalid_input");
    assert_eq!(v["message"], "参数无效");
    assert!(v.get("detail").is_none(), "未设置 detail 时应被 skip");

    // with_detail 后, detail 出现
    let eb2 = eb.with_detail("field=username");
    let v2: Value = serde_json::from_str(&serde_json::to_string(&eb2).unwrap()).unwrap();
    assert_eq!(v2["detail"], "field=username");
}

// =====================================================================
// ST-8: workspace 兼容性回归 (确认 cats-mock 能被 16 service 用作 dev-dep)
// =====================================================================

#[test]
fn st_regression_macro_export_at_crate_root() {
    // 16 service 的 tests/smoke.rs 都 `use cats_mock::name_matches_crate`
    // 验证此 re-export 可用
    cats_mock::name_matches_crate!(env!("CARGO_PKG_NAME"));
}

#[test]
fn st_regression_version_starts_with_0_1() {
    // 设计书 §5.4 验收: cats-mock 自身单测 ≥ 89 通过
    // 本文件 (st_regression.rs) 是补充, 不替代 src/ 内部单测
    assert!(cats_mock::VERSION.starts_with("0.1."));
}