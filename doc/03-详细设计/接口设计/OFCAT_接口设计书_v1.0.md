# OFCAT 接口设计书

**系统名称:** OFCAT — AI 增强型 CAT 浏览器工作台

---

## 文档管理信息

| 项目 | 内容 |
|---|---|
| 文档编号 | OFCAT-DD-I-001 |
| 文档名 | 接口设计书（详细设计 / API 契约） |
| 版本 | 第 1.0 版（草稿） |
| 创建日 | 2026-06-25 |
| 作者 | 架构师 |
| 状态 | 评审前草稿 |
| 密级 | 仅社内 |
| 上游文档 | [基础设计书 v1.0](../../02-基础设计/架构设计/OFCAT_基础设计书_v1.0.md) |

### 修订履历

| 版本 | 日期 | 修订者 | 修订内容 |
|---|---|---|---|
| 1.0 | 2026-06-25 | 架构师 | 初版。扩展↔引擎 API-01~10 完整契约、SSE 事件、错误码 |

---

## 1. 通用约定

### 1.1 基础
- 基址：`http://127.0.0.1:<port>`（默认端口在握手时确定/可配置）。
- 编码：`application/json; charset=utf-8`；时间为 ISO-8601 UTC 字符串。
- 协议版本：请求头 `X-OFCAT-API: 1`。

### 1.2 认证与安全
| 项 | 约定 |
|---|---|
| 鉴权 | `Authorization: Bearer <token>`；令牌由引擎启动时随机生成，经扩展安装态获取（API-02） |
| Origin 校验 | 校验 `Origin`/扩展 ID 在白名单内，否则 `403 FORBIDDEN_ORIGIN` |
| Host 校验 | 仅接受 `127.0.0.1`/`localhost`，防 DNS rebinding |
| 速率 | 单客户端令牌限流（默认 20 req/s，可配置） |

### 1.3 错误信封
```json
{ "error": { "code": "COMPLIANCE_BLOCKED", "message": "…", "details": { } } }
```

### 1.4 错误码表

| code | HTTP | 含义 | 关联 |
|---|---|---|---|
| `VALIDATION_ERROR` | 400 | 请求体校验失败 | 全部 |
| `UNAUTHORIZED` | 401 | 令牌缺失/失效 | E-02 |
| `FORBIDDEN_ORIGIN` | 403 | Origin/扩展 ID 不在白名单 | 安全 |
| `NOT_FOUND` | 404 | 资源不存在（如 job_id） | API-09 |
| `COMPLIANCE_BLOCKED` | 409 | 敏感内容且本地模型不可达，fail-closed | E-04 |
| `QA_BLOCKED` | 422 | QA 阻断项存在，禁止写回 | E-05 |
| `RATE_LIMITED` | 429 | 超出限流 | — |
| `MODEL_ERROR` | 502 | 模型调用失败（重试/回退后仍失败） | E-03 |
| `ENGINE_ERROR` | 500 | 引擎内部错误 | — |
| `MODEL_TIMEOUT` | 504 | 模型超时 | E-03 |

---

## 2. API 详细契约

### API-01 `GET /health`
- 用途：健康检查、版本与能力探测。无需鉴权。
- 响应 200：
```json
{ "status": "ok", "version": "1.0.0",
  "capabilities": { "ocr": false, "local_model": true, "vec": true },
  "local_model_healthy": true }
```

### API-02 `POST /session/handshake`
- 用途：令牌校验、协议/能力协商。
- 请求：`{ "client": "ext", "ext_id": "<extension id>", "api": 1 }`
- 响应 200：`{ "session_id": "…", "expires_at": "…", "limits": { "rps": 20 } }`
- 错误：`401 UNAUTHORIZED` / `403 FORBIDDEN_ORIGIN`。

### API-03 `POST /tm/match`
- 用途：仅做 TM 匹配（不翻译），用于预填/旁路查询。
- 请求：

| 字段 | 类型 | 必填 | 约束 |
|---|---|---|---|
| `segment` | string | 是 | 1..5000 |
| `source_lang` / `target_lang` | string | 是 | BCP-47，如 `ja`/`zh-CN` |
| `domain` | string | 否 | 默认 `""` |
| `project_id` | int | 否 | — |

- 响应 200：
```json
{ "level": "L1", "best": { "score": 96, "target": "…", "diff": [...] },
  "candidates": [ { "score": 96, "target": "…", "source": "…", "origin": "human" } ] }
```
`level ∈ {L0,L1,MISS}`；`MISS` 时 `best=null`。

