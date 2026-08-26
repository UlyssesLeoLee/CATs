# CATs 类图 v1.1

> **文档编号**：CATs-DD-044（CATs 类图）  
> **フェーズ**：44 クラス設計  
> **关联任务**：150 任务 #44、#42（程序结构）、#43（模块设计）  
> **版本**：v1.1（基线升级：Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）  
> **创建日**：2026-08-20  
> **更新**：v1.0 → v1.1（2026-08-26 锁定 Rust 1.98.0 + PG 18.6 + pgvector 0.8.6，见 [`CATs_技术基线_v1.0`](../../02-基础设计/技术选型/CATs_技术基线_v1.0.md)）  
> **作者**：架构师 + Rust Lead + DBA（worker 代签 per DEC-008）

---

## 文档管理信息

### 审批栏

| 角色 | 审批 | 日期 |
|------|------|------|
| 架构师 | ☐ | — |
| Rust Lead | ☐ | — |
| 前端 Lead | ☐ | — |
| QA | ☐ | — |

### 修订履历

| 版本 | 日期 | 修订人 | 修订内容 |
|------|------|--------|----------|
| v1.0 | 2026-08-20 | 架构师 | 评审前草稿：15 微服务 + 共享库类图 |
| **v1.1** | **2026-08-26** | **架构师 + Rust Lead + DBA（worker 代签 per DEC-008）** | **基线升级：统一引用 `CATs_技术基线_v1.0`（Rust 1.98.0 + PostgreSQL 18.6 + pgvector 0.8.6）** |

---

## 1. 目的

为 CATs 项目的**核心代码结构**提供 UML 类图，作为：

- 开发者入手的导航图
- 详细设计评审（DD Review）的依据
- 公共 API 文档的同步
- 重构 / 维护的参考

---

## 2. 范围

| 维度 | 范围 |
|------|------|
| 微服务 | 15 服务（核心 6 + 业务 4 + 支撑 5） |
| 共享库 | 4 个（common / error / grpc / db） |
| 浏览器 | BFF + 扩展 |
| 不含 | 第三方库 / ORM 自动生成 |

---

## 3. 总体类图

```
┌────────────────────────────────────────────────────────────┐
│                    cats-common                              │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │ AppError │  │  Config  │  │ Tracing  │  │ Metrics  │  │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘  │
└────────────────────────────────────────────────────────────┘
                            ▲
                            │ uses
┌────────────────────────────────────────────────────────────┐
│                     cats-error                              │
│  ┌──────────────────┐  ┌──────────────────┐               │
│  │ AppError (enum)  │  │ ErrorContext     │               │
│  │ - NotFound       │  │ - trace_id       │               │
│  │ - InvalidInput   │  │ - request_id     │               │
│  │ - Database       │  │ - user_id        │               │
│  │ - Internal       │  └──────────────────┘               │
│  └──────────────────┘                                      │
└────────────────────────────────────────────────────────────┘
                            ▲
┌────────────────────────────────────────────────────────────┐
│                    cats-grpc                                │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │ GrpcClient   │  │ GrpcServer   │  │ Interceptor  │    │
│  │ - timeout    │  │ - middleware │  │ - auth       │    │
│  │ - retry      │  │ - tracing    │  │ - logging    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└────────────────────────────────────────────────────────────┘
                            ▲
┌────────────────────────────────────────────────────────────┐
│                     cats-db                                 │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐    │
│  │  PgPool      │  │ Transaction  │  │  Outbox      │    │
│  │ - acquire    │  │ - run        │  │ - enqueue    │    │
│  │ - release    │  │ - commit     │  │ - publish    │    │
│  └──────────────┘  └──────────────┘  └──────────────┘    │
└────────────────────────────────────────────────────────────┘
                            ▲
                            │ uses all
┌────────────────────────────────────────────────────────────┐
│              15 微服务 + BFF + Worker                       │
│  tm / term / mt / llm / qa / project / file / version /    │
│  media / auth / notify / audit / admin / bff / worker      │
└────────────────────────────────────────────────────────────┘
```

---

## 4. 共享库类图

### 4.1 cats-error

