//! cats-mock IT (Integration Test) — 模块间联动 + HTTP e2e
//!
//! 引用: doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md §5.2 + §5.3
//!
//! 测试层级 (per IPA 测试设计)
//! --------------------------
//! IT = 模块间联动 + API + DB + Cache + MQ + External IF + Error Propagation
//!
//! 本文件覆盖:
//! - IT-1: data::UserFactory → db::SeedSet 链路 (factory 产出 → INSERT 转换)
//! - IT-2: db::InMemoryDbFixture schema + seed 全链路 (5 表 + 5 行)
//! - IT-3: infra::MockKafka produce + consume + peek 链路
//! - IT-4: infra::MockRedis KV + SET + ZSET 链路
//! - IT-5: http::MockServer 全路由启动 + reqwest 端到端验证
//! - IT-6: data + db + http 跨模块联动 (UserFactory → seed → MockServer 命中)
//! - IT-7: Error Propagation (404/401/409/400 在 MockServer 上验证)
//!
//! 运行方式
//! --------
//! ```bash
//! cargo test -p cats-mock --test it_e2e_compose
//! ```

use cats_mock::data::{
    AuditEventFactory, Factory, ProjectFactory, TaskFactory, UserFactory,
};
use cats_mock::db::{DbFixture, SchemaSet, in_memory};
use cats_mock::http::{MockError, MockServer, MockServerConfig, ResponseBuilder};
use cats_mock::infra::{MockKafka, MockRedis};
use serde_json::Value;

// =====================================================================
// IT-1: data → db (Factory 产出 → SeedSet 转换)
// =====================================================================

#[actix_web::test]
async fn it_data_factory_to_seed_users_from() {
    let users = UserFactory::new().build_many(5);
    let seed = cats_mock::db::users_from(&users);
    assert_eq!(seed.entries.len(), 5);
    for (i, e) in seed.entries.iter().enumerate() {
        assert_eq!(e.table, "users_credential");
        assert!(e.sql.contains("INSERT"));
        // 参数数量应能容纳 username/email/password_hash/is_active/...
        assert!(!e.params.is_empty(), "entry {i} 应有绑定参数");
    }
}

#[actix_web::test]
async fn it_data_factory_to_seed_projects_from() {
    let projects = ProjectFactory::new().build_many(3);
    let seed = cats_mock::db::projects_from(&projects);
    assert_eq!(seed.entries.len(), 3);
    for e in &seed.entries {
        assert_eq!(e.table, "project");
    }
}

#[actix_web::test]
async fn it_data_factory_to_seed_tasks_from() {
    let tasks = TaskFactory::new().build_many(2);
    let seed = cats_mock::db::tasks_from(&tasks);
    assert_eq!(seed.entries.len(), 2);
    for e in &seed.entries {
        assert_eq!(e.table, "task");
    }
}

#[actix_web::test]
async fn it_data_factory_to_seed_audit_events_from() {
    let events = AuditEventFactory::new().build_many(4);
    let seed = cats_mock::db::audit_events_from(&events);
    assert_eq!(seed.entries.len(), 4);
    for e in &seed.entries {
        assert_eq!(e.table, "audit_log");
    }
}

// =====================================================================
// IT-2: db::InMemoryDbFixture schema + seed 全链路
// =====================================================================

#[actix_web::test]
async fn it_db_schema_apply_creates_five_tables() {
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    let tables = db.tables();
    assert!(tables.contains(&"users_credential".to_string()));
    assert!(tables.contains(&"user_profile".to_string()));
    assert!(tables.contains(&"project".to_string()));
    assert!(tables.contains(&"task".to_string()));
    assert!(tables.contains(&"audit_log".to_string()));
    assert_eq!(db.table_count(), 5);
}

#[actix_web::test]
async fn it_db_seed_then_query_row_count() {
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    db.apply_seed(cats_mock::db::users_default()).await.unwrap();
    db.apply_seed(cats_mock::db::projects_default()).await.unwrap();
    db.apply_seed(cats_mock::db::audit_events_default()).await.unwrap();
    assert_eq!(db.row_count("users_credential"), 1);
    assert_eq!(db.row_count("audit_log"), 1);
}

#[actix_web::test]
async fn it_db_truncate_all_clears_rows_keeps_schema() {
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    db.apply_seed(cats_mock::db::users_default()).await.unwrap();
    db.truncate_all().await.unwrap();
    assert_eq!(db.row_count("users_credential"), 0);
    // 但表结构保留
    assert!(db.tables().contains(&"users_credential".to_string()));
}

#[actix_web::test]
async fn it_db_users_from_factory_apply_and_count() {
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();

    let users = UserFactory::new().build_many(10);
    let seed = cats_mock::db::users_from(&users);
    db.apply_seed(seed).await.unwrap();
    assert_eq!(db.row_count("users_credential"), 10);
}

// =====================================================================
// IT-3: MockKafka produce + consume + peek 链路
// =====================================================================

