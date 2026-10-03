//! `translation-core` 入口
//!
//! M0 阶段：actix-web 4 健康检查占位，端口与绑定地址走环境变量。
//! 业务 endpoint 在 M1 阶段按微服务架构书 §4.1 + OpenAPI v1 落地。

use actix_web::{web, App, HttpResponse, HttpServer};
use cats_common::AppMeta;
use serde::Serialize;
use std::env;
use tracing::info;

/// 架构书 §4.1 给 translation-core 的端口
const DEFAULT_BIND: &str = "0.0.0.0:50051";

/// 业务配置（M0 占位：从 env 读取；M1 替换为结构化 config）
#[derive(Debug, Clone)]
struct Config {
    bind_addr: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            bind_addr: resolve_bind_addr(
                env::var("GRPC_BIND_ADDR").ok(),
                env::var("BIND_ADDR").ok(),
            ),
        }
    }
}

/// 绑定地址的取值优先级：`GRPC_BIND_ADDR` > `BIND_ADDR` > 架构书默认端口。
///
/// 拆成纯函数是为了能测：直接改 process env 在 `cargo test` 的并行线程里
/// 是全局竞态，测出来的顺序不可信。
///
/// 真实事故（2026-10-04）：本文件原先只读 `BIND_ADDR`（默认 0.0.0.0:8090），
/// 而 compose 与架构书给的都是 `GRPC_BIND_ADDR=0.0.0.0:50051`——于是容器里
/// 实际监听 8090，宿主 50051 映射到的端口上什么都没有，而 `worker-service`
/// 和 `cats-bff` 都把 `TRANSLATION_CORE_URL` 指向 `translation-core:50051`。
/// 变量名对不上属于拼写级错误，后果却是整条跨服务链路静默不通。
fn resolve_bind_addr(grpc: Option<String>, legacy: Option<String>) -> String {
    grpc
        .or(legacy)
        .unwrap_or_else(|| DEFAULT_BIND.to_string())
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
        app: AppMeta::current(),
    })
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    cats_common::init_tracing();
    let cfg = Config::from_env();
    info!(bind_addr = %cfg.bind_addr, "starting translation-core");

    HttpServer::new(|| App::new().route("/healthz", web::get().to(healthz)))
        .bind(&cfg.bind_addr)?
        .run()
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &str) -> Option<String> {
        Some(v.to_string())
    }

    #[test]
    fn grpc_bind_addr_beats_legacy() {
        assert_eq!(
            resolve_bind_addr(s("0.0.0.0:50051"), s("0.0.0.0:8090")),
            "0.0.0.0:50051"
        );
    }

    #[test]
    fn legacy_bind_addr_still_accepted() {
        assert_eq!(resolve_bind_addr(None, s("0.0.0.0:9999")), "0.0.0.0:9999");
    }

    #[test]
    fn falls_back_to_architectured_port() {
        // 两条事故的共同根因：这个默认值以前是 8090（= ai-gateway 的端口）
        assert_eq!(resolve_bind_addr(None, None), "0.0.0.0:50051");
    }
}
