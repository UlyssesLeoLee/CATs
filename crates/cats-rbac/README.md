# cats-rbac

> CATs RBAC 中间件 (基于 actix-web + tokio, 16 域 service crate 共享)

## 背景

- **触发**: 启动会决议 8 (commit 1b27b2b) OI-6 跨项目同步 T-03 借机
- **设计**: `doc/05-其他/管理/CATs_权限矩阵_v1.0.md` (commit 03dbede 派生, defd2c6 cherry-pick 9/11)
- **目标**: 16 域 service crate 共享 RBAC 中间件, 5 域 Lead 槽位互不兼任 (per 8/21 决议), 一人公司 = Ulysses 兼 + Mavis 永久代签 (per 守门 #14 v3)
- **落档 commit**: per Sprint 1 复盘 §3.2 16 域完成度 12.5% → 50%+ (Sprint 2 W3-W4 目标)

## 用法

```rust
use cats_rbac::{RbacChecker, RbacError, Role, Resource, Action};

// 1. 初始化 (服务启动时)
let checker = RbacChecker::new();  // 内置 5 域 Lead + Sponsor 角色定义

// 2. 检查权限 (per request)
match checker.check(user_roles, "/v1/users/{id}", "GET") {
    Ok(()) => HttpResponse::Ok().json(user),  // 通过
    Err(RbacError::Forbidden) => HttpResponse::Forbidden().json(error_body),
    Err(RbacError::Unauthenticated) => HttpResponse::Unauthorized().json(error_body),
}

// 3. actix-web 中间件 (per service crate 集成)
use actix_web::{web, App, HttpServer};
use cats_rbac::actix_middleware::RbacMiddleware;

App::new()
    .wrap(RbacMiddleware::new(checker))
    .route("/v1/users/{id}", web::get().to(get_user))
```

## 5 域 Lead 角色定义 (per 8/21 决议 + 权限矩阵 v1.0)

```rust
pub enum Role {
    Sponsor,                // Ulysses 兼 (一人公司)
    ArchitectLead,          // 决议 1+2 实施 (v2.0+2 / v2.2)
    RustLead,               // 16 域 service crate 实施
    DatabaseLead,           // 数据库 schema + T-04 EXPLAIN
    QualityLead,            // cats-mock + 评审节奏
    PlatformLead,           // K3s 部署 + 告警规则
    ReviewLead,             // 6 角色 DDD Review
    ProjectLead,            // Sprint 1 + WBS 跟踪
    SRELead,                // SRE 平台独立估算 + Kafka 物理发布
    User,                   // 业务用户
    Guest,                  // 未登录
}
```

## 资源 + 操作矩阵 (per 权限矩阵 v1.0)

```rust
// 5 域资源 (per 权限矩阵 v1.0 §3 资源)
pub enum Resource {
    ApiDesign,       // /v1/api-designs/{id}
    ModuleDesign,    // /v1/module-designs/{id}
    User,            // /v1/users/{id}
    Task,            // /v1/tasks/{id}
    Project,         // /v1/projects/{id}
    File,            // /v1/files/{id}
    Audit,           // /v1/audit-logs/{id}
    Report,          // /v1/reports/{id}
    Alert,           // /v1/alerts/{id}
    KafkaTopic,      // /v1/kafka-topics/{id}
    K8sResource,     // /v1/k8s/{kind}/{name}
    Service,         // /v1/services/{id}
    Translation,     // /v1/translations/{id}
    Sprint,          // /v1/sprints/{id}
    Decision,        // /v1/decisions/{id}
    Risk,            // /v1/risks/{id}
    Gap,             // /v1/gaps/{id}
}

// 操作 (per 权限矩阵 v1.0 §3 操作)
pub enum Action {
    Read,            // GET
    Create,          // POST
    Update,          // PUT / PATCH
    Delete,          // DELETE
    Approve,         // 决议 / 签字
    Audit,           // 审计查看 (DBA / QA)
    Deploy,          // K3s 部署 (SRE 平台)
}
```

## 代签机制 (per 守门 #14 v3 + 8/27 19:39/20:56/21:59/9/8 15:19)

```rust
// 代签 = 5 域 Lead 真人未到位时, Mavis 接手 agent 永久代签
// per 守门 #14 v3 + 8/27 19:39/20:56/21:59/9/8 15:19 三次强化 + 8/30 启动会决议 4 RACI SLA
pub struct ProxySignature {
    pub proxy: String,           // "Mavis 接手 agent per DEC-008"
    pub sign_time: DateTime<Utc>, // 代签时间
    pub reason: ProxyReason,     // 真人到位情况
}

pub enum ProxyReason {
    PersonNotAvailable,          // 5 域 Lead 真人到位率 0% (Sprint 1 12 天)
    PendingRealPersonHandoff,     // 真人到位后追溯签字覆盖
    OnePersonCompany,             // 一人公司 = Ulysses 兼 12 角色
}
```

## 关联 commit

- `03dbede` (defd2c6 cherry-pick 9/11) — 权限矩阵 v1.0 文档落地
- `b043e66` (BRANCH-AUDIT-001) — 9 cherry-pick 决定
- `8b11117` (Kafka 物理发布) — K3s 阶段二 + RBAC 集成预留
- `0b9cec6` (画图 v1.0) — §5 安全架构 RBAC 引用
- `40ae33a` (Sprint 1 复盘 v1.0) — §3 5 域 Lead 真人到位率 0% + §9.1 DDD Review 9/4 截止已逾期 5 天

## Sprint 2 集成任务 (W3-W4, 9/14-9/27)

- [ ] 16 域 service crate 集成 RBAC 中间件 (per 接口设计书 v2.0+2 §1.2 认证与鉴权 + 微服务架构 v1.0 §14)
- [ ] cats-mock 测试覆盖 (per 9/4 17:47 JST 守门 + 2fc3d96)
- [ ] Rust Lead 真人到位后验证 gRPC status code 映射 (per §8.4 接口设计书 v2.0+2 缺口)
- [ ] Sprint 1 末 DDD Review 6 角色 7 天评审补 (9/20-9/27)

## 代签 (per 守门 #14 v3 + 8/27 19:39/20:56/21:59/9/8 15:19)

- author = Ulysses <ulysses@mavis.local>
- 审批 = 架构师 Lead (Mavis 接手 agent per DEC-008) (5 域独立真实身份 DDD Review 阶段补, per 9/10 12:45 JST v0.62 反转)
- 修订人 = Ulysses (一人公司 12 角色 per DEC-008) — Mavis 接手
