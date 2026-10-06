#!/usr/bin/env python3
"""
lint-compose.py — docker-compose-mvp.yml 的静态不变量检查

为什么需要它：2026-10-04 修的 12 处缺陷里，有几处**读代码看不出来**，
只能真正跑起来才暴露。手工修完没有闸门的话，它们会再退化。

本脚本在**不需要构建镜像、不需要起容器**的前提下，检查这些曾经出过事的
不变量。每条都注明它挡住的是哪一次真实事故。

用法:
    python deploy/scripts/lint-compose.py
    # 或在 CI 里
    python deploy/scripts/lint-compose.py && echo "compose 静态检查通过"
"""

import io
import os
import sys
from collections import defaultdict

import yaml

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
# 可选位置参数: [compose_path] [envoy_path]
# 加这个是为了能对**历史版本**的编排做双向验证——把旧 compose 喂进来，
# 必须报出当年那些违规。只看当前版本"全绿"证明不了检查还有牙齿。
COMPOSE = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
    ROOT, "deploy", "docker-compose-mvp.yml")
ENVOY = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
    ROOT, "deploy", "envoy-mvp.yaml")

errors = []
notes = []


def err(msg):
    errors.append(msg)


def note(msg):
    notes.append(msg)


with io.open(COMPOSE, encoding="utf-8") as f:
    doc = yaml.safe_load(f)

services = doc.get("services") or {}

# 启动时会执行某个特定 binary 的 service。
# 这些必须显式写 command:——否则会继承 Dockerfile.runtime 的
# `CMD ["/usr/local/bin/cats-bff"]`，容器里跑的就不是它自己了。
#
# 真实事故: ai-gateway 没有 command，容器里跑的是 cats-bff，
# 于是同一份编排里出现两个 cats-bff 而 ai-gateway 根本没启动。
# 它的 REST_BIND_ADDR / GRPC_BIND_ADDR 无人读取，实际监听 8097。
MUST_DECLARE_COMMAND = [
    "ai-gateway", "translation-core", "auth-service", "user-service",
    "project-service", "task-service", "file-service", "notification-service",
    "report-service", "audit-service", "worker-service", "cats-bff",
]

for name in MUST_DECLARE_COMMAND:
    s = services.get(name)
    if s is None:
        err("service '%s' 不存在（ MUST_DECLARE_COMMAND 里列了它）" % name)
        continue
    if not s.get("command"):
        err("service '%s' 没有显式 command —— 会继承镜像 CMD"
            "（即跑成 cats-bff）。这是 ai-gateway 踩过的坑。" % name)

# 宿主端口不能重复
# 真实事故: ai-gateway 与 cats-bff 都映射 127.0.0.1:8090，
# docker compose up 直接报 "port is already allocated"。
host_ports = defaultdict(list)
for name, s in services.items():
    for p in s.get("ports") or []:
        if not isinstance(p, str):
            continue
        parts = p.split(":")
        if len(parts) < 2:
            err("service '%s' 的端口声明 '%s' 解析不了" % (name, p))
            continue
        # 形如 "127.0.0.1:8090:8090" 或 "8090:8090"
        # 宿主端口是**倒数第二段**——从左边数会被 IP 前缀带偏
        hport = parts[-2]
        if not hport.isdigit():
            err("service '%s' 的端口声明 '%s' 里宿主端口不是数字" % (name, p))
            continue
        host_ports[int(hport)].append(name)
for port, owners in sorted(host_ports.items()):
    if len(owners) > 1:
        err("宿主端口 %d 被多个 service 声明: %s —— up 会失败"
            % (port, ", ".join(sorted(owners))))

# 有 DATABASE_URL 的 service，其库必须被 db-init 建出来
# 真实事故: compose 只建了默认库 postgres，而 8 个 service 的 DATABASE_URL
# 指向 auth_db / user_db / ... 没有人创建 —— 服务能起、healthz 能过，
# 第一个业务请求就是 relation does not exist。
init_script = os.path.join(ROOT, "deploy", "scripts", "init-databases.sh")
with io.open(init_script, encoding="utf-8") as f:
    init_src = f.read()

db_users = {}
for name, s in services.items():
    url = ((s.get("environment") or {}) or {}).get("DATABASE_URL")
    if not url:
        continue
    db = url.rsplit("/", 1)[-1].split("?")[0]
    db_users.setdefault(db, []).append(name)

for db, users in sorted(db_users.items()):
    if db == "postgres":
        continue          # 服务直连默认库是允许的
    # 在 init-databases.sh 里查这个名字是否被建过
    if ("[%s]" % db) not in init_src and ('"%s"' % db) not in init_src:
        err("库 '%s' 被 %s 使用，但 init-databases.sh 不会创建它"
            % (db, ", ".join(sorted(users))))
    else:
        note("库 %-20s <- %s" % (db, ", ".join(sorted(users))))

# 每个依赖数据库的 service 都必须等 db-init 完成
# 少了这个，compose 会并行起服务和建库，出现竞态。
for db, users in sorted(db_users.items()):
    if db == "postgres":
        continue
    for name in users:
        dep = (services.get(name) or {}).get("depends_on") or {}
        cond = dep.get("db-init") if isinstance(dep, dict) else None
        if isinstance(dep, dict) and "db-init" not in dep:
            err("service '%s' 用了库 '%s' 但 depends_on 里没有 db-init"
                % (name, db))

# 声明了 *_TOPIC 的 service 必须等 kafka-init（topic 是显式建的）
# 真实事故: KAFKA_AUTO_CREATE_TOPICS_ENABLE=false，topic 不显式建就收不到消息。
#
# 更正（2026-10-04）：本规则原先的注释写的是"两个 consumer 各自等一个 topic"，
# 那是错的。全仓只有 audit-service 读 *_TOPIC，而且它的 consumer 走的是
# REST proxy（KAFKA_REST_URL），不是 broker；notification-service 当时
# 声明的 KAFKA_NOTIFICATIONS_TOPIC 根本没有任何源码读过。注释比代码更自信，
# 会让下一个人以为 Kafka 已经接通。
for name, s in services.items():
    env = (s.get("environment") or {}) or {}
    topics = [v for k, v in env.items() if k.endswith("_TOPIC")]
    if not topics:
        continue
    dep = s.get("depends_on") or {}
    if isinstance(dep, dict) and "kafka-init" not in dep:
        err("service '%s' 监听 topic %s，但 depends_on 里没有 kafka-init"
            % (name, topics))

# envoy 配置里每个 http_filter 都必须有 typed_config
# 真实事故: envoy.filters.http.router 只写了 name 没写 @type，
# envoy 启动即崩: "Didn't find a registered implementation for
# 'envoy.filters.http.router' with type URL: ''"
with io.open(ENVOY, encoding="utf-8") as f:
    envoy = yaml.safe_load(f)
for lst in (envoy.get("static_resources") or {}).get("listeners") or []:
    for fc in lst.get("filter_chains") or []:
        for filt in fc.get("filters") or []:
            tcfg = ((filt.get("typed_config") or {})
                    .get("@type")
                    if isinstance(filt.get("typed_config"), dict)
                    else None)
            if not tcfg:
                err("envoy filter '%s' 缺 typed_config.@type —— envoy 会启动即崩"
                    % filt.get("name"))
            http_filters = ((filt.get("typed_config") or {})
                            .get("http_filters")
                            if isinstance(filt.get("typed_config"), dict)
                            else None) or []
            for hf in http_filters:
                if not isinstance(hf.get("typed_config"), dict):
                    err("envoy http_filter '%s' 缺 typed_config —— envoy 会启动即崩"
                        % hf.get("name"))

