//! `cats-ai-gateway` build script
//!
//! 编译 `proto/cats/llm/v1/llm_gateway.proto` 为 Rust 类型 + gRPC server/client trait。
//!
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §5.3
//! 引用: doc/02-基础设计/决策/CATs_ADR-010_LLM部署与推理策略_v1.0.md §4
//!
//! 故意独立于 `crates/proto` 编译（per 范围:不污染 cats-proto schema; 待 proto 治理
//! 流程统一后迁回 `proto/cats/v1/`）。

use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=build.rs");

    tonic_build::configure()
        // JSON 序列化（让 REST 出口能直接 serde_json::to_string）
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .type_attribute(".", "#[allow(missing_docs, non_camel_case_types)]")
        .field_attribute(".", "#[allow(missing_docs)]")
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &["./proto/cats/llm/v1/llm_gateway.proto"],
            &["./proto"],
        )?;

    Ok(())
}