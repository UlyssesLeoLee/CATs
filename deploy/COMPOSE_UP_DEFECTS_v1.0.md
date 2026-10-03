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

## 结论

这条 🔴 P0 的真实状态不是"没验证过"，是**"写下来的启动方式从来没成立过"**。
`MVP_BACKEND_RUNBOOK.md` 描述的"一键启动"路径目前 0 可用性。

正确处理顺序：先修 1 / 2 让它能构建能起，再补 3 让数据面非空，
最后修 4 让脚本和文档与实际一致。