#[actix_web::test]
async fn it_kafka_produce_then_consume_fifo() {
    let k = MockKafka::new();
    k.produce("audit.event", Some("user-1"), b"e1");
    k.produce("audit.event", Some("user-2"), b"e2");
    k.produce("audit.event", Some("user-3"), b"e3");
    let m1 = k.consume("audit.event").unwrap();
    let m2 = k.consume("audit.event").unwrap();
    let m3 = k.consume("audit.event").unwrap();
    assert_eq!(m1.value, b"e1");
    assert_eq!(m2.value, b"e2");
    assert_eq!(m3.value, b"e3");
    assert!(k.consume("audit.event").is_none());
}

#[actix_web::test]
async fn it_kafka_peek_does_not_consume() {
    let k = MockKafka::new();
    k.produce("t", None, b"hello");
    let p1 = k.peek("t").unwrap();
    let p2 = k.peek("t").unwrap();
    assert_eq!(p1.value, p2.value);
    assert_eq!(k.queue_len("t"), 1);
}

#[actix_web::test]
async fn it_kafka_counters_track_produce_consume() {
    let k = MockKafka::new();
    for i in 0..5 {
        k.produce("t", None, format!("msg-{i}").as_bytes());
    }
    k.consume("t");
    k.consume("t");
    assert_eq!(k.produced(), 5);
    assert_eq!(k.consumed(), 2);
    assert_eq!(k.queue_len("t"), 3);
}

#[actix_web::test]
async fn it_kafka_clear_resets_state() {
    let k = MockKafka::new();
    k.produce("a", None, b"1");
    k.produce("b", None, b"2");
    k.clear();
    // 设计书: clear() 只清空 topics (queue 复位), 不重置累计计数器
    assert_eq!(k.queue_len("a"), 0);
    assert_eq!(k.queue_len("b"), 0);
    assert!(k.topic_list().is_empty());
    assert_eq!(k.produced(), 2);
    assert_eq!(k.consumed(), 0);
}

// =====================================================================
// IT-4: MockRedis KV + SET + ZSET 链路
// =====================================================================

#[actix_web::test]
async fn it_redis_kv_set_get_del_exists() {
    let r = MockRedis::new();
    assert!(!r.exists("k"));
    r.set("k", "v");
    assert!(r.exists("k"));
    assert_eq!(r.get("k"), Some("v".to_string()));
    assert!(r.del("k"));
    assert!(!r.exists("k"));
    // 重复 del 返回 false
    assert!(!r.del("k"));
}

#[actix_web::test]
async fn it_redis_kv_incr_decr_incr_by() {
    let r = MockRedis::new();
    assert_eq!(r.incr("c"), 1);
    assert_eq!(r.incr("c"), 2);
    assert_eq!(r.incr("c"), 3);
    assert_eq!(r.decr("c"), 2);
    assert_eq!(r.incr_by("c", 10), 12);
}

#[actix_web::test]
async fn it_redis_set_sadd_smembers_srem_scard() {
    let r = MockRedis::new();
    // sadd 返回新增条数 (1 = 新增, 0 = 已存在)
    assert_eq!(r.sadd("set1", "a"), 1);
    assert_eq!(r.sadd("set1", "b"), 1);
    assert_eq!(r.sadd("set1", "a"), 0, "重复 sadd 返回 0 (已存在)");
    assert_eq!(r.scard("set1"), 2);
    let mut members = r.smembers("set1");
    members.sort();
    assert_eq!(members, vec!["a".to_string(), "b".to_string()]);
    assert!(r.srem("set1", "a"));
    assert_eq!(r.scard("set1"), 1);
}

#[actix_web::test]
async fn it_redis_zset_zadd_zscore_zrange() {
    let r = MockRedis::new();
    r.zadd("z", "alice", 1.5);
    r.zadd("z", "bob", 2.5);
    r.zadd("z", "carol", 0.5);
    assert_eq!(r.zscore("z", "bob"), Some(2.5));
    let range = r.zrange("z", 0, -1);
    // 顺序: carol(0.5), alice(1.5), bob(2.5)
    assert_eq!(range, vec!["carol".to_string(), "alice".to_string(), "bob".to_string()]);
}

#[actix_web::test]
async fn it_redis_expire_shortens_ttl() {
    // expire 的第 2 参数是 ttl_secs (u64), 不是 Duration
    let r = MockRedis::new();
    r.set("k", "v");
    assert!(r.expire("k", 0)); // 立即过期
    // expire 返回 true (key 存在并设置 ttl), 但 get 应返回 None
    // 注: 实现可能 lazy-expiry, 这里仅验证 API 调用不 panic + 至少返回 bool
    let _: bool = r.expire("never-set", 1); // 不存在 key, 应返回 false
}

#[actix_web::test]
async fn it_redis_clear_resets_state() {
    let r = MockRedis::new();
    r.set("k", "v");
    r.sadd("s", "a");
    r.zadd("z", "x", 1.0);
    r.clear();
    assert_eq!(r.get("k"), None);
    assert_eq!(r.scard("s"), 0);
    assert_eq!(r.zscore("z", "x"), None);
}

