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
# 为什么难发现: 文件在、代码完整、注释和文档都像那么回事，
# 但 binary 启动后只注册 /healthz。487 个测试和 12 路 healthz 全绿，
# 都不能证明这些代码存在——因为它们根本没进编译。
#
# 实证方式: 往 error.rs 里注入一行语法错误后 `cargo check -p cats-common`
# 仍然 15.55s 成功退出 0。若它真被编译，rustc 必然报错。
#
# 已知存量列入基线（gate new violations），清债方式见 v0.4 §未接项。
ORPHAN_BASELINE = {
    "audit-service": ["db.rs", "handlers.rs", "models.rs", "state.rs"],
    "cats-bff": ["grpc_clients.rs", "routes.rs", "upstream/auth.rs",
                 "upstream/projects.rs", "upstream/tasks.rs",
                 "upstream_passthrough.rs"],
    "cats-mock": ["data/audit.rs", "data/project.rs", "data/task.rs",
                  "data/user.rs", "db/fixture.rs", "db/schema.rs", "db/seed.rs",
                  "http/response.rs", "http/routes.rs", "http/server.rs",
                  "infra/kafka.rs", "infra/redis.rs"],
    "file-service": ["state.rs", "storage.rs"],
    "notification-service": ["consumer.rs", "state.rs"],
    "project-service": ["state.rs"],
    "report-service": ["state.rs"],
    "task-service": ["state.rs"],
    # 已于 2026-10-04 接线、因此从基线移除的有 4 个 crate:
    #   common            error.rs（550 行共享错误体系）
    #   cats-rbac         service_helpers.rs（178 行）
    #   translation-core  6 个模块（550 行，含 4 个 RPC 的 gRPC 实现）
    #   worker-service    3 个模块（170 行，含抢占→派发→回写调度器）
    # 合计约 1450 行从"写在磁盘上"变成"真的在编译"。
    # 它们若再次掉出编译，规则 7 会重新 FAIL —— 这正是基线的用法。
}

MOD_RE = re.compile(r'^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*;', re.M)


def reachable_rs(src_dir):
    """从 lib.rs / main.rs 出发做传递闭包，返回真正会编译的相对路径集合。"""
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
            for cand in (m + ".rs", m + "/mod.rs"):
                cp = os.path.join(src_dir, *cand.split("/"))
                if os.path.isfile(cp):
                    frontier.append(cp)
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
print("")
for n in notes:
    print("  提醒  " + n)
print("==> 静态检查全部通过（注意：这不替代真正 up 一次）")
