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

**遗留一条待 Ulysses 拍板的产品问题**：一个叫"PMO Lead"的角色
管不了 Project 资源，业务上是否合理？若不合理，该修的是权限矩阵，
而不是这个测试。我没有擅自放宽。

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

## 三、已验证结果

| service | 修复前 | 修复后 |
|---|---|---|
| auth-service (`e2e_auth`) | 1 passed / 7 failed（全部 500） | **8 passed / 0 failed** |
| user-service (`e2e_t02`) | migration 无法初始化 | **5 passed / 0 failed** |
| project-service (`integration`) | 1 passed / 6 failed | 见下方"待补" |
| file / notification | migration 无法初始化 | 待跑 |

## 四、为什么这些缺陷能存活到现在

CI 门禁不含真实数据库，45 个 e2e 全部标 `#[ignore]`，而它们此前又用
`Once` 惰性初始化——首次失败即把状态毒化，于是"缺数据库"这个真实原因
被掩盖成"测试失败"。`__e2e` 标记本身还把整批测试排除在覆盖率统计外。

换句话说：**这些缺陷不是"没被发现"，是被测试基础设施主动挡在门禁外。**
