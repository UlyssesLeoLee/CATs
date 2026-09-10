# cats-mock

> CATs 测试 Mock 项目 — HTTP 路由 / DB fixture / 基础设施 / 业务数据 mock 工厂

**引用:** `doc/02-基础设计/测试/CATs_测试Mock项目设计书_v1.0.md`

## 4 大模块

| 模块 | 职责 | 典型用例 |
|------|------|----------|
| `cats_mock::data` | 业务对象 factory (User / Project / Task / AuditEvent) | 单测 / e2e 数据构造 |
| `cats_mock::db`   | PG 18.6 + pgvector schema/seed 工厂 (in-mem + pg stub) | 集成测试 DB 初始化 |
| `cats_mock::infra` | Kafka / Redis in-memory 替身 | worker / audit / notification 测试 |
| `cats_mock::http`  | actix-web 路由 + 响应工厂 + MockServer | e2e HTTP 端到端 |

## 用法

```rust
use cats_mock::{
    data::{UserFactory, ProjectFactory, TaskFactory, AuditEventFactory, Factory},
    db::{in_memory, SchemaSet, SeedSet},
    infra::{MockKafka, MockRedis},
    http::{MockServer, MockServerConfig},
};

#[tokio::test]
async fn e2e_user_create_login_audit() {
    // 1) 业务数据
    let user = UserFactory::new().with_username("alice").build();
    let project = ProjectFactory::new().owned_by(user.id).build();
    let task = TaskFactory::new().for_project(project.id).build();
    let audit = AuditEventFactory::new().by_user(user.id).of_type("login").build();

    // 2) DB fixture (in-memory)
    let db = in_memory();
    db.apply_schema(SchemaSet::all_common()).await.unwrap();
    db.apply_seed(SeedSet::from_user(&user)).await.unwrap();

    // 3) 基础设施 mock
    let kafka = MockKafka::new();
    let redis = MockRedis::new();

    // 4) HTTP server
    let server = MockServer::start(MockServerConfig::all()).await.unwrap();
    let base = server.base_url();

    // 5) 用 reqwest 跑 e2e
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/v1/auth/login"))
        .json(&serde_json::json!({"username": "alice", "password": "p"}))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status().as_u16(), 200);
}
```

## 路线图

- **v0.1 (M0):** 当前实现 — 4 大模块 in-memory 全部就位
- **v0.2 (M1):** 接入 `testcontainers`, 提供 `PgDbFixture` 真 PG 实现
- **v0.3 (M1-Sprint2):** 引入 `proptest` 做 property-based factory
- **v0.4 (M2):** 接入 K3s 真实 Kafka / Redis (per OI-3)