```rust
// cats-error/src/lib.rs

#[derive(Error, Debug)]
pub enum AppError {
    #[error("not found: {resource_type}/{id}")]
    NotFound { resource_type: String, id: String },
    
    #[error("invalid input: {0}")]
    InvalidInput(String),
    
    #[error("unauthorized")]
    Unauthorized,
    
    #[error("forbidden")]
    Forbidden,
    
    #[error("conflict: {0}")]
    Conflict(String),
    
    #[error("rate limit exceeded")]
    RateLimited { retry_after_ms: u64 },
    
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),
    
    #[error("grpc error: {0}")]
    Grpc(#[from] tonic::Status),
    
    #[error("internal error: {0}")]
    Internal(#[from] anyhow::Error),
    
    #[error("upstream timeout: {service}")]
    UpstreamTimeout { service: String },
}

pub struct ErrorContext {
    pub trace_id: String,
    pub request_id: Option<String>,
    pub user_id: Option<String>,
    pub tenant_id: Option<String>,
}

impl AppError {
    pub fn http_status(&self) -> u16 {
        match self {
            AppError::NotFound { .. } => 404,
            AppError::InvalidInput(_) => 400,
            AppError::Unauthorized => 401,
            AppError::Forbidden => 403,
            AppError::Conflict(_) => 409,
            AppError::RateLimited { .. } => 429,
            AppError::Database(_) | AppError::Redis(_) => 500,
            AppError::UpstreamTimeout { .. } => 504,
            _ => 500,
        }
    }
    
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::NotFound { .. } => "NOT_FOUND",
            AppError::InvalidInput(_) => "INVALID_INPUT",
            AppError::Unauthorized => "UNAUTHORIZED",
            AppError::Forbidden => "FORBIDDEN",
            AppError::Conflict(_) => "CONFLICT",
            AppError::RateLimited { .. } => "RATE_LIMITED",
            _ => "INTERNAL",
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;
```

### 4.2 cats-grpc

```rust
// cats-grpc/src/lib.rs

pub struct GrpcClient<T> {
    inner: T,
    timeout: Duration,
    retry: u32,
    interceptors: Vec<Box<dyn Interceptor>>,
}

impl<T> GrpcClient<T> {
    pub fn new(inner: T) -> Self { ... }
    pub fn with_timeout(mut self, d: Duration) -> Self { ... }
    pub fn with_retry(mut self, n: u32) -> Self { ... }
    pub fn with_interceptor(mut self, i: impl Interceptor + 'static) -> Self { ... }
}

pub struct GrpcServer {
    middlewares: Vec<Box<dyn Middleware>>,
    interceptors: Vec<Box<dyn Interceptor>>,
}

impl GrpcServer {
    pub fn new() -> Self { ... }
    pub fn use_middleware(mut self, m: impl Middleware + 'static) -> Self { ... }
    pub fn use_interceptor(mut self, i: impl Interceptor + 'static) -> Self { ... }
}

pub trait Interceptor: Send + Sync {
    fn call(&self, req: Request<()>) -> AppResult<Request<()>>;
}

pub trait Middleware: Send + Sync {
    fn before(&self, req: &mut Request<()>);
    fn after(&self, req: &Request<()>, resp: &mut Response<()>);
}
```

### 4.3 cats-db

```rust
// cats-db/src/lib.rs

pub struct PgPool {
    pool: sqlx::PgPool,
    config: PoolConfig,
}

pub struct PoolConfig {
    pub max_connections: u32,
    pub min_connections: u32,
    pub acquire_timeout: Duration,
    pub idle_timeout: Duration,
}

impl PgPool {
    pub async fn connect(url: &str, cfg: PoolConfig) -> AppResult<Self> { ... }
    pub async fn acquire(&self) -> AppResult<Connection> { ... }
    pub async fn transaction<F, T>(&self, f: F) -> AppResult<T>
        where F: FnOnce(&mut Transaction) -> BoxFuture<AppResult<T>> { ... }
}

pub struct Outbox {
    pool: PgPool,
    producer: KafkaProducer,
}

impl Outbox {
    pub async fn enqueue(&self, event: OutboxEvent) -> AppResult<()> { ... }
    pub async fn publish_pending(&self) -> AppResult<usize> { ... }
}

pub struct OutboxEvent {
    pub aggregate_type: String,
    pub aggregate_id: String,
    pub event_type: String,
    pub payload: serde_json::Value,
}
```