# compose 注入的每个 env 变量，必须真的被该 service 的源码读过
#
# 真实事故（2 处，都在这一条上）:
#   1. cats-bff: compose 写 BIND_ADDR，而 config.rs 读的是 SERVICE_BIND_ADDR
#      —— 于是 bff 默默用了默认 8097，端口映射指向无人监听的 8080。
#   2. compose 给 cats-bff 设的 TASK_SERVICE_URL / USER_SERVICE_URL 根本没生效，
#      因为变量名在源码里根本不存在（对应缺失该变量的默认值指向 localhost）。
#
# 反过来也要查: 源码里 `env::var("X")` **没有默认值**（没跟 unwrap_or_else）
# 而 compose 又没提供 X —— 那种情况运行期会直接报错。
#
# 这一条把"人肉比对变量名"变成机械检查。变量名写错是拼写级错误，
# 但后果是整个 service 静默走默认值——最难自查的一类。
#
# 2026-10-04 扩展后又抓到 8 处，其中 1 处是真 P0:
#   translation-core: compose 与架构书给 GRPC_BIND_ADDR=0.0.0.0:50051，
#   而 main.rs 只读 BIND_ADDR（默认 8090）—— 容器实际监听 8090，宿主 50051
#   映射到的端口上什么都没有，而 worker-service 与 cats-bff 的
#   TRANSLATION_CORE_URL 都指向 translation-core:50051。整条链路静默不通。
import glob
import re

ENV_RE = re.compile(r'env::var\(\s*"([A-Z0-9_]+)"\s*\)')
HAS_DEFAULT_RE = re.compile(r'env::var\(\s*"([A-Z0-9_]+)"\s*\)\s*\.')
# 这些是标准库/运行时变量，不是业务配置，不参与比对
IGNORED_ENV = {"CARGO_HOME", "RUST_LOG", "PATH", "HOME", "HOSTNAME", "PWD",
               "PROTOC", "CARGO_TARGET_DIR"}

# service 目录名 -> compose 里的 service 名（默认同名）
SERVICE_DIR = {
    "ai-gateway": "cats-ai-gateway",
    "translation-core": "translation-core",
    "cats-bff": "cats-bff",
}

# crate 内的 path 依赖写法有两种:
#   foo = { path = "../common" }
#   foo.path = "../common"
PATH_DEP_INLINE = re.compile(r'^\s*([A-Za-z0-9_-]+)\s*=\s*\{[^}]*?path\s*=\s*"([^"]+)"',
                             re.M)
PATH_DEP_DOTTED = re.compile(r'^\s*([A-Za-z0-9_-]+)\.path\s*=\s*"([^"]+)"', re.M)


def in_repo_src_dirs(crate_dir):
    """本 crate 的 src，加上它在仓内 path 依赖的 src。

    只扫自己的 src 会误报：service 把配置读取委托给共享 crate 时，
    变量在依赖里而不在自己这里。宁可多扫，不可错杀。
    """
    dirs = [os.path.join(crate_dir, "src")]
    cargo_toml = os.path.join(crate_dir, "Cargo.toml")
    if not os.path.isfile(cargo_toml):
        return dirs
    with io.open(cargo_toml, encoding="utf-8") as f:
        raw = f.read()
    paths = [m.group(2) for m in PATH_DEP_INLINE.finditer(raw)]
    paths += [m.group(2) for m in PATH_DEP_DOTTED.finditer(raw)]
    for rel in paths:
        dep_dir = os.path.normpath(os.path.join(crate_dir, rel))
        dep_src = os.path.join(dep_dir, "src")
        if os.path.isdir(dep_src):
            dirs.append(dep_src)
    return dirs


for name, s in sorted(services.items()):
    env_block = (s.get("environment") or {}) or {}
    if not env_block:
        continue
    dirname = SERVICE_DIR.get(name, name)
    crate_dir = os.path.join(ROOT, "crates", dirname)
    if not os.path.isdir(os.path.join(crate_dir, "src")):
        continue

    # 收集该 service（含仓内 path 依赖）源码里出现过的 env::var("X")
    read_vars = set()
    no_default = set()
    for src_dir in in_repo_src_dirs(crate_dir):
        for rs in glob.glob(os.path.join(src_dir, "**", "*.rs"), recursive=True):
            with io.open(rs, encoding="utf-8") as f:
                src = f.read()
            read_vars.update(ENV_RE.findall(src))
            for m in re.finditer(r'env::var\(\s*"([A-Z0-9_]+)"\s*\)(.*)', src, re.S):
                # env::var("X").map(...) / .unwrap_or_else(...) 都算有默认值；
                # 只跟到本行/分号为止的粗略判断，宁可少报也不误报
                tail = m.group(2)[:120]
                if ".unwrap_or" in tail or ".map(" in tail or ".ok()" in tail:
                    continue
                no_default.add(m.group(1))

    for var in sorted(env_block):
        if var in IGNORED_ENV:
            continue
        if var not in read_vars:
            err("service '%s' 设了环境变量 %s，但 %s 的源码里**没有任何 env::var(\"%s\")**"
                " —— 变量名对不上，配置会被静默忽略" % (name, var, dirname, var))

    for var in sorted(no_default - IGNORED_ENV):
        if var not in env_block:
            note("注意: %s 读 %s 且无默认值，compose 未提供" % (dirname, var))

