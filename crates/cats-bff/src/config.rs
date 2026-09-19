//! `cats-bff` 运行时配置
//!
//! 全部从环境变量读取;不在此处打印任何 secret 值（per 8/27 11:06 JST 守门 #5）。
//!
//! 服务基址约定:
//! - `auth-service`     K8s Service DNS: `http://auth-service.cats-core.svc.cluster.local:8080`
//! - `project-service`  K8s Service DNS: `http://project-service.cats-core.svc.cluster.local:8083`
//! - `translation-core` K8s Service DNS: `http://translation-core.cats-core.svc.cluster.local:8090`
//!
//! 测试场景下用 `127.0.0.1:<port>` 覆盖。

use std::env;

/// BFF 运行时配置
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP 监听地址（默认 `0.0.0.0:8097`，env `BIND_ADDR`）
    pub bind_addr: String,
    /// auth-service 基址（不含路径前缀），默认 K8s Service DNS
    pub auth_service_base: String,
    /// project-service 基址，默认 K8s Service DNS
    pub project_service_base: String,
    /// translation-core gRPC URL（含 scheme），默认 `http://translation-core...:8090`
    pub translation_core_grpc: String,
    /// 上游调用超时（毫秒），默认 `5000`
    pub upstream_timeout_ms: u64,
}

impl Config {
    /// 从环境变量构建配置；缺失项走默认值
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8097".to_string()),
            auth_service_base: env::var("AUTH_SERVICE_BASE").unwrap_or_else(|_| {
                "http://auth-service.cats-core.svc.cluster.local:8080".to_string()
            }),
            project_service_base: env::var("PROJECT_SERVICE_BASE").unwrap_or_else(|_| {
                "http://project-service.cats-core.svc.cluster.local:8083".to_string()
            }),
            translation_core_grpc: env::var("TRANSLATION_CORE_GRPC").unwrap_or_else(|_| {
                "http://translation-core.cats-core.svc.cluster.local:8090".to_string()
            }),
            upstream_timeout_ms: env::var("UPSTREAM_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5000),
        }
    }

    /// 测试构造函数：直接传入 base/grpc URL
    pub fn for_test(
        auth_base: String,
        project_base: String,
        translation_grpc: String,
    ) -> Self {
        Self {
            bind_addr: "127.0.0.1:0".to_string(),
            auth_service_base: auth_base,
            project_service_base: project_base,
            translation_core_grpc: translation_grpc,
            upstream_timeout_ms: 2000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_has_sane_defaults() {
        let c = Config::for_test(
            "http://127.0.0.1:9001".into(),
            "http://127.0.0.1:9003".into(),
            "http://127.0.0.1:9090".into(),
        );
        assert_eq!(c.auth_service_base, "http://127.0.0.1:9001");
        assert_eq!(c.project_service_base, "http://127.0.0.1:9003");
        assert_eq!(c.translation_core_grpc, "http://127.0.0.1:9090");
        assert_eq!(c.upstream_timeout_ms, 2000);
    }
}