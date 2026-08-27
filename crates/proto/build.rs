//! `cats-proto` build script
//!
//! 调用 `tonic-build` 编译 `proto/cats/v1/*.proto` 为 Rust 类型 + gRPC client/server trait。
//!
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §5.3（gRPC）+ §8.2（Protobuf）
//! 引用: proto/cats/v1/{auth,common,media,translation_core}.proto
//!
//! 编译产物位置：OUT_DIR 环境变量指向的目录（`tonic-build` 自动管理）
//! 编译产物会在 `cargo build` 时通过 `cats_proto::cats::v1::*` 暴露。

use std::io::Result;

fn main() -> Result<()> {
    // 关闭 `tonic-build` 的 rerun-if-changed 默认行为（避免每次 cargo build 都重编）
    println!("cargo:rerun-if-changed=../../proto");
    println!("cargo:rerun-if-changed=build.rs");

    // tonic-build 0.13: tonic_prost_build::configure()
    // include path = workspace root (../..)，从而解析 `import "proto/cats/v1/common.proto"`
    tonic_build::configure()
        // 启用 serde 派生（让生成的消息结构直接走 JSON 序列化）
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        // 抑制 tonic-build 生成代码的 missing_docs 警告
        // （生成的 proto 类型不带 doc 注释，开启 workspace 严格 lint 时会刷出 100+ 警告；
        //  module 级别 allow 在 cats-proto/src/lib.rs 的 `pub mod v1` 上加）
        .type_attribute(".", "#[allow(missing_docs, non_camel_case_types)]")
        .field_attribute(".", "#[allow(missing_docs)]")
        // 编译期检查：proto 字段缺失时用 default
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "proto/cats/v1/common.proto",
                "proto/cats/v1/auth.proto",
                "proto/cats/v1/media.proto",
                "proto/cats/v1/translation_core.proto",
            ],
            &["../.."],
        )?;

    Ok(())
}
