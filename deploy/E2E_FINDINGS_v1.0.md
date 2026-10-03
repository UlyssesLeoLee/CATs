# 真实 PG 验证发现（2026-10-03）

在真实 PostgreSQL 18.6 上跑此前从未执行过的 e2e 测试时，发现的缺陷分两类。
两类都不是"测试写错"那么简单。

## 一、schema 初始化在全新数据库上无法完成（生产级阻塞）

这不是测试环境问题。每个 service 的 `migrations/` 里有**三份 init**，
来自三条平行开发线，都以 `CREATE TABLE IF NOT EXISTS <同名表>` 建表。
版本号最小的先执行，抢先建出自己那套表；后面两份的 `CREATE TABLE`
被**静默跳过**（IF NOT EXISTS 不报错），随后它们的索引引用别的 init
才会创建的列，于是报：

```
ERROR: column "workspace_id" does not exist
ERROR: column "read_at" does not exist
```

生产首次部署到空库会踩到同样的一脚。

### 权威判据：生产代码实际读写的列

| service | 生产代码用的模型 | 停用的旧 init |
|---|---|---|
| file | `workspace_id` / `owner_user_id` / `filename` / `storage_path` | `0001`（`org_id`）、`20260919`（`storage_backend`/`purpose`） |
| project | `workspace_id`（`src/db.rs` 出现 18 次，`org_id` 零引用） | `0001`、`20260919`（均 `org_id`） |
| notification | `type` / `title` / `body` / `payload` / `read_at` / `status` / `updated_at` | `0001`（`channel`/`event_type`/`sent_at`）、`20260919`（缺三列） |

处理方式：停用旧 init 中与生产代码不符的表定义，保留其不冲突的附属表
（`file_versions`、`outbox_event`、`terms`、`glossary_versions`、
`translation_memory`、`tm_vectors`、`notification_prefs`）。
外键改为在权威版建表后用动态 `EXECUTE` 补加——直接写
`REFERENCES projects(id)` 即使在 `IF to_regclass(...) IS NOT NULL`
保护下也会在解析期失败。

### user-service 是另一类：嵌套 dollar-quote

```
ERROR: syntax error at or near "BEGIN"   (LINE 9: BEGIN)
```

`20260919_0002_align_v2.sql` 的 `DO $$ ... $$` 内部又写了
`CREATE FUNCTION ... AS $$ ... $$`。PostgreSQL 的 dollar-quote **不支持嵌套**，
内层 `$$` 提前关闭了外层 `DO` 块，于是 `BEGIN` 成了裸语句。

修法：内层改用 `$func$`。

> 补充一个踩过的坑：我第一版修复在注释里写了 `$$` 来解释原因，
> 结果注释本身位于 `DO $$` 块内，同样被 dollar-quote 截断，
> 报 `syntax error at or near "不支持嵌套,"`。注释里也必须避开 `$$`。

## 二、e2e 测试的 App 装配与认证落后于实现

这一类不是产品缺陷，是**测试没跟上实现**，但它伪装成了产品缺陷。

### auth-service：少注册 AppState → 全部 500

真实 PG 下 8 个测试里 7 个失败，全部返回 500：

```
500 "Requested application data is not configured correctly."
```

`main.rs` 注册 `web::Data<AppState>`（pool + audit sink），
而测试的 `make_app` 只注册了裸 `pool`。handler 签名是
`web::Data<AppState>`，extractor 取不到就 panic 成 500。

**这个 500 极具误导性**：断言 `should be 401` 看到 500 会让人以为是
认证逻辑坏了，实际是测试装配缺失。修 `make_app` 后 **8 passed / 0 failed**。

### project-service：缺 rbac_data → 500；缺认证头 → 401；角色不足 → 403

同一个文件踩了三层坑，每修一层就暴露下一层：

| 症状 | 真实原因 |
|---|---|
| 全部 500 | `make_app` 漏注册 `web::Data<Arc<RbacChecker>>`（`main.rs` 有） |
| 全部 401 | 请求完全不带 `Authorization`，而每个 handler 都跑 `rbac::enforce` |
| 写操作 403 | 补了 `User` 角色，但 User 对 `Resource::Project` 只有 Read |

最终按 HTTP 方法分配角色——这是 RBAC 矩阵的属性，不是测试的偏好：

```
GET           -> User      (Read)
POST/PATCH/DELETE -> Sponsor (Create / Update / Delete)
```

> 注意：不能图省事给所有请求统一加 `User`。User 确实没有 Project 写权限，
> 那样只是把 401 变成 403。授权矩阵是刻意收紧过的（PR #19 里我还把两个
> "User 能读 Alert/Report" 的越权测试改成了断言真实 403）。

### ⚠ 更正：写操作角色不是 ProjectLead（我先前写错了）