---

## 5. 核心微服务类图

### 5.1 tm-service（翻译记忆）

```rust
// services/tm-service/src/main.rs
//
// TMService (gRPC impl)
//   ├─ MatchHandler
//   │   ├─ match_segment (QueryEmbedding)
//   │   └─ batch_match (BatchQuery)
//   ├─ TuHandler
//   │   ├─ create_tu
//   │   ├─ update_tu
//   │   └─ delete_tu (soft)
//   ├─ ImportHandler
//   │   ├─ import_tmx
//   │   └─ import_xliff
//   └─ HealthHandler
//       └─ check
//
// MatchEngine
//   ├─ VectorIndex (HNSW)
//   ├─ FuzzyIndex (trigram)
//   └─ HybridRanker
//
// Repository
//   ├─ TuRepository (CRUD)
//   ├─ SegmentRepository (CRUD + 相似查询)
//   └─ OutboxEventStore
```

**类图**：

```
┌──────────────────────────────────────────────┐
│ TMService (gRPC Server)                      │
│ - pool: PgPool                                │
│ - engine: Arc<MatchEngine>                    │
│ - redis: RedisPool                            │
├──────────────────────────────────────────────┤
│ + match_segment(req) -> MatchResp            │
│ + batch_match(req) -> BatchMatchResp         │
│ + create_tu(req) -> TuResp                   │
│ + update_tu(req) -> TuResp                   │
│ + import_tmx(stream) -> ImportResp           │
│ + check(req) -> HealthResp                   │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ MatchEngine                                  │
│ - vector_index: HnswIndex                    │
│ - fuzzy_index: TrgmIndex                     │
│ - ranker: HybridRanker                       │
├──────────────────────────────────────────────┤
│ + match(emb, lang, limit) -> Vec<Match>     │
│ + warm_up() -> AppResult<()>                 │
│ + stats() -> IndexStats                      │
└──────────────────────────────────────────────┘
            │ uses
   ┌────────┴────────┐
   ▼                 ▼
┌──────────┐  ┌──────────────┐
│ HnswIndex │  │ TrgmIndex    │
│ - ef_search│ │ - threshold  │
│ - m       │  └──────────────┘
└──────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ SegmentRepository                            │
├──────────────────────────────────────────────┤
│ + find_similar(emb, threshold) -> Vec<Seg>  │
│ + create(seg) -> SegId                       │
│ + update(id, seg) -> ()                      │
│ + soft_delete(id) -> ()                      │
│ + batch_create(seg[]) -> AppResult<()>       │
└──────────────────────────────────────────────┘
```

### 5.2 term-service（术语库）

```rust
// services/term-service/src/main.rs
//
// TermService (gRPC)
//   ├─ LookupHandler
//   │   └─ lookup_term
//   ├─ TermHandler
//   │   ├─ create_term
//   │   ├─ update_term
//   │   └─ delete_term
//   ├─ ImportHandler
//   │   └─ import_tbx
//   └─ HealthHandler
//
// TermMatcher
//   ├─ ExactMatcher
//   └─ FuzzyMatcher
```

```
┌──────────────────────────────────────────────┐
│ TermService                                  │
├──────────────────────────────────────────────┤
│ + lookup_term(text, lang) -> Term[]         │
│ + create_term(term) -> TermId                │
│ + update_term(id, term) -> ()                │
│ + import_tbx(stream) -> ImportResp           │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ TermMatcher                                  │
│ - exact: ExactMatcher                        │
│ - fuzzy: FuzzyMatcher                        │
├──────────────────────────────────────────────┤
│ + match(text, lang) -> Vec<Term>            │
└──────────────────────────────────────────────┘
```

### 5.3 bff（Backend for Frontend）