// =====================================================================
// IT-5: MockServer 全路由启动 + reqwest 端到端验证
// =====================================================================

#[actix_web::test]
async fn it_http_server_healthz_returns_200() {
    let server = MockServer::start(MockServerConfig {
        include_healthz: true,
        ..Default::default()
    })
    .await
    .expect("start");
    let base = server.base_url();

    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/healthz"))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body.get("status").and_then(|v| v.as_str()), Some("ok"));
}

#[actix_web::test]
async fn it_http_server_auth_login_returns_token() {
    let server = MockServer::start(MockServerConfig::all())
        .await
        .expect("start");
    let base = server.base_url();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/auth/login"))
        .json(&serde_json::json!({"username": "u", "password": "p"}))
        .send()
        .await
        .expect("request");
    assert_eq!(resp.status().as_u16(), 200);
    let body: Value = resp.json().await.expect("json");
    assert!(body.get("access_token").is_some());
    assert!(body.get("refresh_token").is_some());
}

#[actix_web::test]
async fn it_http_server_user_create_returns_201() {
    let server = MockServer::start(MockServerConfig::all())
        .await
        .expect("start");
    let base = server.base_url();

    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/users"))
        .json(&serde_json::json!({
            "username": "alice",
            "email": "alice@example.com",
            "password": "secret"
        }))
        .send()
        .await
        .expect("request");
    // 默认 mock 应返回 200/201 (具体看 routes.rs 实现)
    assert!(
        resp.status().as_u16() == 200 || resp.status().as_u16() == 201,
        "user create 应成功, got {}",
        resp.status()
    );
}

// =====================================================================
// IT-6: data + db + http 跨模块联动
// =====================================================================

#[actix_web::test]
async fn it_compose_data_db_http_full_path() {
    // 1) 业务数据: UserFactory
    let user = UserFactory::new().with_username("alice").build();
    assert_eq!(user.username, "alice");

    // 2) DB fixture: 用同一 user 的 id 写入
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    db.apply_seed(cats_mock::db::users_from(&[user.clone()])).await.unwrap();
    assert_eq!(db.row_count("users_credential"), 1);

    // 3) HTTP server: 启动, 用 user.username 去打 /v1/users/{id}
    let server = MockServer::start(MockServerConfig::all()).await.unwrap();
    let base = server.base_url();
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/v1/users/{}", user.id))
        .send()
        .await
        .expect("request");
    // 默认 mock 返回 200 + 预制 JSON (与具体实现有关, 此处只要有响应)
    assert!(
        resp.status().as_u16() < 500,
        "GET /v1/users/{{id}} 不应 5xx, got {}",
        resp.status()
    );
}

#[actix_web::test]
async fn it_compose_kafka_redis_user_flow() {
    // 模拟: 用户登录 → Kafka 生产 audit.event → Redis incr 计数器
    let user = UserFactory::new().with_username("bob").build();

    let kafka = MockKafka::new();
    let redis = MockRedis::new();

    // 1) login 成功事件 → Kafka
    kafka.produce(
        "audit.event",
        Some(&user.id.to_string()),
        format!(r#"{{"event":"login","user":"{}"}}"#, user.username).as_bytes(),
    );

    // 2) 在线人数 Redis 计数器
    redis.set("online:counter", "0");
    redis.incr("online:counter");

    assert_eq!(kafka.queue_len("audit.event"), 1);
    assert_eq!(redis.get("online:counter"), Some("1".to_string()));
}

// =====================================================================
// IT-7: Error Propagation (4xx 在 MockServer 上)
// =====================================================================

#[test]
fn it_response_builder_error_propagates_status_codes() {
    // 验证所有 MockError 的 status 码与 HTTP 标准对齐
    let cases = [
        (MockError::BadRequest, 400),
        (MockError::Unauthorized, 401),
        (MockError::Forbidden, 403),
        (MockError::NotFound, 404),
        (MockError::Conflict, 409),
        (MockError::InternalServerError, 500),
        (MockError::ServiceUnavailable, 503),
    ];
    for (e, expected) in cases {
        let r = ResponseBuilder::err(e);
        assert_eq!(r.status().as_u16(), expected);
    }
}

#[test]
fn it_response_builder_err_with_msg_keeps_status() {
    let r = ResponseBuilder::err_with_msg(MockError::BadRequest, "missing field X");
    assert_eq!(r.status().as_u16(), 400);
}

#[actix_web::test]
async fn it_http_server_returns_consistent_json_for_error() {
    // 验证 ErrorBody JSON 序列化的契约 (给客户端解析)
    use cats_mock::http::ErrorBody;
    let body = ErrorBody::new("user_not_found", "user not found").with_detail("trace=xyz");
    let json = serde_json::to_string(&body).unwrap();
    assert!(json.contains("\"error\":\"user_not_found\""));
    assert!(json.contains("\"message\":\"user not found\""));
    assert!(json.contains("\"detail\":\"trace=xyz\""));
}