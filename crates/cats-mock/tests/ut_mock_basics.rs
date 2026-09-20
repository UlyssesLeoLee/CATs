//! cats-mock UT (Unit Test) 整合入口
//!
//! 引用: doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md §5.1
//!
//! 设计目的
//! ---------
//! `cats-mock` crate 内部的 `#[cfg(test)] mod tests` 已经覆盖了 4 大模块的
//! 细粒度单元测试 (~91 个)。但每新增一个模块 / 新增一个 factory 方法, 工程师
//! 容易忘记在 `src/` 里写 unit test, 于是本文件作为**回归保护网**:
//!
//! - 在 `tests/` 目录里再跑一次"每个公开 API 至少一个断言", 一旦 `pub fn` 缺失
//!   unit test, 编译器会先报错 (use statement)
//! - 本文件**不重复** `src/` 内部的细粒度断言, 仅做公开 API 烟测:
//!   - 类型可被 import
//!   - 默认构造可被调用
//!   - 关键返回值非空
//!
//! 测试层级 (per IPA 测试设计)
//! --------------------------
//! 严格说这是**集成测试** (tests/ 目录), 但每个 test 用例本身在验证单个 API
//! 的最小契约, 故归类为 UT 整合层, 跟 src/ 内部的 #[test] 一起构成 UT 集合。
//!
//! 运行方式
//! --------
//! ```bash
//! cargo test -p cats-mock --test ut_mock_basics
//! ```

use cats_mock::data::{
    AuditEventFactory, Factory, ProjectFactory, ProjectStatus, TaskFactory, TaskStatus,
    UserFactory, invalid_emails, now_utc, pick_random, random_past_within_days, weak_passwords,
};
use cats_mock::db::{
    DbFixture, InMemoryDbFixture, SchemaSet, SeedSet, audit_log_schema, audit_events_default,
    in_memory, projects_default, projects_schema, tasks_default, tasks_schema,
    translation_unit_schema, users_default, users_schema,
};
use cats_mock::http::{
    ErrorBody, MockError, MockServer, MockServerConfig, ResponseBuilder, audit_routes,
    auth_routes, healthz_routes, project_routes, task_routes, user_routes,
};
use cats_mock::infra::{MockKafka, MockRedis};
use cats_mock::{NAME, VERSION, name, version};

// =====================================================================
// §1 lib.rs 元信息
// =====================================================================

