//! `translation-core` 入口
//!
//! 启动 tonic gRPC server, 监听 GRPC_BIND_ADDR (默认 0.0.0.0:50051)

use cats_common::init_tracing;
use cats_proto::cats::v1::translation_core_service_server::TranslationCoreServiceServer;
use std::env;
use tonic::transport::Server;
use tracing::info;
use translation_core::TranslationCoreServiceImpl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();

    let grpc_addr = env::var("GRPC_BIND_ADDR")
        .unwrap_or_else(|_| "0.0.0.0:50051".to_string())
        .parse()?;
    let service = TranslationCoreServiceImpl::default();

    info!(grpc_addr = %grpc_addr, "starting translation-core gRPC server");

    Server::builder()
        .add_service(TranslationCoreServiceServer::new(service))
        .serve(grpc_addr)
        .await?;

    Ok(())
}