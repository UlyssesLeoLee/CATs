# cats-proto

> CATs gRPC 协议契约 crate — 由 `tonic-build` 编译期生成

| 项目 | 内容 |
|---|---|
| Crate 名 | `cats-proto` |
| 生成方式 | `tonic-build` 在 `build.rs` 中调用 `compile_protos` |
| proto 源 | [`proto/cats/v1/`](../../proto/cats/v1/) 4 份：auth / common / media / translation_core |
| 阶段 | M0 脚手架 → M1 接入 |

## 概述

`cats-proto` 是 CATs 的 **跨服务 gRPC 契约单一来源**。所有 16 个服务 + BFF 通过本 crate 引用协议消息与服务 trait，避免：

- proto 字段重复声明（DRY）
- 字段类型在 Rust 侧与 proto 侧漂移
- 编译器不抓到的 wire format 不一致

## 当前已编译 proto

| proto 文件 | 命名空间 | 关键 Service / 消息 |
|---|---|---|
| `common.proto` | `cats.v1` | `Pagination`, `PageMeta`, `LanguageCode`, `TaskStatus` |
| `auth.proto` | `cats.v1` | `AuthService`（Login/RefreshToken/ValidateToken） |
| `media.proto` | `cats.v1` | `MediaProcessingService`（TranscribeAudio/ExtractOCR/BurnSubtitles） |
| `translation_core.proto` | `cats.v1` | `TranslationCoreService`（MatchTM/TranslateSegment/RunQA/BatchTranslate） |

## 用法

```rust
use cats_proto::cats::v1::auth_service_client::AuthServiceClient;
use cats_proto::cats::v1::LoginRequest;

let mut client = AuthServiceClient::connect("http://auth-service:8080").await?;
let req = LoginRequest {
    username: "alice".into(),
    password: "secret".into(),
    tenant_id: "t1".into(),
};
let resp = client.login(req).await?.into_inner();
```

## 数据边界

- **无独立数据库**
- **无独立网络端点**
- **无业务逻辑**，仅作为契约载体

## 上下游服务

- **上游（被依赖）**：所有需要调用 gRPC 的 service（auth / project / task / translation-core / media 子服务）
- **下游（依赖）**：`tonic` + `prost`（编译期）+ `tonic-build`（build 脚本）

## 重新生成 proto

```powershell
# cargo build 会自动触发 build.rs
cd crates/proto
cargo build

# 或仓库级脚本
pwsh ci/scripts/proto-gen.ps1
```

修改 `proto/cats/v1/*.proto` 后必须：

1. 重新 `cargo build` 触发 tonic-build
2. 检查生成的类型签名是否破坏 service 实现
3. 提交 `Cargo.lock`（workspace 已锁定 transitive deps）

## 引用基线文档

- [CATs_技术选型书_v2.0 §5.3](../../doc/02-基础设计/技术选型/CATs_技术选型书_v2.0.md)（gRPC 选型）
- [CATs_Rust技术选型书_v1.0 §5.3/§8.2](../../doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md)
- [proto/cats/v1/](../../proto/cats/v1/)（proto 源）

## 注意

- M0 阶段：service 实现尚未接入 grpc server（仅占位 main.rs），gRPC 调用在 M1 阶段落地
- 编译期产物在 `OUT_DIR`，不入仓；`Cargo.lock` 入仓
- tonic-build 0.13 配 prost 0.13，版本必须严格匹配