# ---------------------------------------------------------------------
# 规则 7: src/ 下每个 .rs 文件都必须真的被编译
#
# 真实事故（2026-10-04 实证）: 12 个 crate 里有 5731 行 .rs 从未被编译。
# Rust 2018 起必须显式 `mod` 声明，而这一批 crate 的 lib.rs 只声明了
# version()/name()，磁盘上却躺着一整套业务实现:
#
#   translation-core  service.rs(174 行真实编排逻辑) / qa.rs / db.rs /
#                     tm.rs / glossary.rs / ai_gateway.rs —— 共 550 行
#   worker-service    scheduler.rs(84 行 run_scheduler_loop) / state.rs /
#                     handlers.rs —— 共 170 行
#   common            error.rs(550 行) —— lib.rs 里另有一份 inline 的 CatsError
#   cats-mock         2899 行, lib.rs 文档宣称提供 http/db/infra/data 四个模块
#   其余 8 个 crate   各自的 state.rs / models.rs 等
#
# 【更正 · 2026-10-05】上面这两行是**假账**，不要当作现状：
#   - cats-mock 那 2899 行**从来不是孤儿**。`cats-mock/src/lib.rs` 一直声明着
#     `pub mod data; pub mod db; pub mod http; pub mod infra; pub mod smoke;`，
#     而 `cargo check -p cats-mock` 一直通过。它会被误报，是因为
#     reachable_rs 的可达性一行 bug（把 `mod` 声明恒定相对 src_dir
#     解析，从不进入子目录），已修。详见下方 ORPHAN_BASELINE
#     内的撤回记录。
#   - "其余 8 个 crate 各自的 state.rs / models.rs"也不成立：
#     逐项查过后，5 个 state.rs 已于 2026-10-05 删除（见下方
#     ORPHAN_BASELINE 的逐项调查结论），models.rs 则从未被证实为孤儿。
#
# 为什么难发现: 文件在、代码完整、注释和文档都像那么回事，
# 但 binary 启动后只注册 /healthz。487 个测试和 12 路 healthz 全绿，
# 都不能证明这些代码存在——因为它们根本没进编译。
#
# 实证方式: 往 error.rs 里注入一行语法错误后 `cargo check -p cats-common`
# 仍然 15.55s 成功退出 0。若它真被编译，rustc 必然报错。
#
# 已知存量列入基线（gate new violations），清债方式见 v0.4 §未接项。
ORPHAN_BASELINE = {
    # 2026-10-05: upstream_passthrough.rs 已接线（第 1 步）。
    # 剩余 4 项仍未接入 —— routes.rs / grpc_clients.rs 是另两步，
    # upstream/{auth,projects,tasks}.rs 一直被 upstream/mod.rs 声明着，
    # 它们留在基线里是 lint 自身的历史遗留（见 BACKEND_STATUS_v0.4 §4.1c）。
    # 2026-10-05: cats-bff 三步全部接完，孤儿清零。
    #   1) upstream_passthrough.rs  2) grpc_clients.rs  3) routes.rs

    # 2026-10-05 移除 cats-mock 整项（原 12 个文件 / 2899 行）。
    #
    # 那一条**全是 reachable_rs 的 bug 造出来的假账**，不是真孤儿：
    # `cats-mock/src/lib.rs` 一直声明着
    #     pub mod data; pub mod db; pub mod http; pub mod infra; pub mod smoke;
    # 而旧实现恒把 `mod` 声明相对 `src_dir` 解析，于是 `mod data;` 去找
    # `src/data.rs`（不存在），**从不进入 `src/data/` 目录**。
    # 证据：`cargo check -p cats-mock` 一直通过 —— 真孤儿编不过。
    #
    # 也就是说 BACKEND_STATUS_v0.4 §4.1 里「cats-mock 2899 行孤儿
    # （lib.rs 宣称四模块却一个都没声明）」这句**是错的**，在此显式撤回。
    # 真正的孤儿存量因此从「7 crate / 3963 行」降到「6 crate / 558 行」。
    # 2026-10-05 移除 6 项（file-service/storage.rs + 5 个 state.rs，合计 155 行）。
    #
    # 逐项调查结论 —— 这 6 个文件**不是"等着被接线"的半成品，而是已经被
    # 取代的旧稿或零引用的第二份定义**。接线只会让门禁变绿，不会让代码变活。
    #
    # 1) file-service/src/storage.rs（63 行）
    #    被 src/handlers.rs 内联重写取代。两套实现的语义直接冲突：
    #      - 分区键：storage.rs 按 org_id，handlers.rs 按 workspace_id
    #      - 根目录：storage.rs 硬编码相对路径 var/files/（相对 cwd），
    #        handlers.rs 读 FILE_STORAGE_ROOT，缺省落到 temp_dir()/file-storage
    #      - 文件名：storage.rs 把用户传入的 name 拼进路径并过滤 `/ \ . \0`，
    #        handlers.rs 用纯 uuid + .bin
    #    权威是 workspace_id：migrations/20260920_0001_init.sql 建表用
    #    workspace_id + storage_path，索引建在 (workspace_id, sha256) 上；
    #    两份更早的迁移（0001_init_file_db.sql / 20260919_0001_init.sql）
    #    注释里明确写了 org_id 是"旧版约定，已被 workspace_id 取代"。
    #    附带删掉 2 个假测试：ensure_base_dir_succeeds 算了路径、删了目录、
    #    设了环境变量，但从未调用被测函数；path_traversal_protection_strips_dots
    #    把过滤逻辑抄了一遍而非调用 write_file，改生产代码它照样绿。
    #
    # 2) file-service / notification-service / project-service / report-service
    #    各自的 state.rs（17~24 行）
    #    AppState 只在文件内部出现，crate 内无任何其它引用；4 个 crate 的
    #    main.rs 都是**分别**注册 web::Data::new(pool) 和
    #    web::Data::new(rbac_checker) 两个独立参数 —— 活设计本来就不是
    #    "一个合并的 AppState"。
    #    关键：给 lib crate 加 `pub mod state;` **不会**产生 dead_code 警告
    #    （pub 且从 crate root 可达的项不被标记），所以"加 mod"的效果是
    #    把"零调用方、门禁看得见"变成"零调用方、门禁看不见"。
    #
    # 3) task-service/src/state.rs（17 行）
    #    与上面 4 个不同：权威已经定了。task-service 的 src/handlers.rs 自己
    #    定义了一个 AppState 且它是活的 —— web::Data<AppState> 出现 6 次，
    #    main.rs 确实注册了 web::Data::new(state)。这个孤儿文件是给一个
    #    "已经存在且正在使用"的类型又写了一份定义。
    #
    "notification-service": ["consumer.rs"],
    # 2026-10-05 保留 consumer.rs（69 行）：它**不是**旧稿。
    #   process_event 是真代码（反序列化 KafkaEvent → db::insert_from_event 真落库），
    #   kafka_event_round_trip 是真测试。但 run_consumer_loop 依赖的 poll_once
    #   恒返回 Ok(0)，接上去等于多 spawn 一个永远 5 秒一轮空转的任务；
    #   main.rs 目前完全没 spawn 任何 consumer。文件自己的文档注释写明
    #   "真实 Kafka 集成留 Sprint 2" —— 它自己声明是占位符。
    #   保留在基线里，正是为了避免"占位符被当成已完成"。
    # 已于 2026-10-04 接线、因此从基线移除的有 4 个 crate:
    #   common            error.rs（550 行共享错误体系）
    #   cats-rbac         service_helpers.rs（178 行）
    #   translation-core  6 个模块（550 行，含 4 个 RPC 的 gRPC 实现）
    #   worker-service    3 个模块（170 行，含抢占→派发→回写调度器）
    # 合计约 1450 行从"写在磁盘上"变成"真的在编译"。
    # 它们若再次掉出编译，规则 7 会重新 FAIL —— 这正是基线的用法。
}

MOD_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;', re.M)


def _mod_search_dirs(path, src_dir):
    """一个文件里的 `mod x;` 可能落在哪些目录（Rust 2018 规则）。

    - `src/lib.rs` / `src/main.rs` / `src/foo/mod.rs` → 同目录
    - `src/foo.rs`                        → 先 `src/foo/`，再退回 `src/`（旧式）
    """
    base = os.path.dirname(path)
    stem = os.path.basename(path)
    if stem in ("lib.rs", "main.rs", "mod.rs"):
        return [base]
    return [os.path.join(base, stem[:-3]), base]


def reachable_rs(src_dir):
    """从 lib.rs / main.rs 出发做传递闭包，返回真正会编译的相对路径集合。

    2026-10-05 修：**`mod` 声明必须相对「当前文件所在目录」解析，不能恒定
    相对 `src_dir`。**

    原实现恒用 `os.path.join(src_dir, ...)`，于是走到 `src/upstream/mod.rs`
    时，它的 `pub mod auth;` 被解析成 `src/auth.rs` —— 不存在，于是**不往下
    走**。结果 `upstream/{auth,projects,tasks}.rs` 明明被 `upstream/mod.rs`
    声明着（crate 能构建就是证明），却被规则 7 报成孤儿。

    这个 bug 的危害不是"多报了 3 个文件"，而是**它会掩盖真正的孤儿**：只要一个
    crate 用子目录组织模块，那整个子目录的可达性就是瞎的 —— 里面真躺着一个
    没被声明的文件，规则 7 也看不见。

    顺带按 Rust 2018 规则补上 `src/foo.rs` 声明 `mod bar;` 解析到
    `src/foo/bar.rs` 的情形（旧式 `src/bar.rs` 仍作回退）。
    """
    found = set()
    frontier = [os.path.join(src_dir, n) for n in ("lib.rs", "main.rs")
                if os.path.isfile(os.path.join(src_dir, n))]
    while frontier:
        path = frontier.pop()
        rel = os.path.relpath(path, src_dir).replace("\\", "/")
        if rel in found:
            continue
        found.add(rel)
        with io.open(path, encoding="utf-8") as f:
            mods = MOD_RE.findall(f.read())
        for m in mods:
            placed = False
            for d in _mod_search_dirs(path, src_dir):
                for cand in (m + ".rs", m + "/mod.rs"):
                    cp = os.path.join(d, *cand.split("/"))
                    if os.path.isfile(cp):
                        frontier.append(cp)
                        placed = True
                        break
                if placed:
                    break
    return found


