//! cats-bff 多上游服务配置 (per 切片 A _slice_a_bff.md)
//!
//! BFF 聚合层不直连 PG，仅透传到下游 service
//! (auth-service / user-service / project-service / task-service / translation-core)
//!
//! env 变量读取 (per 守门 #11 缺标比错标: 缺标用默认值, 不打 fail-fast):
//!   SERVICE_BIND_ADDR   默认 `0.0.0.0:8097`
//!   AUTH_SERVICE_URL    默认 `http://localhost:8081`
//!   USER_SERVICE_URL    默认 `http://localhost:8082`
//!   PROJECT_SERVICE_URL 默认 `http://localhost:8083`
//!   TASK_SERVICE_URL    默认 `http://localhost:8084`
//!   TRANSLATION_CORE_URL 默认 `http://localhost:8086`
//!   UPSTREAM_TIMEOUT_SECS 默认 `5`

use std::env;

/// BFF 业务配置
#[derive(Debug, Clone)]
pub struct Config {
    /// 监听地址 (per BFF 端口约定 8097)
    pub bind_addr: String,
    /// auth-service 上游 URL (REST: /v1/auth/*)
    pub auth_service_url: String,
    /// user-service 上游 URL (REST: /v1/users/*)
    pub user_service_url: String,
    /// project-service 上游 URL (REST: /v1/projects)
    pub project_service_url: String,
    /// task-service 上游 URL (REST: /v1/tasks)
    pub task_service_url: String,
    /// translation-core 上游 URL (REST: /v1/translations/*)
    pub translation_core_url: String,
    /// 上游调用 timeout (秒)
    pub upstream_timeout_secs: u64,
}

impl Config {
    /// 从环境变量读 Config
    pub fn from_env() -> Self {
        Self {
            bind_addr: env::var("SERVICE_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8097".to_string()),
            auth_service_url: env::var("AUTH_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8081".to_string()),
            user_service_url: env::var("USER_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8082".to_string()),
            project_service_url: env::var("PROJECT_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8083".to_string()),
            task_service_url: env::var("TASK_SERVICE_URL")
                .unwrap_or_else(|_| "http://localhost:8084".to_string()),
            translation_core_url: env::var("TRANSLATION_CORE_URL")
                .unwrap_or_else(|_| "http://localhost:8086".to_string()),
            upstream_timeout_secs: env::var("UPSTREAM_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_documented_ports() {
        let c = Config::from_env();
        assert_eq!(c.bind_addr, "0.0.0.0:8097");
        assert!(c.auth_service_url.starts_with("http://"));
        assert_eq!(c.upstream_timeout_secs, 5);
    }
}
