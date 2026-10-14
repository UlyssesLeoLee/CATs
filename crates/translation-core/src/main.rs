//! `translation-core` 入口
//!
//! 两个监听端口（per 微服务架构设计书 §4.1 + 技术基线 §1）：
//!
//! - `GRPC_BIND_ADDR`（默认 `0.0.0.0:50051`）：tonic gRPC server，
//!   承载 `TranslationCoreServiceImpl` 的 4 个 RPC。
//! - `BIND_ADDR`（默认 `0.0.0.0:8080`）：HTTP `/healthz`，
//!   与本仓其余 11 个 service 的约定一致，让探活/监控可以统一走 HTTP。
//!
//! 两个端口用**各自的 runtime**：gRPC 跑在 tokio 上，healthz 跑在
//! 独立线程的 actix `System` 上。把 actix 的 HttpServer 和 tonic 塞进
//! 同一个 runtime 是常见的踩坑点（actix-rt 的 LocalSet 与 tonic 的
//! spawn 要求会打架），隔离开最省事也最稳。
//!
//! 2026-10-04：本文件此前是只注册 `/healthz` 的 M0 占位，而
//! `service.rs` / `qa.rs` / `db.rs` / `ai_gateway.rs` 等 550 行
//! 因为 `lib.rs` 没有 `mod` 声明**从未被编译**——于是服务对外只表现为
//! 一个 healthz 探针，4 个 RPC 一个都没实现。

use std::net::SocketAddr;

use actix_web::{web, App, HttpResponse, HttpServer};
use cats_common::AppMeta;
use serde::Serialize;
use std::env;
use tonic::transport::Server;
use tracing::{error, info};

use cats_proto::cats::v1::translation_core_service_server::TranslationCoreServiceServer;
use translation_core::service::TranslationCoreServiceImpl;

/// 架构书 §4.1 给 translation-core 的 gRPC 端口
const DEFAULT_GRPC_BIND: &str = "0.0.0.0:50051";
/// HTTP `/healthz` 端口，与其余 service 的 `BIND_ADDR` 同义
const DEFAULT_HTTP_BIND: &str = "0.0.0.0:8080";

/// 业务配置
#[derive(Debug, Clone)]
struct Config {
    grpc_bind_addr: String,
    http_bind_addr: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            grpc_bind_addr: env::var("GRPC_BIND_ADDR")
                .unwrap_or_else(|_| DEFAULT_GRPC_BIND.to_string()),
            http_bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| DEFAULT_HTTP_BIND.to_string()),
        }
    }
}

/// 健康检查响应
#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    app: AppMeta,
}

/// `GET /healthz` — 存活探针 + 启动探针复用
async fn healthz() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        // 2026-10-05: 原来用 AppMeta::current(), 它返回的是 **cats-common 自己的**
        // CARGO_PKG_NAME —— 于是所有 service 的 /healthz 都自报 "cats-common",
        // 监控无法区分是哪个服务应答的, 版本号也是共享库的版本。
        // env! 是编译期按**本 crate** 展开的, 所以这里报的是本服务自己。
        app: AppMeta {
            name: env!("CARGO_PKG_NAME").to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        },
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    cats_common::init_tracing();
    let cfg = Config::from_env();

    // HTTP /healthz 独立线程 + 独立 actix runtime
    {
        let http_bind_addr = cfg.http_bind_addr.clone();
        std::thread::Builder::new()
            .name("translation-core-healthz".into())
            .spawn(move || {
                actix_web::rt::System::new().block_on(async move {
                    let srv =
                        HttpServer::new(|| App::new().route("/healthz", web::get().to(healthz)))
                            .bind(&http_bind_addr);
                    match srv {
                        Ok(s) => {
                            info!(addr = %http_bind_addr, "healthz listening");
                            if let Err(e) = s.run().await {
                                error!(error = %e, "healthz server stopped");
                            }
                        }
                        Err(e) => error!(addr = %http_bind_addr, error = %e, "healthz bind failed"),
                    }
                });
            })
            .map_err(|e| std::io::Error::other(e.to_string()))?;
    }

    let grpc_addr: SocketAddr = cfg.grpc_bind_addr.parse()?;
    info!(
        grpc = %grpc_addr,
        healthz = %cfg.http_bind_addr,
        "starting translation-core"
    );

    Server::builder()
        .add_service(TranslationCoreServiceServer::new(
            TranslationCoreServiceImpl::default(),
        ))
        .serve(grpc_addr)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_architectured_ports() {
        // gRPC 50051 是架构书 §4.1 的值；8090（曾经在这里）会让本服务
        // 与 ai-gateway 撞语义，是 2026-10-04 那个 P0 的同一个坑。
        assert_eq!(Config::from_env().grpc_bind_addr, DEFAULT_GRPC_BIND);
    }

    #[test]
    fn grpc_port_parses_as_socket_addr() {
        let a: SocketAddr = DEFAULT_GRPC_BIND.parse().unwrap();
        assert_eq!(a.port(), 50051);
    }
}