crates_root = os.path.join(ROOT, "crates")
orphan_total = 0
orphan_crates = 0
for crate_name in sorted(os.listdir(crates_root)):
    src_dir = os.path.join(crates_root, crate_name, "src")
    if not os.path.isdir(src_dir):
        continue
    on_disk = set()
    for dp, _, fns in os.walk(src_dir):
        for fn in fns:
            if fn.endswith(".rs"):
                on_disk.add(os.path.relpath(os.path.join(dp, fn), src_dir)
                            .replace("\\", "/"))
    if not on_disk:
        continue
    orphans = sorted(on_disk - reachable_rs(src_dir))
    if not orphans:
        continue
    baseline = set(ORPHAN_BASELINE.get(crate_name, []))
    new_ones = [o for o in orphans if o not in baseline]
    stale = sorted(baseline - set(orphans))
    orphan_crates += 1
    orphan_total += sum(
        len(io.open(os.path.join(src_dir, *o.split("/")), encoding="utf-8")
            .read().splitlines())
        for o in orphans
    )
    for o in new_ones:
        err("crate '%s' 的 src/%s 没被任何 mod 声明引用 —— 写了但永远不会编译"
            % (crate_name, o))
    for o in stale:
        err("基线里列了 crate '%s' 的 src/%s，但它现在已被编译，"
            "请把它从 ORPHAN_BASELINE 里删掉（否则基线会腐烂）"
            % (crate_name, o))

if orphan_crates:
    note("存量死代码: %d 个 crate 共 %d 行 .rs 从未编译（已列入 ORPHAN_BASELINE，"
         "新增会 FAIL）" % (orphan_crates, orphan_total))

# ---------------------------------------------------------------------
# 规则 8: 每个产出二进制的 crate 都必须在 ci-docker-build 的
# matrix 里——否则部署侧拿不到镜像
#
# 2026-10-05 新增。发现过的真实缺陷：cats-ai-gateway 一直有
# src/main.rs（产出二进制），也被 compose 声明为一个服务，但**不在**
# ci-docker-build 的 matrix 里。它因此永远没有对应的 Harbor 镜像。
#
# 为什么一直没人发现：本地 compose 走的是**另一条路径**——
# `deploy/Dockerfile.runtime` 产出单一个含**全部二进制**的
# `cats-runtime:latest`，所以本地 `docker compose up` 一切都能跑。
# 只有 CI 输出的 per-service 镜像才会显出缺口。
#
# 所以这条规则的价值不在于“查出一个有问题的服务”，
# 而在于**把一类静默漂移变成可发现的**：以后新增任何服务
# crate，未加入 matrix 就会 FAIL。
NOT_A_SERVICE = {
    # m1-s0-smoke 产出二进制，但它是烟雾测试工具，不是一个需要
    # 镜像的服务。需要新增时在这里加一行并写下理由——
    # 本意是强制一次有意识的决定，而不是默认忽略。
    "m1-s0-smoke",
}


def _service_crates():
    """ci-docker-build matrix 里列出的 crate 名。"""
    p = os.path.join(ROOT, ".github", "workflows", "ci-docker-build.yaml")
    if not os.path.isfile(p):
        return None
    txt = io.open(p, encoding="utf-8").read()
    # matrix 里的写法是 `- service: X` 换行后 `crate: Y`，
    # 不是同一行的 `- crate: X`——之前的正则只能匹配
    # 它们之间的 `-` 和 `service:`，所以始终返回空集，
    # 规则 8 会把全部 18 个 crate 都报成缺失。
    return set(re.findall(r"(?m)^\s*crate:\s*(\S+)\s*$", txt))


def _binary_crates():
    """workspace 里所有有 src/main.rs 的 crate（即会产出可执行文件的）。"""
    p = os.path.join(ROOT, "Cargo.toml")
    if not os.path.isfile(p):
        return set()
    txt = io.open(p, encoding="utf-8").read()
    i = txt.find("members")
    j = txt.find("]", i)
    if i < 0 or j < 0:
        return set()
    out = set()
    for name in re.findall(r'"crates/([^"]+)"', txt[i:j]):
        if os.path.isfile(os.path.join(ROOT, "crates", name, "src", "main.rs")):
            out.add(name)
    return out


_matrix = _service_crates()
if _matrix is None:
    note("规则 8 跳过：找不到 .github/workflows/ci-docker-build.yaml")
else:
    _bins = _binary_crates() - NOT_A_SERVICE
    for _missing in sorted(_bins - _matrix):
        err("crate '%s' 会产出二进制，但不在 ci-docker-build 的 matrix 里"
            " —— 部署侧拿不到它的镜像" % _missing)
    for _stale in sorted(_matrix - _bins):
        err("ci-docker-build matrix 里的 crate '%s' 不再产出二进制"
            "（src/main.rs 没了）—— 请从 matrix 里删掉" % _stale)

# ---------------------------------------------------------------------
# 规则 9: 全仓库零引用的 pub 函数
#
# 规则 7 管的是"文件级"孤儿（.rs 从未被编译）；这一条管**函数级**：
# 文件在编译、pub 函数也在编译，但没有任何调用方——既无生产调用，也无
# 任何测试引用（内联 #[cfg(test)] mod tests 与 tests/ 目录同样算引用）。
#
# 2026-10-06 新增。一次性清掉 22 个（20 删 + 2 接线），基线因此为空。
# 详见 BACKEND_STATUS_v0.4 §4.1j。
#
# 扫描算法：不用正则找调用点，而是**一次遍历收集每个候选名在全仓库的出现位置**，
# 再把命中按"是否落在 #[cfg(test)] 块内"分成 DEF / PROD-USE / TEST-USE，
# 只有 DEF>=1 且 PROD-USE=0 且 TEST-USE=0 的才算零引用。
#
#   （早前两版扫描器分别漏掉裸调用 `foo(x)`、以及只统计 tests/ 目录而漏掉
#     内联 #[cfg(test)] mod tests。两次都把"已被调用"误报成"零引用"。）

_DEADCODE_MIN_PUB_FNS = 120

# 名称本身由 derive / trait / 框架消费，不存在文本调用点
_DEADCODE_SKIP = {
    "new", "default", "from", "into", "from_str", "from_slice", "clone", "fmt",
    "get", "set", "iter", "next", "build", "run", "start", "main", "as_ref",
    "as_str", "as_bytes", "as_mut", "drop", "eq", "ne", "hash", "deref",
    "deref_mut", "to_string", "to_vec", "to_path_buf", "clone_from",
    "serialize", "deserialize", "source", "cause", "add", "sub", "not",
    "serialize_struct", "serialize_field", "end", "visit_str", "visit_map",
}

# 已知存量。新增即 FAIL；本表里已不成立的条目同样 FAIL（要顺手删掉）。
# 新增例外必须在这里加一行并写下理由，强制一次有意识的决定。
DEADCODE_BASELINE = {
    # 2026-10-06 一次性清零：22 个（20 删 + 2 接线），故基线为空。
}


