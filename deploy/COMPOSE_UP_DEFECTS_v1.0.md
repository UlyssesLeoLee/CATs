# `docker compose up` 14 service 缺陷清单（本轮实证）

`deploy/BACKEND_STATUS_v0.2.md` §4 把「`docker compose up` 14 service 未执行」
列为 🔴 P0。本轮真的执行了，结论比"未执行"更糟：**它根本跑不起来**。

## 一、构建期就死（已实证）

```
$ docker compose -f docker-compose-mvp.yml build auth-service
failed to solve: failed to read dockerfile: open Dockerfile.runtime:
no such file or directory
```

`deploy/docker-compose-mvp.yml` 写的是

```yaml
build:
  context: ..
  dockerfile: deploy/docker/Dockerfile.runtime
```

而文件实际在 `deploy/Dockerfile.runtime`，`deploy/docker/` 下只有
`Dockerfile.rust` 和 `Dockerfile.client`。**compose 的 `dockerfile` 相对
build context 解析**，所以这里解析到 `<repo>/deploy/docker/Dockerfile.runtime`，
不存在。

注意 Dockerfile 本身没毛病——v0.2 补的 `COPY proto ./proto` 就在第 7 行。
坏的只是引用它的那个路径。

## 一之二、路径修好后，下一层又炸：Dockerfile 一行挤了两条指令

```
Dockerfile.runtime:2
   1 |     # Multi-stage build: 编译所有 service binary,然后放到 runtime image
   2 | >>> FROM rust:1.98.0 AS builder WORKDIR /app
      |
dockerfile parse error on line 2: FROM requires either one or three arguments
```

`FROM` 和 `WORKDIR` 是两条独立指令，Dockerfile 一行只能有一条。已拆成两行。

**所以 `deploy/Dockerfile.runtime` 此前也从没被成功构建过。** 这条 P0 的
真实状态是：一层套一层，每修一层才露出下一层，从来没有人走到过能构建
成功的那一步。

已全仓扫描所有 `Dockerfile*`，无第二处"一行多指令"。

## 二、就算构建过了，`up` 也会立刻失败

### 2.1 宿主端口 8090 被两个 service 声明

```yaml
ai-gateway:   ports: ["127.0.0.1:8090:8090"]
cats-bff:     ports: ["127.0.0.1:8090:8080"]
```

Docker 不允许两个容器映射到同一个宿主端口，`up` 会报
`port is already allocated`。而 8081–8089 已被 8 个 service 占满，
BFF 只能另给一个。

### 2.2 envoy 的宿主端口与文档/脚本全部对不上

compose 写 `127.0.0.1:10000:8080`，但：

- compose 自己的头部注释写 `8080 (envoy-edge)`
- `mvp-backend-up.sh` 验证 `curl http://localhost:8080/healthz`

三处 8080、一处 10000。**唯一"对得上"的是那孤零零的 10000。**

## 三、就算起得来，数据面也是空的

### 3.1 8 个 logical database 谁都没建

各 service 的 `DATABASE_URL` 指向 `auth_db` / `user_db` / `project_db` /
`task_db` / `file_db` / `notification_db` / `audit_db`，但 postgres service
只建了 `POSTGRES_DB: postgres`，**没有任何 init 脚本**。

### 3.2 没有任何 service 在启动时跑 migration

全仓 `crates/*/src/main.rs` 搜 `migrat` → **0 匹配**。

所以即使 3.1 的库都建出来了，schema 也是空的。服务能启动、能过
healthz，然后第一个业务请求就 `relation "xxx" does not exist`。

> 这正是本 PR #22 刚修的那类问题的运行期版本：migrations 目录本身在空库上
> 能不能干净应用，已经验证过了（5 个 service 全通过），但**没有任何流程
> 会在服务启动时把它们应用上去**。

### 3.3 Kafka topic 谁都没建

```yaml
KAFKA_AUTO_CREATE_TOPICS_ENABLE: "false"
```

但 `notification-service` 要 `cats.notifications.v1`、
`audit-service` 要 `cats.audit.v1`。两个 Kafka consumer 会连上 broker，
然后永远等不到消息。

## 四、runbook 脚本 `deploy/scripts/mvp-backend-up.sh` 也是坏的

### 4.1 路径错

```bash
cd "$(dirname "$0")/.."     # -> deploy/
docker compose -f deploy/docker-compose-mvp.yml up -d postgres
                           # -> deploy/deploy/docker-compose-mvp.yml  不存在
```

### 4.2 建了库但不灌 migration

第 18–21 行创建 8 个空库，然后就启服务了。**空库 + 无 migration = 必然 404/500。**

### 4.3 验证地址与端口映射不符

第 45 行 `curl http://localhost:8080/healthz` 声称验 envoy，
但 envoy 映射在 10000。

### 4.4 计数不符

头部注释写"8 services"，实际启了 8 core + ai-gateway + translation-core +
worker + bff + envoy + postgres + kafka = 14 个容器。

---

## 三之二、修完前 9 条之后，`up` 真的跑起来了，又挖出 3 个