我一度把上表写成 `POST/PATCH/DELETE -> ProjectLead`，**这是错的**。
补角色后 5 个测试仍然 403。去查权限矩阵才发现：

`Role::ProjectLead`（枚举注释写的是"PMO Lead"）的权限只覆盖
**Sprint / Decision / Risk / Gap** 四个资源，**根本不含 Project**。
名字叫"Project Lead"却管不了 Project 资源，这正是名字骗人的地方——
不查矩阵只凭名字分配角色，必然踩坑。

`Resource::Project` 的权限实测分布（`cats-rbac/src/lib.rs`
`default_permissions()`）：

| 角色 | Project 上的权限 |
|---|---|
| **Sponsor** | **全部 action（唯一有写权限的角色）** |
| ArchitectLead | Read |
| DatabaseLead | Read |
| QualityLead | Read |
| User | Read |
| ProjectLead (PMO) | **无** |
| RustLead / SRELead / Guest | 无 |

按"实现是权威，测试是过时的"原则，改测试（用 Sponsor），不动生产权限矩阵。
放宽权限矩阵是安全相关的决策，不该为了让测试变绿而顺手做掉。

**已拍板（2026-10-03，Ulysses）**：保持现状——`Project` / `File` 的写权限
**只给 `Sponsor`**，符合权限矩阵 v1.0 §3 原文。不给 `ProjectLead` 补权限。

于是这条不再是"遗留待办"，而是一条**已确认的设计选择**：一个叫
"PMO Lead"的角色在权限矩阵上管不到 Project，是矩阵本身的结果。
将来若有人再看到 `ProjectLead` 管不了 Project 而当成 bug 去"修"，
请先读这一段——它是被显式确认过的，不是遗漏。

> 备注：该拍板是通过选项表单的**超时默认选中**收到的
> （`responseSource: automatic_timeout`，非显式回复），
> 选中的正是推荐项。结论照此执行；若判断有误，改权限矩阵即可回退。

### file / notification：同一个 500 根因，掩盖成"只有 healthz 能过"

这两个 service 的表现极具迷惑性：

```
running 8 tests
test e2e_healthz_returns_200 ... ok
test e2e_upload_file_returns_201 ... FAILED
... 其余 7 个全 FAILED（且整个 run 只花 0.06s）
```

**"healthz 过、业务全挂"是一条强信号**：healthz 是唯一一个不注入
`web::Data<Arc<RbacChecker>>` 的 handler。业务 handler 全都注入它，
而两个测试的 `make_app` 都只注册了 `pool` / `bus`，于是 actix 的
extractor 取不到，返回

```
500 "Requested application data is not configured correctly"
```

失败得**极快**（0.02–0.06s）也是线索：真连库失败的请求会卡在连接或
事务上，不会瞬间结束。

补 `rbac_data` 后，第二个坑立刻露出来：请求**完全不带 `Authorization`**，
于是全部 401。所以每个文件要改两处，不是一处。

> 这一类三次都栽在同一个地方（auth 缺 AppState、project 缺 rbac_data、
> file/notification 缺 rbac_data），说明"测试的 App 装配与 `main.rs` 漂移"
> 是个系统性问题，不是三处独立的笔误。真正的解法是让 `make_app` 复用
> `main.rs` 的注册代码，而不是每个测试文件手抄一遍——手抄就一定会漂移。

### 角色分配：必须查矩阵，不能凭角色名猜

三处都靠猜角色名踩了坑，而实际矩阵与名字的对应关系很反直觉：

| service | 资源 | 写操作授权角色 | 读操作授权角色 |
|---|---|---|---|
| project | `Resource::Project` | **仅 Sponsor** | User / ArchitectLead / DatabaseLead / QualityLead |
| file | `Resource::File` | **仅 Sponsor** | User / ArchitectLead / DatabaseLead / QualityLead |
| notification | `Resource::Alert` | SRELead / Sponsor | SRELead / ArchitectLead / QualityLead / Sponsor |

反直觉的三点：

1. `Role::ProjectLead`（枚举注释写"PMO Lead"）**在 Project 上没有任何权限**。
   不查矩阵只看名字，会一直以为"Project Lead 当然能管 Project"。
2. `Resource::File` 的写权限也只有 Sponsor——File 和 Project 待遇一致。
3. notification 走的是 `Resource::Alert`（不是 `Resource::Notification`），
   且 **User 对 Alert 连 Read 都没有**（User 的 Read 列表是
   User/Task/Project/File/Translation，不含 Alert）。
   给它套用 project/file 的 "User 读、Sponsor 写" 会 403。

## 二之二、CI 自身的两个缺陷（run 37112321110 实证）

把 e2e 接进 CI 后，第一次 run 就暴露了两个**与测试无关**的 CI 缺陷：