```rust
// apps/bff/src/main.rs
//
// BFF Server
//   ├─ GraphQLHandler (前端主入口)
//   ├─ RestHandler (REST API 兼容)
//   ├─ WebSocketHandler (实时推送)
//   ├─ AuthMiddleware
//   └─ RateLimitMiddleware
//
// QueryService
//   ├─ TranslationQuery (聚合 tm + term + llm)
//   └─ ProjectQuery
//
// MutationService
//   ├─ TranslationMutation
//   └─ ProjectMutation
```

```
┌──────────────────────────────────────────────┐
│ BFFServer                                    │
├──────────────────────────────────────────────┤
│ + graphql(query, vars) -> GraphQLResp       │
│ + rest(method, path, body) -> RestResp      │
│ + ws(subscription) -> WsStream              │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ QueryService                                 │
│ - tm: TmClient                               │
│ - term: TermClient                           │
│ - llm: LlmClient                             │
├──────────────────────────────────────────────┤
│ + suggest_translation(text) -> Suggestion   │
│   (并行调用 tm + term + llm，5s 内返回)      │
└──────────────────────────────────────────────┘
```

### 5.4 llm-gateway（本地 LLM 推理网关）

```rust
// services/llm-gateway/src/main.rs
//
// LlmService
//   ├─ InferenceHandler
//   │   ├─ translate (主任务)
//   │   ├─ complete (补全)
//   │   └─ embed (embedding)
//   ├─ ModelHandler
//   │   ├─ load_model
//   │   └─ unload_model
//   └─ HealthHandler
//
// ModelManager
//   ├─ LoadedModel (LRU 缓存)
//   └─ Backend (llama.cpp / candle / vllm)
//
// RateLimiter
//   └─ TokenBucket (per user)
```

```
┌──────────────────────────────────────────────┐
│ LlmService                                   │
├──────────────────────────────────────────────┤
│ + translate(req) -> TranslateResp           │
│ + complete(req) -> CompleteResp             │
│ + embed(text) -> Embedding                   │
│ + load_model(name) -> ()                    │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ ModelManager (单例)                          │
│ - models: LruCache<ModelKey, LoadedModel>   │
│ - backends: HashMap<String, Backend>         │
├──────────────────────────────────────────────┤
│ + get_or_load(name) -> Arc<LoadedModel>     │
│ + unload(name) -> ()                         │
└──────────────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────────────┐
│ LoadedModel                                  │
│ - backend: Arc<dyn Backend>                  │
│ - tokenizer: Arc<Tokenizer>                  │
│ - metadata: ModelMetadata                    │
├──────────────────────────────────────────────┤
│ + infer(input, params) -> Output             │
│ + tokenize(text) -> Vec<Token>               │
│ + embed(text) -> Vec<f32>                    │
└──────────────────────────────────────────────┘
```

### 5.5 qa-engine（QA 引擎）

```
┌──────────────────────────────────────────────┐
│ QaService                                    │
├──────────────────────────────────────────────┤
│ + scan_document(doc) -> ScanReport          │
│ + run_rule(rule_id, doc) -> RuleResult      │
│ + list_rules() -> Rule[]                     │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ RuleEngine                                   │
│ - rules: Vec<Box<dyn Rule>>                  │
├──────────────────────────────────────────────┤
│ + register(rule: impl Rule + 'static)       │
│ + execute(doc, rule_set) -> Vec<Issue>      │
└──────────────────────────────────────────────┘
            │ uses
   ┌────────┴────────────────────────┐
   ▼                                 ▼
┌────────────────┐         ┌────────────────┐
│ TermConsistency │         │ NumberConsistency│
│ Rule            │         │ Rule            │
├────────────────┤         ├────────────────┤
│ + check(doc)    │         │ + check(doc)    │
│   -> Vec<Issue> │         │   -> Vec<Issue> │
└────────────────┘         └────────────────┘
```

### 5.6 worker-svc（批处理 / 异步任务）