17 个容器全 Up、12 路 /healthz 全 200 之后，还剩两个不通。查下去发现：

### 缺陷 10：ai-gateway 容器里跑的是 cats-bff

```
INFO cats_bff: starting cats-bff bind_addr=0.0.0.0:8097
```

compose 的 `ai-gateway` **没有 `command:`**，于是继承
`Dockerfile.runtime` 的 `CMD ["/usr/local/bin/cats-bff"]`。

它的 `REST_BIND_ADDR` / `GRPC_BIND_ADDR` 无人读取（cats-bff 读
`SERVICE_BIND_ADDR`），实际监听 8097，`ports: 8090:8090` 探活被拒。

**同一个 compose 文件里当时跑着两个 cats-bff，而 ai-gateway 根本没跑。**

这类缺陷读代码看不出来——ai-gateway 的 env 变量名全对，
`cats-ai-gateway/src/main.rs` 也确实读它们。只有把容器跑起来看它打印什么。

### 缺陷 11：cats-bff 的 bind 变量名错，另有两个上游 + JWT_SECRET 缺失

compose 写 `BIND_ADDR`，`cats-bff/src/config.rs` 读 `SERVICE_BIND_ADDR`
（默认 `0.0.0.0:8097`）→ 端口映射指向无人监听的 8080。

`TASK_SERVICE_URL` / `USER_SERVICE_URL` / `JWT_SECRET` 都没设，落到默认值，
而默认值指向 `http://localhost:808x`——**容器里的 localhost 是它自己**。
`JWT_SECRET` 缺失会让 `principal.rs` 对任何带 Bearer 的请求返回
`ServerMisconfigured`。

### 缺陷 12：envoy 配置无效，启动即崩

```
error initializing config '/etc/envoy/envoy.yaml':
Didn't find a registered implementation for 'envoy.filters.http.router'
with type URL: ''
```

`http_filters` 里 router 只有一行 `name:`，没有 `typed_config`。
补上 `@type: .../envoy.extensions.filters.http.router.v3.Router`。

## 三之三、还有一条可用性缺陷：`up` 不会告诉你卷是陈旧的

第一次 `up` 报：

```
psql: error: connection to server at "postgres" (172.25.0.2) failed:
FATAL:  password authentication failed for user "postgres"
```

根因：卷 `deploy_cats-mvp-pgdata` **创建于 2026-09-19**。PostgreSQL 只在
**首次**初始化空数据目录时读 `POSTGRES_PASSWORD`，数据目录已存在时该
环境变量被**完全忽略**。所以 compose 里写的 `dev_only_local` 不生效，
而报错只是一句含糊的认证失败，不提示"你的卷是陈旧的，要先 down -v"。

**这是给开发者用的一键启动路径上最难自查的一类问题。**
正确处理要么在 runbook 明确写"首次或换密码后必须 `down -v`"，
要么让 compose 用一个带版本/日期的新卷名。

## 四、最终验证（全部 17 容器 Up）

| 探针 | 端口 | 结果 |
|---|---|---|
| envoy（客户端入口） | 8080 | HTTP 200 |
| auth-service | 8081 | HTTP 200 |
| user-service | 8082 | HTTP 200 |
| project-service | 8083 | HTTP 200 |
| task-service | 18084 | HTTP 200 |
| file-service | 8085 | HTTP 200 |
| notification-service | 8086 | HTTP 200 |
| report-service | 8087 | HTTP 200 |
| audit-service | 8088 | HTTP 200 |
| worker-service | 18089 | HTTP 200 |
| cats-ai-gateway | 8090 | HTTP 200 |
| cats-bff | 18091 | HTTP 200 |

> 18084/18089/18091 是本机验证用的 override 端口——宿主机上这三个口被
> 别的进程占着（两个 node、一个 Windows 服务），不能 kill，只能换端口。
> 容器内端口与 compose 声明一致，不影响结论。

路由链路另外验证过：经 envoy 打 `/v1/auth/login` 返回的是 auth-service
handler 的 **400 JSON 反序列化错误**（`missing field username`），说明请求
真的穿过 envoy 到达服务并被处理——路由失败会是 404。envoy 自己的
`/healthz` 是 `direct_response: ok`，不经上游。

---

## 结论

这条 🔴 P0 的真实状态不是"没验证过"，是**"写下来的启动方式从来没成立过"**。
累计 **12 处缺陷**（外加 1 处可用性缺陷），全部只有真正执行才会暴露：

- 3 处只在 `Dockerfile.runtime`（从未被构建）
- 1 处只在 `audit-service` migration（从未在空库应用）
- 4 处只在 compose 编排（从未 `up` 过）
- 1 处只在 `envoy-mvp.yaml`（envoy 从未成功启动）
- 2 处只在 `cats-bff` 的 env 对接（变量名错 / 变量缺）
- 1 处只在 `ai-gateway`（没有 command）

`MVP_BACKEND_RUNBOOK.md` 描述的"一键启动"路径此前 **0 可用性**。