### API-04 `POST /translate`（SSE）
- 用途：完整翻译管道（TM→术语→保护→翻译→校验→QA），**流式**返回。
- 请求：

| 字段 | 类型 | 必填 | 约束 |
|---|---|---|---|
| `segment` | string | 是 | 1..5000，含标签/占位符原样 |
| `source_lang` / `target_lang` | string | 是 | BCP-47 |
| `domain` | string | 否 | — |
| `project_id` | int | 否 | — |
| `url` | string | 否 | 用于合规判定 |
| `mode` | string | 否 | `default`(L2) / `high_quality`(L3)，默认 `default` |

- 响应：`Content-Type: text/event-stream`，事件序列：

| event | data | 说明 |
|---|---|---|
| `tm_hit` | `{level,score,target,diff}` | 命中 L0/L1 时下发（随后直接 `done`） |
| `terms` | `{matched:[{source_term,target_term}]}` | 命中术语 |
| `delta` | `{text}` | 流式译文增量（哨兵态/已回填） |
| `qa` | `{pass,blocks,warns}` | QA 结果 |
| `done` | `{target,level,terms,qa,elapsed_ms}` | 终态最终译文与元数据 |
| `error` | `{code,message}` | 异常终止（对应错误码表） |

- 错误：`409 COMPLIANCE_BLOCKED`（在流前以 HTTP 状态返回；流中异常则发 `error` 事件后结束）。

### API-05 `POST /tm/commit`
- 用途：确认译文回存 TM（F9）。
- 请求：

| 字段 | 类型 | 必填 |
|---|---|---|
| `segment` / `target` | string | 是 |
| `source_lang` / `target_lang` | string | 是 |
| `domain` / `context` | string | 否 |
| `project_id` | int | 否 |
| `origin` | string | 否（默认 `human`） |

- 响应 200：`{ "id": 123, "action": "created" | "updated" }`

### API-06 `POST /qa/check`
- 用途：对给定原文/译文独立做 QA（标签/术语/数量）。
- 请求：`{ "segment", "translation", "source_lang", "target_lang", "domain", "project_id" }`
- 响应 200：`{ "pass": false, "blocks": [{rule:"QA-01",msg:"…"}], "warns": [...] }`

### API-07 `GET /settings` · `PUT /settings`
- 用途：读写设置（模型/合规策略/语言对/路径）。
- `GET` 响应 / `PUT` 请求体（节选）：
```json
{ "models": { "cloud": {"endpoint","model","key_ref"}, "local": {"endpoint","model"} },
  "compliance": { "sensitive_domains": ["jira.example.com"], "default": "cloud_ok" },
  "lang_pairs": [["ja","zh-CN"],["en","zh-CN"]],
  "paths": { "db": "…/ofcat.db" } }
```
- 注：云端 `key` 不回显，仅以 `key_ref` 引用引擎侧密钥库。

### API-08 `POST /import`（异步任务）
- 用途：提交存量导入（F11）。`multipart/form-data`：文件 + `mapping`(JSON)。
- `mapping` 示例：`{ "source_col":"A", "target_col":"B", "src_lang":"ja", "tgt_lang":"zh-CN", "domain_col":"C", "dedup":"by_hash" }`
- 响应 202：`{ "job_id": "uuid", "status": "pending" }`

### API-09 `GET /import/{job_id}`
- 响应 200：
```json
{ "job_id":"…","status":"done","total":1200,"succeeded":1100,"duplicated":80,"failed":20,
  "report":[{ "row":34,"error":"missing target" }] }
```
- 错误：`404 NOT_FOUND`。

### API-10 `POST /sync/pull` · `POST /sync/push`
- 用途：经引擎代理与中心同步服务交换术语/核心 TM（C4，异步）。
- 请求：`{ "entity":"terms"|"tm", "cursor":"…" }`
- 响应：`{ "changes":[...], "next_cursor":"…", "conflicts":[{id, strategy}] }`
- 冲突策略：术语库以中心为准；TM 采用「后写优先 + 保留双方」并标记待人工（详见数据库设计 §合并）。

---

## 3. 时序（/translate 未命中分支）
```
扩展 ──POST /translate(SSE)──► 引擎
  ◄─ event: terms
  ◄─ event: delta (多次)
  ◄─ event: qa
  ◄─ event: done
（用户确认后）
扩展 ──POST /tm/commit──► 引擎 ─► 200 {id, action}
```

---

## 4. 兼容与版本策略
- 以 `X-OFCAT-API` 头标识协议大版本；不兼容变更升大版本并保留过渡期双支持。
- 新增可选字段不升大版本；扩展须忽略未知字段（前向兼容）。
