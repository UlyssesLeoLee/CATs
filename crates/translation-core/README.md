# translation-core

> CATs 翻译核心 (gRPC server + TM/术语/QA + AI 网关抽象, per `cats.v1.TranslationCoreService` proto)

## 角色

- **MatchTM**: 100% 精确 + 模糊 (pgvector 余弦) 命中
- **TranslateSegment**: TM 命中优先 → 否则 AI 网关
- **RunQA**: 术语强制 + 占位符/标签保护
- **BatchTranslate**: 串行 batch

## 数据层

- `project_db.translation_memory` (per DB 设计 v2.0 §4.3) — pgvector 0.8.6 嵌入列
- `project_db.glossary_terms` — 术语表
- **MVP 状态**: stub 模式, `match_tm` 直接 mock; Sprint 末接真实 DB

## AI 网关抽象

```rust
#[async_trait]
pub trait AiGateway: Send + Sync {
    async fn translate(&self, input: TranslateInput) -> Result<TranslateOutput, CatsError>;
}
```

- MVP: `MockAiGateway` (deterministic mock)
- Sprint 3: `HttpAiGateway` 调 `cats-ai-gateway` REST `/v1/llm/chat`

## 启动

```bash
GRPC_BIND_ADDR=0.0.0.0:50051 \
DATABASE_URL=postgres://svc_translation@postgres:5432/project_db \
cargo run -p translation-core
```

## 测试

```bash
cargo test -p translation-core
# 5 个单元测试 (mock + qa + placeholder)
```

## 已知缺口

- ❌ 项目 DB 真实连 (MVP 阶段 mock)
- ❌ HttpAiGateway (接 cats-ai-gateway) — Sprint 3
- ❌ 真 AI provider 接 OpenAI/Anthropic 等 — Sprint 3

## 引用

- 接口设计 v2.0+2: `doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md` §6.5
- 数据库 v2.0: `doc/03-详细设计/数据库设计/CATs_数据库设计书_v2.0.md` §4.3
- proto: `proto/cats/v1/translation_core.proto`
- ADR-010 LLM 推理策略: `doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md`