### 缺陷 1：e2e job 的 Postgres 镜像缺 pgvector

```
psql:crates/project-service/migrations/20260919_0001_init.sql:14:
ERROR:  extension "vector" is not available
HINT:  The extension must first be installed on the system where PostgreSQL is running.
```

`project-service/20260919_0001_init.sql` 有
`CREATE EXTENSION IF NOT EXISTS vector`（per 技术选型 ADR-30）。
`pgcrypto` 和 `citext` 官方 postgres 镜像自带 contrib，**`vector` 不带**。
本机 e2e 用的是 `pgvector/pgvector:pg18`，所以本地一路绿灯——
**这正是"本机跑绿 ≠ 门禁能跑绿"的实例**（镜像差异，不是宿主差异）。
已把 CI 镜像同步为 `pgvector/pgvector:pg18`。

### 缺陷 2：ubuntu `test` job 里残留一段死代码

```
psql: error: connection to server at "127.0.0.1", port 5432 failed:
Connection refused
```

`test` job 矩阵含 macOS/Windows，容器只能放 Linux-only 的 `e2e` job，
所以 `test` job **根本没有 Postgres service**。但我此前把灌库的步骤
从 `test` job 移走时，忘了删掉 ubuntu 条件分支下残留的同名步骤，
它在 `runner.os == 'Linux'` 时照常执行，必然连接被拒并让 ubuntu 变红。
已删除，并留注释说明该步骤为何不属于本 job。

顺带一提：这段残留步骤里的注释还写着"版本号递增且都用 IF NOT EXISTS，
顺序应用不会冲突"——**这句话本身就是错的**，正是第一节记录的
"IF NOT EXISTS 静默跳过 → 后面报列不存在"的成因。注释比代码更危险，
因为它是后来者唯一会读的文档。

## 三、已验证结果（真实 PG 18.6 / pgvector/pgvector:pg18）

**CI run 37113777361 实证：e2e job 45 passed / 0 failed，test 三平台全绿。**

| service | 测试文件 | 修复前 | 修复后 |
|---|---|---|---|
| auth-service | `e2e_auth.rs` | 1 passed / 7 failed | **8 / 0** |
| auth-service | `e2e_t01.rs` | 从未执行 | **11 / 0**（零修改） |
| user-service | `e2e_t02.rs` | migration 起不来 | **5 / 0** |
| project-service | `integration.rs` | 1 passed / 6 failed | **7 / 0** |
| file-service | `integration.rs` | 1 passed / 7 failed | **8 / 0** |
| notification-service | `integration.rs` | 1 passed / 5 failed | **6 / 0** |
| **合计** | 6 个文件 | — | **45 passed / 0 failed** |

`e2e_t01.rs` 当时是**一行没改就过了 11/11**。缺陷并非均匀分布，而是集中在
少数文件的 App 装配上——不能因为「某个文件能过」就推断「这类都没问题」。

### 事后订正：这 45 个里有 2 个本来就不该被 ignore

`e2e_t01.rs` 的 11 个中有 2 个是**纯静态源码断言**：

```rust
fn e2e_t01_build_audit_must_not_use_spawn()   // include_str! + substring
fn e2e_t01_build_audit_still_awaits_emit()    // include_str! + substring
```

它们不建连接、不跑 migration，却被批量标 `#[ignore = "e2e-needs-real-pg"]`
一并盖住，理由与事实不符。代价是实的：这两条守护 ULYS-46 的 INVIOLABLE 约束，
标了 ignore 就只在 Linux 的 e2e job 里被检查，macOS / Windows 常规门禁全漏。

解除后本地实测（**刻意不设 `DATABASE_URL`**）：`2 passed / 9 ignored`。

所以准确的口径是：

- **43 个** 真正需要数据库的 e2e → 全部在 e2e job 里对真实 PG 执行并通过
- **2 个** 静态源码断言 → 归位到常规 `cargo test --workspace`，三平台都跑

> 顺带一个自差点：统计 `#[ignore]` 数量时用 `git grep '#\[ignore'`，
> 结果把我自己新写的**注释里提到的 `#[ignore` 字样**也数了进去
> （e2e_t01 报 11 而非 9）。改用 `^\s*#\[ignore` 精确匹配才得到正确的 43。
> 验证手段本身出错时，报出的数字和被验证的对象一样有欺骗性。

## 四、为什么这些缺陷能存活到现在

CI 门禁不含真实数据库，45 个 e2e 全部标 `#[ignore]`，而它们此前又用
`Once` 惰性初始化——首次失败即把状态毒化，于是"缺数据库"这个真实原因
被掩盖成"测试失败"。`__e2e` 标记本身还把整批测试排除在覆盖率统计外。

换句话说：**这些缺陷不是"没被发现"，是被测试基础设施主动挡在门禁外。**