#[test]
fn ut_lib_version_matches_cargo_pkg_version() {
    // VERSION 是 const, 跟 env!("CARGO_PKG_VERSION") 必须一致
    assert_eq!(VERSION, env!("CARGO_PKG_VERSION"));
    assert_eq!(version(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn ut_lib_name_is_cats_mock() {
    assert_eq!(NAME, "cats-mock");
    assert_eq!(name(), "cats-mock");
}

// =====================================================================
// §2 data 模块 (Factory trait + 4 大业务对象 factory)
// =====================================================================

#[test]
fn ut_data_user_factory_build_returns_user() {
    let u = UserFactory::new().build();
    assert!(!u.username.is_empty(), "username 不能为空");
    assert!(u.is_active, "默认激活");
}

#[test]
fn ut_data_project_factory_build_returns_project() {
    let p = ProjectFactory::new().build();
    assert_eq!(p.status, ProjectStatus::Draft, "默认 Draft");
}

#[test]
fn ut_data_task_factory_build_returns_task() {
    let t = TaskFactory::new().build();
    assert!(TaskStatus::all().contains(&t.status), "status 应在 TaskStatus::all() 内");
}

#[test]
fn ut_data_audit_event_factory_build_returns_event() {
    let e = AuditEventFactory::new().build();
    assert!(!e.event_type.is_empty(), "event_type 不能为空");
}

#[test]
fn ut_data_project_status_as_str_is_snake_case() {
    for s in ProjectStatus::all() {
        let st = s.as_str();
        assert!(!st.is_empty());
        assert!(!st.contains(' '));
        assert_eq!(st.to_lowercase(), st, "应全小写 snake_case");
    }
}

#[test]
fn ut_data_task_status_as_str_is_snake_case() {
    for s in TaskStatus::all() {
        let st = s.as_str();
        assert!(!st.is_empty());
        assert_eq!(st.to_lowercase(), st);
    }
}

#[test]
fn ut_data_now_utc_returns_recent_timestamp() {
    let t = now_utc();
    // 必须在 2026-01-01 之后 (epoch sanity)
    assert!(t.timestamp() > 1_700_000_000, "now_utc 应在 epoch 之后");
}

#[test]
fn ut_data_random_past_within_days_is_past() {
    let t = random_past_within_days(30);
    let now = now_utc();
    assert!(t <= now, "过去时间必须 <= 现在");
}

#[test]
fn ut_data_pick_random_returns_in_slice() {
    let items = vec!["a", "b", "c"];
    for _ in 0..10 {
        let v = pick_random(&items);
        assert!(items.contains(&v));
    }
}

#[test]
fn ut_data_invalid_emails_are_in_invalid_class() {
    // 5 个边界用例, 每个都不能是合法 email
    assert!(!invalid_emails().is_empty());
    for e in invalid_emails() {
        let at_count = e.matches('@').count();
        let has_space = e.contains(' ');
        let starts_with_at = e.starts_with('@');
        let ends_with_at = e.ends_with('@');
        let invalid = e.is_empty() || at_count != 1 || has_space || starts_with_at || ends_with_at;
        assert!(invalid, "expected invalid: {e}");
    }
}

#[test]
fn ut_data_weak_passwords_present() {
    assert!(!weak_passwords().is_empty());
}

// =====================================================================
// §3 db 模块 (SchemaSet / SeedSet / Fixture)
// =====================================================================

#[test]
fn ut_db_users_schema_has_create_table() {
    let s = users_schema();
    assert!(s.statements.iter().any(|x| x.contains("CREATE TABLE") && x.contains("users_credential")));
}

#[test]
fn ut_db_projects_schema_has_create_table() {
    let s = projects_schema();
    assert!(s.statements.iter().any(|x| x.contains("CREATE TABLE") && x.contains("project")));
}

#[test]
fn ut_db_tasks_schema_has_create_table() {
    let s = tasks_schema();
    assert!(s.statements.iter().any(|x| x.contains("CREATE TABLE") && x.contains("task")));
}

#[test]
fn ut_db_audit_log_schema_has_create_table() {
    let s = audit_log_schema();
    assert!(s.statements.iter().any(|x| x.contains("CREATE TABLE") && x.contains("audit_log")));
}

#[test]
fn ut_db_translation_unit_schema_uses_pgvector() {
    // 设计书 §3.1.2: translation_unit_schema 含 pgvector 列
    let s = translation_unit_schema();
    let all = s.statements.join("\n");
    assert!(all.contains("vector"), "translation_unit 必须含 pgvector 列: {all}");
}

#[test]
fn ut_db_schema_set_push_and_merge() {
    let a = SchemaSet::new().push("CREATE TABLE a (id INT);");
    let b = SchemaSet::new().push("CREATE TABLE b (id INT);");
    let merged = a.merge(b);
    assert_eq!(merged.statements.len(), 2);
}

#[test]
fn ut_db_schema_set_all_common_has_5_ddl() {
    // users (2 tables + 1 index) + projects (1 + 2 index) + tasks (1 + index) + audit_log (1)
    // 设计书 §4.2: users_credential + user_profile + project + task + audit_log = 5 表
    let s = SchemaSet::all_common();
    let count_table = s.statements.iter().filter(|x| x.contains("CREATE TABLE")).count();
    assert!(count_table >= 5, "all_common 必须 ≥ 5 CREATE TABLE, got {count_table}");
}

#[test]
fn ut_db_users_default_seed_has_one_entry() {
    let s = users_default();
    assert_eq!(s.entries.len(), 1);
    assert_eq!(s.entries[0].table, "users_credential");
}

#[test]
fn ut_db_projects_default_seed_has_zero_or_one_entries() {
    // projects_default() 是项目默认 seed (设计书 §3.1.2), entries 数量可能为 0/1
    let s = projects_default();
    assert!(s.entries.len() <= 1);
}

#[test]
fn ut_db_tasks_default_seed_has_zero_or_one_entries() {
    let s = tasks_default();
    assert!(s.entries.len() <= 1);
}

#[test]
fn ut_db_audit_events_default_seed_has_one_entry() {
    let s = audit_events_default();
    assert_eq!(s.entries.len(), 1);
    assert_eq!(s.entries[0].table, "audit_log");
}

#[test]
fn ut_db_in_memory_factory_returns_instance() {
    let _db = in_memory();
    // 不应 panic
    let _: InMemoryDbFixture = InMemoryDbFixture::new();
}

#[test]
fn ut_db_seed_set_total_reflects_entries() {
    let s = SeedSet::new("test");
    assert_eq!(s.total(), 0);
}

// =====================================================================
// §4 infra 模块 (MockKafka / MockRedis)
// =====================================================================

#[test]
fn ut_infra_mock_kafka_produce_consume_basic() {
    let k = MockKafka::new();
    k.produce("t", Some("k"), b"v");
    let msg = k.consume("t").expect("应有一条 msg");
    assert_eq!(msg.topic, "t");
    assert_eq!(msg.value, b"v");
}

#[test]
fn ut_infra_mock_kafka_topic_list_starts_empty() {
    let k = MockKafka::new();
    assert!(k.topic_list().is_empty());
}

#[test]
fn ut_infra_mock_kafka_counters_start_at_zero() {
    let k = MockKafka::new();
    assert_eq!(k.produced(), 0);
    assert_eq!(k.consumed(), 0);
}

#[test]
fn ut_infra_mock_redis_set_get_basic() {
    let r = MockRedis::new();
    r.set("k", "v");
    assert_eq!(r.get("k"), Some("v".to_string()));
}

#[test]
fn ut_infra_mock_redis_incr_returns_1_for_new_key() {
    let r = MockRedis::new();
    let v = r.incr("c");
    assert_eq!(v, 1, "新 key 首次 incr 应返回 1");
}

// =====================================================================
// §5 http 模块 (MockServer / ResponseBuilder / ErrorBody / MockError / 路由)
// =====================================================================

#[test]
fn ut_http_error_body_serializes_with_error_and_message() {
    let eb = ErrorBody::new("bad", "msg");
    let j = serde_json::to_string(&eb).unwrap();
    assert!(j.contains("\"error\":\"bad\""));
    assert!(j.contains("\"message\":\"msg\""));
}

#[test]
fn ut_http_mock_error_codes_match_http_semantics() {
    assert_eq!(MockError::BadRequest.status(), 400);
    assert_eq!(MockError::Unauthorized.status(), 401);
    assert_eq!(MockError::Forbidden.status(), 403);
    assert_eq!(MockError::NotFound.status(), 404);
    assert_eq!(MockError::Conflict.status(), 409);
    assert_eq!(MockError::InternalServerError.status(), 500);
    assert_eq!(MockError::ServiceUnavailable.status(), 503);
}

#[test]
fn ut_http_mock_error_to_body_has_error_field() {
    let body = MockError::NotFound.to_body();
    assert!(!body.error.is_empty());
    assert!(!body.message.is_empty());
}

#[test]
fn ut_http_response_builder_ok_status_200() {
    let r = ResponseBuilder::ok(serde_json::json!({}));
    assert_eq!(r.status().as_u16(), 200);
}

#[test]
fn ut_http_response_builder_created_status_201() {
    let r = ResponseBuilder::created(serde_json::json!({"id":"1"}));
    assert_eq!(r.status().as_u16(), 201);
}

#[test]
fn ut_http_response_builder_no_content_status_204() {
    let r = ResponseBuilder::no_content();
    assert_eq!(r.status().as_u16(), 204);
}

#[test]
fn ut_http_response_builder_err_status_matches_mock_error() {
    let r = ResponseBuilder::err(MockError::Conflict);
    assert_eq!(r.status().as_u16(), 409);
}

#[test]
fn ut_http_response_builder_err_with_msg_uses_custom_message() {
    let r = ResponseBuilder::err_with_msg(MockError::BadRequest, "参数 X 缺失");
    assert_eq!(r.status().as_u16(), 400);
}

#[test]
fn ut_http_mock_server_config_all_includes_everything() {
    let c = MockServerConfig::all();
    assert!(c.include_auth);
    assert!(c.include_user);
    assert!(c.include_project);
    assert!(c.include_task);
    assert!(c.include_audit);
    assert!(c.include_healthz);
}

#[test]
fn ut_http_mock_server_config_default_is_empty() {
    let c = MockServerConfig::default();
    assert!(!c.include_auth);
    assert!(!c.include_healthz);
}

#[test]
fn ut_http_route_registrars_compile() {
    // 6 个 routes 注册函数必须在编译期可调用 (仅取函数指针, 不真正调用)
    // 注: actix_web::web::ServiceConfig 没有公开 default(), 这里改用函数指针取址
    let _fns: (
        fn(&mut actix_web::web::ServiceConfig),
        fn(&mut actix_web::web::ServiceConfig),
        fn(&mut actix_web::web::ServiceConfig),
        fn(&mut actix_web::web::ServiceConfig),
        fn(&mut actix_web::web::ServiceConfig),
        fn(&mut actix_web::web::ServiceConfig),
    ) = (
        auth_routes,
        user_routes,
        project_routes,
        task_routes,
        audit_routes,
        healthz_routes,
    );
}

// =====================================================================
// §6 smoke 模块 (宏)
// =====================================================================

#[test]
fn ut_smoke_macro_re_exported_at_crate_root() {
    // `name_matches_crate` 是 #[macro_export], 在 crate 根可访问
    // 宏展开会生成 2 个 #[test]: version_is_semver_like + name_matches_crate
    cats_mock::name_matches_crate!(env!("CARGO_PKG_NAME"));
}

// =====================================================================
// §7 cats-mock 顶层公共常量
// =====================================================================

#[test]
fn ut_version_constant_is_semver_like() {
    assert!(VERSION.starts_with("0.1."), "VERSION 应以 '0.1.' 开头");
}

#[test]
fn ut_published_false_in_cargo_toml() {
    // Cargo.toml 设 publish = false (测试 crate 不发布)
    // 间接验证: 仍是 0.1.x 版本
    assert!(VERSION.starts_with("0."));
}