```
┌──────────────────────────────────────────────┐
│ WorkerService                                │
├──────────────────────────────────────────────┤
│ + enqueue_task(task) -> TaskId              │
│ + get_status(task_id) -> TaskStatus         │
│ + cancel(task_id) -> ()                      │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ TaskQueue (Kafka based)                      │
├──────────────────────────────────────────────┤
│ + submit(task) -> TaskId                    │
│ + poll(timeout) -> Vec<Task>                 │
│ + complete(task_id, result) -> ()            │
│ + fail(task_id, error) -> ()                 │
└──────────────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────────────┐
│ TaskHandler (per kind)                       │
├──────────────────────────────────────────────┤
│ + handle_etl(args) -> EtlResult             │
│ + handle_export(args) -> ExportResult       │
│ + handle_index_rebuild(args) -> IndexResult │
│ + handle_aggregation(args) -> AggResult     │
└──────────────────────────────────────────────┘
```

### 5.7 auth-svc（认证授权）

```
┌──────────────────────────────────────────────┐
│ AuthService                                  │
├──────────────────────────────────────────────┤
│ + login(credentials) -> LoginResp           │
│ + refresh(refresh_token) -> LoginResp       │
│ + logout(token) -> ()                        │
│ + verify(token) -> Claims                   │
│ + issue_service_token(svc) -> ServiceToken  │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ JwtIssuer                                    │
│ - issuer: String                             │
│ - audience: String                           │
│ - private_key: SigningKey                    │
├──────────────────────────────────────────────┤
│ + issue(claims, ttl) -> String              │
│ + verify(token) -> Claims                   │
└──────────────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────────────┐
│ RbacEnforcer                                 │
│ - policy: CasbinEnforcer                     │
├──────────────────────────────────────────────┤
│ + check(user, resource, action) -> bool     │
│ + grant(user, role) -> ()                    │
│ + revoke(user, role) -> ()                   │
└──────────────────────────────────────────────┘
```

### 5.8 audit-svc（审计）

```
┌──────────────────────────────────────────────┐
│ AuditService                                 │
├──────────────────────────────────────────────┤
│ + record(event) -> ()                        │
│ + query(filter) -> Vec<AuditEvent>          │
│ + export(filter) -> Stream<AuditEvent>      │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ AuditStore (append-only)                     │
│ - pg: PgPool                                 │
│ - cold: S3Archive (可选)                     │
├──────────────────────────────────────────────┤
│ + insert(event) -> ()                        │
│ + find(filter) -> Vec<AuditEvent>           │
│ + sign(records) -> WORM sealed               │
└──────────────────────────────────────────────┘
```

---

## 6. 浏览器扩展类图

```
┌──────────────────────────────────────────────┐
│ BrowserExtension                             │
├──────────────────────────────────────────────┤
│ - background: BackgroundScript               │
│ - content: ContentScript                     │
│ - popup: Popup                               │
│ - options: OptionsPage                       │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ BackgroundService                            │
│ - jwt: string                                │
│ - ws: WebSocket                              │
├──────────────────────────────────────────────┤
│ + login(creds) -> Promise<Jwt>               │
│ + logout() -> Promise<void>                  │
│ + translate(text) -> Promise<Translation>   │
│ + on_event(cb) -> Subscription              │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ ContentScript                                │
│ - editor: EditorIntegration                  │
├──────────────────────────────────────────────┤
│ + injectUI(target) -> ()                     │
│ + onSelection(cb) -> Subscription           │
│ + showSuggestion(text) -> ()                 │
└──────────────────────────────────────────────┘
```

---

## 7. 前端类图（Next.js）

```
┌──────────────────────────────────────────────┐
│ App                                          │
├──────────────────────────────────────────────┤
│ + router: AppRouter                          │
│ + providers: [QueryProvider, AuthProvider]   │
└──────────────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────────────┐
│ Page: /translate                             │
│ - editor: Editor                            │
│ - sidebar: Sidebar                          │
│ - statusbar: StatusBar                      │
└──────────────────────────────────────────────┘
            │ uses
            ▼
┌──────────────────────────────────────────────┐
│ Editor (TipTap based)                        │
│ - doc: Y.Doc (CRDT)                          │
│ - extensions: TipTapExt[]                    │
├──────────────────────────────────────────────┤
│ + insert(text) -> ()                         │
│ + suggest(text) -> Promise<Suggestion>      │
│ + onChange(cb) -> Subscription              │
└──────────────────────────────────────────────┘
            │
            ▼
┌──────────────────────────────────────────────┐
│ useGraphQL (TanStack Query)                  │
├──────────────────────────────────────────────┤
│ + suggestMutation: useMutation              │
│ + queryClient: QueryClient                   │
│ + wsClient: GraphQLWsClient                  │
└──────────────────────────────────────────────┘
```

