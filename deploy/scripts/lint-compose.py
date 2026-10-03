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
COMPOSE = os.path.join(ROOT, "deploy", "docker-compose-mvp.yml")
ENVOY = os.path.join(ROOT, "deploy", "envoy-mvp.yaml")

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

# Kafka consumer 必须等 kafka-init（topic 是显式建的）
# 真实事故: KAFKA_AUTO_CREATE_TOPICS_ENABLE=false，而两个 consumer 各自
# 等一个 topic。不建的话连得上 broker 然后永远收不到消息。
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

print("=" * 66)
print("compose 静态不变量检查")
print("=" * 66)
for n in notes:
    print("  note  " + n)
print("")
if errors:
    for e in errors:
        print("  FAIL  " + e)
    print("")
    print("==> %d 项不变量被破坏" % len(errors))
    sys.exit(1)

print("  OK    %d 个 service 全部显式声明 command" % len(MUST_DECLARE_COMMAND))
print("  OK    宿主端口无重复")
print("  OK    所有 DATABASE_URL 指向的库都由 db-init 创建，且相关 service 等它")
print("  OK    Kafka consumer 都等 kafka-init")
print("  OK    envoy 每个 filter 都有 typed_config")
print("")
print("==> 静态检查全部通过（注意：这不替代真正 up 一次）")