def _cfg_test_lines(lines):
    """{行号: 是否在 #[cfg(test)] 块内}，按大括号配平计算。"""
    flags = {}
    in_test = False
    depth = 0
    i = 0
    while i < len(lines):
        line = lines[i]
        if not in_test:
            if re.search(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", line):
                j = i
                while j < len(lines) and "{" not in lines[j]:
                    j += 1
                if j < len(lines):
                    depth = lines[j].count("{") - lines[j].count("}")
                    in_test = True
                    for k in range(i, j + 1):
                        flags[k + 1] = True
                    i = j + 1
                    continue
        else:
            depth += line.count("{") - line.count("}")
            flags[i + 1] = True
            if depth <= 0:
                in_test = False
        i += 1
    for k in range(1, len(lines) + 1):
        flags.setdefault(k, False)
    return flags


_FN_DEF_RE = re.compile(r"^\s*pub(?:\([^)]*\))?\s+(?:default\s+)?(?:const\s+)?"
                        r"(?:async\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)")
_IDENT_RE = re.compile(r"[A-Za-z_][A-Za-z0-9_]*")


def _deadcode_scan():
    """返回 (零引用清单, 收集到的 pub fn 总数, 扫描文件数)"""
    # 未编译文件由规则 7 的 ORPHAN_BASELINE 记账，不参与本规则（否则重复报）
    orphan = set()
    for crate, names in ORPHAN_BASELINE.items():
        sub = SERVICE_DIR.get(crate, crate)
        for n in names:
            orphan.add("crates/%s/src/%s" % (sub, n))

    files = []
    for dp, dn, fns in os.walk(os.path.join(ROOT, "crates")):
        dn[:] = [d for d in dn if d not in ("target", ".git")]
        for fn in fns:
            if not fn.endswith(".rs"):
                continue
            full = os.path.join(dp, fn)
            rel = os.path.relpath(full, ROOT).replace("\\", "/")
            if rel in orphan:
                continue
            lines = io.open(full, encoding="utf-8", errors="replace").read().splitlines()
            files.append((rel, lines, _cfg_test_lines(lines)))

    # 第一遍：所有 pub fn 定义
    defs = defaultdict(list)
    for rel, lines, _flags in files:
        for i, line in enumerate(lines, 1):
            m = _FN_DEF_RE.match(line)
            if m:
                defs[m.group(1)].append((rel, i))

    candidates = set(n for n in defs
                     if n not in _DEADCODE_SKIP and not n.startswith("test_"))

    # 第二遍：一次遍历收集每个候选名的出现位置
    occ = defaultdict(list)
    for fi, (_rel, lines, _flags) in enumerate(files):
        for i, line in enumerate(lines, 1):
            for m in _IDENT_RE.finditer(line):
                nm = m.group(0)
                if nm in candidates:
                    occ[nm].append((fi, i))

    zero = []
    for nm, ds in defs.items():
        if nm not in candidates:
            continue
        n_def = n_prod = n_test = 0
        for fi, i in occ.get(nm, []):
            rel, lines, flags = files[fi]
            line = lines[i - 1]
            if (re.search(r"\bfn\s+%s\b" % re.escape(nm), line)
                    and not re.search(r"::\s*fn\s+%s\b" % re.escape(nm), line)):
                n_def += 1
            elif flags.get(i) or "/tests/" in rel:
                n_test += 1
            else:
                n_prod += 1
        if n_def >= 1 and n_prod == 0 and n_test == 0:
            for rel, i in ds:
                zero.append((rel, i, nm))
    zero.sort()
    return zero, len(defs), len(files)


_dead, _n_defs, _n_files = _deadcode_scan()

# 扫描器有效性断言。若 FN_DEF 正则失效 / crates/ 路径写错，一个 pub 函数都
# 收集不到，零引用清单自然是空，下面两条比较都会得到空集 => **门禁恒绿**。
# 这与规则 8 第一版的失效方式同型（正则恒返回空集），见 §4.1h。
if _n_defs < _DEADCODE_MIN_PUB_FNS:
    err("规则 9 失效：只收集到 %d 个 pub fn（阈值 %d）——扫描器大概率没在查任何东西。"
        "请检查 FN_DEF 正则与 crates/ 路径；**空结果不等于没有问题**"
        % (_n_defs, _DEADCODE_MIN_PUB_FNS))
else:
    _found = set(n for _r, _l, n in _dead)
    for _nm in sorted(_found - set(DEADCODE_BASELINE)):
        _where = ["%s:%d" % (r, l) for r, l, n in _dead if n == _nm]
        err("pub fn `%s()` 全仓库零调用方（%d 个定义，prod=0 test=0）: %s"
            " —— 请删掉或接线；**不要用 #[allow(dead_code)] 把门禁刷绿**"
            % (_nm, len(_where), ", ".join(_where[:3])))
    for _nm in sorted(set(DEADCODE_BASELINE) - _found):
        err("DEADCODE_BASELINE 里列了 `%s()`，但它现在已有调用方了，"
            "请把它从基线里删掉（否则基线会腐烂）" % _nm)
_deadcode_n = len(_dead)

# ---------------------------------------------------------------------
# 规则 10: `GET /healthz` 的响应形状全仓统一
#
# 真实事故（2026-10-07 之前，18 个注册了 `/healthz` 的服务对"自己叫什么"
# 有 4 种不同答案）:
#   8 个  {"status":"ok","app":{"name":...,"version":...}}   —— 正确
#   5 个  {"status":"ok","name":...,"version":...}           —— key 是 name 不是 service
#   3 个  {"status":"ok","service":...}                      —— 连 version 都没有
#   1 个  {"status":"ok","service":...,"version":...,"bind_addr":...,"upstreams":{5 个上游}}
#
# 运行期后果不是报错，是**静默的 null**: 任何读 `.service` 的监控脚本在那 5 个
# 服务上拿到 null —— 没有异常、没有告警，只是少了数据。更糟的是那 3 个连
# version 都没有，于是"这个响应是哪一版程序吐的"根本无法回答。
#
# 唯一形状（2026-10-07 起全仓统一）:
#   {"status":"ok","app":{"name":"<CARGO_PKG_NAME>","version":"<CARGO_PKG_VERSION>"}}
#
# `env!("CARGO_PKG_NAME")` 是这条规则的关键: 它按**本 crate** 在编译期展开，
# 所以服务自报的一定是自己，而不是共享库 cats-common 的名字。2026-10-05 修过
# 10 处"自报 cats-common"的同类问题（见 crates/audit-service/tests/
# rbac_audit_read.rs 的回归钉子），本条是它的静态版本。
#
# `upstreams` / `bind_addr` 一律不许回来 —— 它们是内部拓扑，全仓没有任何
# 消费者，而且换个部署形态就过期。
#
# 【静态检查的边界】本条证明的是"源码里写的是这个形状"，**不等于运行时真的
# 返回了它**。要证明端点确实这么回，仍然必须 up 一次、curl 一次。
#
# 不在本条范围内: 形状之外的 healthz 语义（是否真的探活 DB 等）由 /readyz 负责。
HEALTHZ_SKIP = {
    # cats-mock 是给集成测试用的 mock server，不是部署单元（规则 8 的 matrix
    # 里也没有它）。它的 /healthz 故意只有 {"status":"ok"}，而且 handler 是
    # 一个闭包而不是具名函数 —— 被它 mock 的正是"服务自己"，测试替身不该长得
    # 比被替身复杂。
    "cats-mock": "集成测试的 mock server，不是部署单元；healthz 只有 status",
    # m1-s0-smoke 是烟雾测试工具二进制（规则 8 的 NOT_A_SERVICE 里也列了它，
    # 同一份"有意识的例外"记两处）。它的 `#[get("/healthz")]` 只用来确认
    # actix 起来了，不是服务探针。
    "m1-s0-smoke": "烟雾测试二进制，不是服务；#[get(\"/healthz\")] 只证明 actix 起得来",
}

# 扫描器有效性断言。2026-10-07 实证: crates/*/src/ 下受本规则检查的
# `/healthz` 注册共 **18 处**，跨 18 个 crate（每个 service crate 恰好一处）；
# 加上 2 个被跳过的非服务 crate（cats-mock / m1-s0-smoke）共 20 处。
#
# 阈值取 16 = 18 往下留 2 处余量给合法删除（某个 service 整体下线）。
# 掉到 16 以下几乎不可能是"服务真的少了"，更可能是 route 正则或 crates/ 路径
# 写坏了 —— 而那会让本条恒绿，是最危险的失效方式（同规则 8/9 的失效方式）。
# 空结果不等于没有问题。
#
# 计数口径是**注册处**而不是 crate 数: 某个 crate 若在同一路径注册两次，两处
# 都会被检查（重复注册本身另有提醒），那时站点数会 > crate 数，这是有意的。
_HEALTHZ_MIN_SITES = 16

# `.route("/healthz", ...)`：ANY 版只认"这里注册了 healthz"（handler 可以是
# 闭包，见 cats-mock），FN 版才去取具名 handler。允许跨行 —— notification-service
# 与 project-service 的 main.rs 把参数拆成了 3 行。
_HEALTHZ_ROUTE_ANY_RE = re.compile(r'\.route\(\s*"/healthz"')
_HEALTHZ_ROUTE_FN_RE = re.compile(
    r'\.route\(\s*"/healthz"\s*,\s*web::get\(\s*\)\s*\.\s*to\(\s*'
    r'(?P<h>[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*)\s*\)')
# 属性宏形式（m1-s0-smoke）: `#[get("/healthz")]`
_HEALTHZ_ATTR_RE = re.compile(r'#\s*\[\s*get\(\s*"/healthz"\s*\)\s*\]')
# 函数**定义**: 必须在语句开头，不能是 `handlers::healthz` / `self.healthz`
# 这种调用点或路径末段（`(?:^|(?<=[{};]))` + re.M 排除 `::` / `.` 前缀）。
_HEALTHZ_FN_DEF_RE = re.compile(
    r'(?:^|(?<=[{};]))\s*'
    r'(?:#\s*\[[^\]\n]*\]\s*)*'
    r'(?:pub(?:\([^)\n]*\))?\s+)?'
    r'(?:default\s+)?(?:const\s+)?(?:async\s+)?(?:unsafe\s+)?'
    r'fn\s+(?P<name>[A-Za-z_][A-Za-z0-9_]*)', re.M)
_HEALTHZ_RAWSTR_RE = re.compile(r'r(#+)?"')

_HEALTHZ_PKG_RE = re.compile(r'env!\(\s*"CARGO_PKG_NAME"\s*\)')
# struct 字段 `app:` 或 json 键 `"app":`。`\b` 挡住 `my_app:`（`_` 是词字符，
# app 前面没有边界）；`"app":` 里 app 与 `:` 之间隔着引号，故要单列一条。
_HEALTHZ_APP_RE = re.compile(r'\bapp\s*:|"app"\s*:')
_HEALTHZ_LEAK_KEYS = ("upstreams", "bind_addr")


def _strip_line_comments(src):
    """丢掉每行 `//` 之后的全部内容，**保留换行**以便行号不变。

    只做行注释，不做 `/* */` 解析（本仓库在这件事上不用块注释）。这一步是
    必需的: 源码里大量注释直接写着 `/healthz`（例如
    crates/cats-ai-gateway/src/main.rs 关于该路径归属的说明），
    不剥掉就会被当成注册点 —— 这正是
    crates/cats-ai-gateway/tests/healthz_single_registration.rs 踩过的坑。

    但**不能**用 `ln.split("//")[0]` 一刀切: 字符串字面量里出现 `//`（典型是
    URL）不是注释。误剪会把该行后半截连同右大括号一起吃掉，于是括号配平失败、
    函数体解析不出来 —— 而"重新长出 upstreams"恰恰就是本规则要抓的改动，
    那 5 个上游 URL 一旦回来，这条规则就会瞎。逐字符跳过字符串/字符字面量
    才能既剥注释又不伤字符串。
    """
    out = []
    for ln in src.splitlines():
        i = 0
        n = len(ln)
        cut = None
        while i < n:
            c = ln[i]
            if c == '"' or (c == "r" and i + 1 < n and ln[i + 1] in '"#'):
                i = _skip_rust_string(ln, i)
                continue
            if c == "'":
                i = _skip_rust_char(ln, i)
                continue
            if c == "/" and ln[i + 1:i + 2] == "/":
                cut = i
                break
            i += 1
        out.append(ln if cut is None else ln[:cut])
    return "\n".join(out)


def _skip_rust_string(code, i):
    """i 指向 `"` 或 raw 串前缀 `r`/`r#`；返回闭引号之后的位置。"""
    if code[i] == "r":
        m = _HEALTHZ_RAWSTR_RE.match(code, i)
        if m:
            close = '"' + m.group(1)
            j = code.find(close, m.end())
            return len(code) if j < 0 else j + len(close)
        return i + 1
    j = i + 1
    while j < len(code):
        if code[j] == "\\":
            j += 2
            continue
        if code[j] == '"':
            return j + 1
        j += 1
    return len(code)


def _skip_rust_char(code, i):
    """i 指向 `'`。字符字面量则整体跳过；生命周期（'static/'a）只前进一格。"""
    if i + 1 < len(code) and code[i + 1] == "\\":
        j = code.find("'", i + 2)
        return len(code) if j < 0 else j + 1
    if i + 2 < len(code) and code[i + 2] == "'":
        return i + 3
    return i + 1


def _fn_body(code, start):
    """从 `fn` 定义处按大括号配平取函数体，跳过字符串 / raw 串 / 字符字面量。

    不配平就会把下一个函数的尾巴也算进来，于是"本函数没有 upstreams"会被
    下一个函数里的 upstreams 误判为违规（或反之）。
    返回 (body, body 起始行号)；找不到大括号返回 (None, None)。
    """
    i = code.find("{", start)
    if i < 0:
        return None, None
    n = len(code)
    depth = 0
    open_i = i
    while i < n:
        c = code[i]
        if c == '"' or (c == "r" and i + 1 < n and code[i + 1] in '"#'):
            i = _skip_rust_string(code, i)
            continue
        if c == "'":
            i = _skip_rust_char(code, i)
            continue
        if c == "{":
            depth += 1
        elif c == "}":
            depth -= 1
            if depth == 0:
                return (code[open_i + 1:i],
                        code.count("\n", 0, open_i) + 1)
        i += 1
    return None, None


_healthz_sites = []          # 受本规则检查的注册点（已剔除 HEALTHZ_SKIP）
_healthz_crates = set()       # 上面这些注册点跨了多少个 crate
_healthz_all_sites = defaultdict(int)   # 每个 crate 的注册点总数（含被跳过的）
_healthz_skip_sites = defaultdict(int)  # 每个被跳过 crate 实际找到的注册点数

for _crate in sorted(os.listdir(crates_root)):
    _src_dir = os.path.join(crates_root, _crate, "src")
    if not os.path.isdir(_src_dir):
        continue
    _code = {}
    for _dp, _dn, _fns in os.walk(_src_dir):
        # 只看生产代码。tests/ 不在 src/ 之下，但仍显式剪掉 src/tests/，
        # 免得将来某个 crate 在那里放测试专用路由把计数抬高。
        _dn[:] = [d for d in _dn if d != "tests"]
        for _fn in sorted(_fns):
            if not _fn.endswith(".rs"):
                continue
            _full = os.path.join(_dp, _fn)
            _rel = os.path.relpath(_full, ROOT).replace("\\", "/")
            with io.open(_full, encoding="utf-8", errors="replace") as _f:
                _code[_rel] = _strip_line_comments(_f.read())

    # 具名函数体索引: fn 名 -> [(相对路径, body, body 行号)]
    _defs = defaultdict(list)
    for _rel in sorted(_code):
        for _m in _HEALTHZ_FN_DEF_RE.finditer(_code[_rel]):
            _body, _bline = _fn_body(_code[_rel], _m.start())
            _defs[_m.group("name")].append((_rel, _body, _bline))

    for _rel in sorted(_code):
        _src = _code[_rel]
        _hits = []          # (注册行号, 解析用的 handler 名或 None)
        for _m in _HEALTHZ_ROUTE_ANY_RE.finditer(_src):
            _fnm = _HEALTHZ_ROUTE_FN_RE.match(_src, _m.start())
            _hits.append((_src.count("\n", 0, _m.start()) + 1,
                          _fnm.group("h") if _fnm else None))
        for _m in _HEALTHZ_ATTR_RE.finditer(_src):
            _fnm = _HEALTHZ_FN_DEF_RE.search(_src, _m.end())
            _hits.append((_src.count("\n", 0, _m.start()) + 1,
                          _fnm.group("name") if _fnm else None))

        for _reg_line, _handler in _hits:
            _healthz_all_sites[_crate] += 1
            if _crate in HEALTHZ_SKIP:
                _healthz_skip_sites[_crate] += 1
                continue
            _healthz_crates.add(_crate)
            _name = _handler.split("::")[-1] if _handler else None
            _hint = _handler.split("::")[-2] if _handler and "::" in _handler else None
            _drel = _body = _bline = None
            if _name:
                # 同文件优先；其次文件里带最后一级模块名（`handlers::healthz` →
                # handlers.rs）；再否则取第一个候选。两处都会检查，取最可能的那个。
                # body 为 None 的候选直接排除 —— 括号没配平（而不是真的没函数体）
                # 不该悄悄当成"通过"，否则解析失败会变成假绿。
                _cands = sorted((c for c in _defs.get(_name, []) if c[1] is not None),
                                key=lambda t: (0 if t[0] == _rel else 1,
                                               0 if _hint and _hint in os.path.basename(t[0]) else 1,
                                               t[0]))
                if _cands:
                    _drel, _body, _bline = _cands[0]
            _healthz_sites.append((_crate, _rel, _reg_line, _drel, _bline, _name))

_healthz_n_sites = len(_healthz_sites)
_healthz_n_crates = len(_healthz_crates)

# 扫描器有效性断言: 正则失效 / crates/ 路径写错 ⇒ 一处都收集不到，下面两条
# 校验都拿到空集 ⇒ **门禁恒绿**。这与规则 8/9 的失效方式同型（§4.1h）。
if _healthz_n_sites < _HEALTHZ_MIN_SITES:
    err("规则 10 失效：crates/*/src/ 下只收集到 %d 处 /healthz 注册"
        "（阈值 %d）—— 扫描器大概率没在查任何东西。"
        "请检查 _HEALTHZ_ROUTE_ANY_RE / crates_root 路径；"
        "**空结果不等于没有问题**" % (_healthz_n_sites, _HEALTHZ_MIN_SITES))
else:
    for _site in _healthz_sites:
        _crate, _reg_rel, _reg_line, _drel, _bline, _name = _site
        if _drel is None:
            err("crate '%s' 的 %s:%d 注册了 /healthz，但解析不到 handler 的函数体"
                "（handler=%s；可能是闭包/method 值，也可能大括号没配平）—— "
                "无法确认它返回统一形状。"
                "若这是有意为之的闭包 handler，请把该 crate 加进 HEALTHZ_SKIP "
                "并写下理由" % (_crate, _reg_rel, _reg_line, _name))
            continue
        _bad = []
        if not _HEALTHZ_PKG_RE.search(_body):
            _bad.append('没有 env!("CARGO_PKG_NAME") —— 会自报共享库的名字而不是本服务')
        if not _HEALTHZ_APP_RE.search(_body):
            _bad.append('没有 app 键（struct 字段 app: 或 json 键 "app":）')
        for _k in _HEALTHZ_LEAK_KEYS:
            if re.search(r"\b%s\b" % _k, _body):
                _bad.append("泄漏内部拓扑字段 %s" % _k)
        if _bad:
            err("crate '%s' 的 /healthz handler `%s()`（注册于 %s:%d，定义于 %s:%d）"
                "不合统一形状：%s —— 统一形状是 "
                '{"status":"ok","app":{"name":..,"version":..}}；'
                "形状不一致会让读 .service 的监控脚本拿到**静默的 null**"
                % (_crate, _name, _reg_rel, _reg_line, _drel, _bline,
                   "；".join(_bad)))

# 例外不能腐烂: 跳过清单里的 crate 一旦不再注册 /healthz，说明例外已失效，
# 必须删掉 —— 否则下一个真的不合规的 healthz 会被这条豁免静默放过。
for _skip_crate, _reason in sorted(HEALTHZ_SKIP.items()):
    if _healthz_skip_sites.get(_skip_crate, 0) == 0:
        err("HEALTHZ_SKIP 里列了 crate '%s'（%s），但它的 src/ 下已经找不到 "
            "/healthz 注册了 —— 请把它从 HEALTHZ_SKIP 删掉（否则例外会腐烂，"
            "下一个真的不合规的 healthz 会被静默放过）" % (_skip_crate, _reason))

# 同一 crate 多于一处注册不是本条的错误（两处都被检查了），但重复注册本身
# 值得看一眼：同一路径注册两次，至少有一处收不到请求。
#
# 哪一处收不到，本仓库**实测过**：actix-web 4.15.0 下**先注册的赢**。证据是
# crates/cats-ai-gateway/tests/healthz_single_registration.rs 的
# `first_registration_wins`（注册两个 body 不同的 handler，返回的是先注册那个），
# 该用例在 CI 上通过。cats-ai-gateway 正是因为把 `/healthz` 注册了两次
# （main.rs 一次、api/mod.rs 一次）而让其中一个 handler 运行时永远收不到请求，
# 而它自己的测试因为只 mount 其中一张表所以一直是绿的 —— 见
# BACKEND_STATUS §4.1n。
#
# 所以这里只陈述"重复了"，不替 actix 下结论：语义可能随版本变，而
# `first_registration_wins` 一旦变红就说明本段结论要重新核。
_healthz_dupe = sorted(c for c in _healthz_all_sites
                       if c not in HEALTHZ_SKIP and _healthz_all_sites[c] > 1)
if _healthz_dupe:
    note("规则 10 附带发现：%s 在 src/ 下各有 %s 处 /healthz 注册 —— "
         "两处都已被本规则检查，但重复注册意味着其中一处收不到请求"
         "（本仓库实测：先注册的赢，见 healthz_single_registration.rs）"
         % (", ".join(_healthz_dupe),
            "/".join(str(_healthz_all_sites[c]) for c in _healthz_dupe)))

# B: 可被外部 diff 的计数出口（LINT_HEALTHZ_COUNT_ONLY=1 时只打印数字，
# 退出 0，不跑其它规则 —— 便于与"18"直接对拍）。
if os.environ.get("LINT_HEALTHZ_COUNT_ONLY"):
    print("healthz_crates=%d healthz_sites=%d healthz_sites_all=%d"
          % (_healthz_n_crates, _healthz_n_sites, sum(_healthz_all_sites.values())))
    sys.exit(0)


# ---------------------------------------------------------------------
# 规则 11: 测试不得再断言统一前的 `/healthz` 形状
#
# 真实事故 (2026-10-07 实证): 把 18 个服务的 healthz 统一到形状 A 之后，
# **本地 `cargo test` 全绿**，CI 的 `e2e (real PostgreSQL)` 却红了
# (`body["service"]` 变成 null)。原因有两层叠加：
#
#  1. 那 5 个断言旧形状的用例全部带 `#[ignore = "e2e-needs-real-pg"]`，
#     本地没有真 PostgreSQL，`cargo test` 直接跳过 —— 本地验证对它们
#     **完全没有覆盖**，而"本地全绿"被当成了证据。
#  2. CI 的 e2e 步骤是 `set -e` + 顺序调用，所以 auth-service 一红，
#     后面的 user / project / file / notification 四个 suite 根本不会跑。
#     一次变更打中 5 个 suite，CI 只报了 1 个，其余 4 个每个都要再等一个
#     CI 周期才暴露。（第 2 点已在 ci-rust-test.yaml 里改成失败累积。）
#
# 规则 11 把第 1 点变成静态检查：无论用例是不是 `#[ignore]`，只要它对
# `/healthz` 的响应体做断言，就不许用统一前的键。
#
# 【一个必须写在这里的坑】判断"用没用旧键"**只能看顶层访问**。
# `body["name"]` 是旧扁平形状，`body["app"]["name"]` 是统一后的形状；
# 朴素的子串搜索分不开这两者 —— 本规则的第一版就因为分不开，在**已经改对**
# 的 5 个用例上报了 5 个假阳性（"门禁在正确代码上失败"，比门禁不响更糟）。
# 所以正则是 `(?<!\])\["key"\]`。
#
# 负向断言（`.is_none()`）不违规：那是"这个键必须不出现"，正是规则 10
# 在服务侧钉的那件事，在测试侧钉住是合理的。

_HEALTHZ_TEST_MIN = 5        # 实测 9 个 healthz 断言用例；阈值留一半余量
_LEGACY_HEALTHZ_KEYS = ("service", "name", "version", "upstreams", "bind_addr")
_TEST_ATTR_RE = re.compile(r"#\[(?:actix_web::test|test|tokio::test)")
_IGNORE_ATTR_RE = re.compile(r"#\[ignore")
_TEST_FN_RE = re.compile(r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+([A-Za-z0-9_]+)")

_healthz_test_hits = []      # (rel, line, fn, [旧键])
_healthz_test_n = 0
_healthz_test_ignored = 0

for _dp, _dn, _fns in os.walk(os.path.join(ROOT, "crates")):
    _dn[:] = [d for d in _dn if d not in ("target", ".target-verify", ".git")]
    for _fn in _fns:
        if not _fn.endswith(".rs"):
            continue
        _full = os.path.join(_dp, _fn)
        _rel = os.path.relpath(_full, ROOT).replace("\\", "/")
        _lines = io.open(_full, encoding="utf-8", errors="replace").read().splitlines()
        _i = 0
        while _i < len(_lines):
            _m = _TEST_FN_RE.match(_lines[_i])
            if not _m:
                _i += 1
                continue
            # 上方连续的 #[...] 属性块
            _j = _i - 1
            _attrs, _ign = [], False
            while _j >= 0 and _lines[_j].strip().startswith("#["):
                _attrs.append(_lines[_j])
                if _IGNORE_ATTR_RE.match(_lines[_j].strip()):
                    _ign = True
                _j -= 1
            if not any(_TEST_ATTR_RE.search(a) for a in _attrs):
                _i += 1
                continue
            _name = _m.group(1)
            # 函数体：到下一个"带属性块的 fn"为止
            _body, _k = [], _i
            while _k < len(_lines) and _k < _i + 80:
                if _k > _i and _TEST_FN_RE.match(_lines[_k]):
                    _b = _k - 1
                    _is_attr = False
                    while _b >= 0 and _lines[_b].strip().startswith("#["):
                        _is_attr = True
                        _b -= 1
                    if _is_attr:
                        break
                _body.append(_lines[_k])
                _k += 1
            _blob = "\n".join(_body)
            if "healthz" in _blob:
                # 只把"真的对响应体键做了断言"的用例算进被检查集合。
                # 只提到 healthz（比如 make_app 里注册了路由）但没断言键的用例与
                # 本规则无关，算进去会把阈值虚高、并放大函数体窗口串到隔壁代码
                # 造成的误报面。
                _all_keys = [k for k in ("status", "app") + _LEGACY_HEALTHZ_KEYS
                             if re.search(r'(?<!\])\[\s*"%s"\s*\]' % k, _blob)
                             or re.search(r'get\(\s*"%s"\s*\)' % k, _blob)
                             or re.search(r'pointer\(\s*"/app/%s"' % k, _blob)]
                if not _all_keys:
                    _i += 1
                    continue
                _healthz_test_n += 1
                if _ign:
                    _healthz_test_ignored += 1
                _bad = []
                for _key in _LEGACY_HEALTHZ_KEYS:
                    _top = re.search(r'(?<!\])\[\s*"%s"\s*\]' % _key, _blob)
                    _get = re.search(r'get\(\s*"%s"\s*\)' % _key, _blob)
                    if not (_top or _get):
                        continue
                    # 负向断言豁免
                    if re.search(r'(?<!\])\[\s*"%s"\s*\]\s*\.is_none\(\)' % _key, _blob) \
                       or re.search(r'get\(\s*"%s"\s*\)\s*\.is_none\(\)' % _key, _blob):
                        continue
                    _bad.append(_key)
                if _bad:
                    _healthz_test_hits.append((_rel, _i + 1, _name, _bad, _ign))
            _i += 1

if _healthz_test_n < _HEALTHZ_TEST_MIN:
    err("规则 11 失效：只找到 %d 个断言 /healthz 的测试（阈值 %d）—— 扫描器大概率没在查"
        "任何东西（属性回溯或 fn 正则坏了）。**空结果不等于没有问题**"
        % (_healthz_test_n, _HEALTHZ_TEST_MIN))
else:
    for _rel, _line, _name, _bad, _ign in _healthz_test_hits:
        _ign_note = "（该用例 #[ignore]，本地跑不到）" if _ign else ""
        err("测试 %s:%d 的 `%s()` 仍断言统一前的 /healthz 形状的键 %s%s —— "
            "统一后的形状是 {\"status\",\"app\":{\"name\",\"version\"}}，"
            "请改成 body[\"app\"][\"name\"]"
            % (_rel, _line, _name, _bad, _ign_note))

note("healthz 断言用例 %d 个，其中 %d 个带 #[ignore]（本地无真 PostgreSQL 跑不到，"
     "只有 CI 的 e2e job 会执行 —— 改 healthz 形状时它们是盲区）"
     % (_healthz_test_n, _healthz_test_ignored))

print("=" * 66)
print("compose 静态不变量检查")
print("=" * 66)
print("")
if errors:
    for e in errors:
        print("  FAIL  " + e)
    print("")
    for n in notes:
        print("  提醒  " + n)
    if notes:
        print("")
    print("==> %d 项不变量被破坏" % len(errors))
    sys.exit(1)

print("  OK    规则 1  %d 个 service 全部显式声明 command" % len(MUST_DECLARE_COMMAND))
print("  OK    规则 2  宿主端口无重复")
print("  OK    规则 3  所有 DATABASE_URL 指向的库都由 db-init 创建，且相关 service 等它")
print("  OK    规则 4  声明了 *_TOPIC 的 service 都等 kafka-init")
print("  OK    规则 5  envoy 每个 filter 都有 typed_config")
print("  OK    规则 6  compose 注入的每个 env 变量都被源码读过")
print("  OK    规则 7  src/ 下没有新增的未编译 .rs 文件")
print("  OK    规则 8  每个产出二进制的 crate 都在 ci-docker-build 的 matrix 里")
print("  OK    规则 9  没有零调用的 pub fn（扫了 %d 个定义 / %d 个文件）"
      % (_n_defs, _n_files))
print("  OK    规则 10 %d 个 crate 的 %d 处 /healthz 全部返回统一形状"
      "（另跳过 %d 个非服务 crate：%s）"
      % (_healthz_n_crates, _healthz_n_sites, len(HEALTHZ_SKIP),
         ", ".join(sorted(HEALTHZ_SKIP))))
print("  OK    规则 11 %d 个断言 /healthz 的测试都不再用统一前的键"
      "（其中 %d 个 #[ignore]，本地跑不到）"
      % (_healthz_test_n, _healthz_test_ignored))
print("")
for n in notes:
    print("  提醒  " + n)
print("==> 静态检查全部通过（注意：这不替代真正 up 一次）")
