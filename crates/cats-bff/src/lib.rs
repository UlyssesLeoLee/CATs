//! `cats-bff` — BFF 聚合服务
//!
//! 引用: doc/02-基础设计/架构设计/CATs_微服务架构设计书_v1.0.md §4.1
//! 引用: doc/03-详细设计/接口设计/CATs_接口设计书_v2.0.md §3.5 错误响应 + §1.1 基址
//!
//! M1 阶段：actix-web 4 + reqwest 透传 REST + tonic gRPC 调 translation-core。

pub mod config;
pub mod error;
pub mod grpc_clients;
pub mod routes;
pub mod upstream;

use actix_web::{web, App};

pub use config::Config;
pub use error::{BffError, ErrorBody};
pub use grpc_clients::{TmCommitAck, TmLookupResponse, TmMatchItemJson, TranslationClient};
pub use upstream::UpstreamClient;

/// BFF 共享应用状态（注入到 actix `web::Data`）
#[derive(Clone)]
pub struct AppState {
    /// 上游透传客户端（REST）
    pub upstream: UpstreamClient,
    /// translation-core gRPC 客户端
    pub grpc: TranslationClient,
}

impl AppState {
    /// 从 [`Config`] 构造应用状态
    pub async fn from_config(cfg: &Config) -> Result<Self, BffError> {
        let upstream = UpstreamClient::new(cfg)?;
        let grpc = TranslationClient::connect(cfg).await?;
        Ok(Self { upstream, grpc })
    }

    /// 测试用：直接传入现成 client
    pub fn for_test(upstream: UpstreamClient, grpc: TranslationClient) -> Self {
        Self { upstream, grpc }
    }
}

/// 构造 actix `App` 工厂（main.rs + 集成测试都用）
pub fn build_app(
    state: web::Data<AppState>,
) -> App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    App::new()
        .app_data(state)
        .configure(routes::configure)
}

/// 当前 crate 语义版本
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        let v = version();
        assert!(
            v.starts_with("0.1."),
            "version should start with '0.1.', got {v}"
        );
    }

    #[test]
    fn name_is_crate_name() {
        assert_eq!(name(), "cats-bff");
    }
}