---

## 8. 关系图（服务间依赖）

```
       浏览器扩展
            │
            ▼
       ┌────BFF────┐
       │  GraphQL  │
       └─────┬─────┘
             │
   ┌─────────┼─────────┐
   ▼         ▼         ▼
 tm-svc  term-svc  llm-gateway
   │         │         │
   └────┬────┘         │
        ▼              │
    project-svc        │
        │              │
        ├─ file-svc    │
        ├─ version-svc │
        └─ media-pipeline
                │
                ▼
            worker-svc
                │
                ▼
         audit-svc + notify-svc
                │
                ▼
            admin-svc
                │
                ▼
            auth-svc (所有服务都鉴权)
```

---

## 9. 设计模式应用

| 模式 | 应用位置 | 价值 |
|------|----------|------|
| **Repository** | 所有服务的 xxxRepository | 持久化抽象 |
| **Factory** | GrpcClient / PgPool | 构造复杂对象 |
| **Strategy** | RuleEngine / MatchEngine | 多种算法可替换 |
| **Builder** | OutboxEvent / QueryBuilder | 链式构造 |
| **Decorator** | Interceptor / Middleware | 横切关注点 |
| **Observer** | EventBus / Subscription | 事件驱动 |
| **Singleton** | ModelManager / Config | 全局唯一 |
| **CQRS** | Query/Mutation 分离 | BFF |
| **Outbox** | 所有服务写事件 | 事务性事件 |
| **Saga** | 跨服务事务 | 复杂业务流 |

---

## 10. 与 150 任务 / 文档关联

| 任务 / 文档 | 关联 |
|-------------|------|
| 42 プログラム構造設計 | §3 总体类图 |
| 43 モジュール設計 | §5 微服务类图 |
| 44 クラス設計 | 本文 |
| 50 エラー処理設計 | §4.1 cats-error |
| 51 ログ設計 | §4.1-§4.3 |
| 45 ロジック設計 | §5 算法逻辑 |
| CATs_模块设计书 v2.0 | 总体类图 |
| CATs_接口设计书 v2.0 | §8 关系图 |
| CATs_微服务架构设计书 v1.0 | 整体框架 |

---

## 11. 关联文档

| 文档 | 路径 |
|------|------|
| CATs_模块设计书 v2.0 | `03-详细设计\模块设计\` |
| CATs_接口设计书 v2.0 | `03-详细设计\接口设计\` |
| CATs_数据库设计书 v2.0 | `03-详细设计\数据库设计\` |
| CATs_SQL 设计一览 v1.0 | `03-详细设计\SQL\` |
| CATs_微服务架构设计书 v1.0 | `02-基础设计\架构设计\` |
| CATs_ロジック設計書 v1.0 | `03-详细设计\逻辑\` |
| CATs_批处理详细设计 v1.0 | `03-详细设计\批处理\` |
| CATs_DD 评审纪要 v1.0 | `05-其他\评审记录\` |

---

## 12. 待办 / Open Items

| # | 项 | 责任 | 期望关闭 |
|---|----|------|----------|
| OI-1 | 各服务类图同步到代码（用 cargo doc / typedoc） | Rust Lead | M1-S1 |
| OI-2 | 关键算法详细逻辑（TM 召回、LLM 推理） | 架构师 | DD Review 前 |
| OI-3 | 跨服务 Saga 详细设计 | 架构师 | M1-S1 |
| OI-4 | CRDT 协作详细设计 | 前端 Lead | M1-S1 |
| OI-5 | 类图 → PlantUML / Mermaid 源码化 | 架构师 | M1-S1 |

---

**文档结束**
