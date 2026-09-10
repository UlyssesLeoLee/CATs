//! MockServer: 启动 actix-web test server, 暴露 base_url
//!
//! 引用: 设计书 §4.4.1
//!
//! 用 actix_web::test::start() 启动, 监听 0.0.0.0:0 (随机端口)
//!
//! 注意: 当前实现是 actix-web test server (in-process, 适合单测),
//!       不是 OS-level 端口绑定; 如果需要跨进程 e2e, 用 standalone mode
//!       (把 cfg 转 actix_web::HttpServer 跑在 tokio runtime)

use actix_web::{App, HttpServer};
use std::net::TcpListener;

/// MockServer 配置
#[derive(Debug, Clone, Default)]
pub struct MockServerConfig {
    /// 包含的路由组
    pub include_auth: bool,
    pub include_user: bool,
    pub include_project: bool,
    pub include_task: bool,
    pub include_audit: bool,
    pub include_healthz: bool,
}

impl MockServerConfig {
    /// 全部路由
    pub fn all() -> Self {
        Self {
            include_auth: true,
            include_user: true,
            include_project: true,
            include_task: true,
            include_audit: true,
            include_healthz: true,
        }
    }

    /// 仅 auth + user
    pub fn auth_user_only() -> Self {
        Self {
            include_auth: true,
            include_user: true,
            include_healthz: true,
            ..Default::default()
        }
    }
}

/// MockServer handle (test server 包装)
pub struct MockServer {
    /// actix test server address (e.g. "127.0.0.1:54321")
    addr: String,
}

impl MockServer {
    /// 启动一个 test server, 按 config 注册路由
    pub async fn start(config: MockServerConfig) -> std::io::Result<Self> {
        // 用 TcpListener 选一个空闲端口
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?.to_string();
        drop(listener); // 释放, 让 actix_web 重新 bind 同一端口

        let server = HttpServer::new(move || {
            App::new()
                .configure(|cfg| {
                    if config.include_auth { super::routes::auth_routes(cfg); }
                    if config.include_user { super::routes::user_routes(cfg); }
                    if config.include_project { super::routes::project_routes(cfg); }
                    if config.include_task { super::routes::task_routes(cfg); }
                    if config.include_audit { super::routes::audit_routes(cfg); }
                    if config.include_healthz { super::routes::healthz_routes(cfg); }
                })
        })
        .bind(addr.clone())?
        .run();

        // 后台运行, 不阻塞
        let handle = server.handle();
        tokio::spawn(server);

        // 给 actix 一小段时间启动
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // 防止 handle 被 drop 警告
        let _ = handle;

        Ok(Self { addr })
    }

    /// base URL (e.g. "http://127.0.0.1:54321")
    pub fn base_url(&self) -> String { format!("http://{}", self.addr) }

    /// addr
    pub fn addr(&self) -> &str { &self.addr }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest;

    #[tokio::test]
    async fn start_healthz_only() {
        let server = MockServer::start(MockServerConfig {
            include_healthz: true,
            ..Default::default()
        })
        .await
        .expect("start");
        let base = server.base_url();

        // 用 reqwest 打 /healthz
        let client = reqwest::Client::new();
        let resp = client
            .get(format!("{base}/healthz"))
            .send()
            .await
            .expect("request");
        assert_eq!(resp.status().as_u16(), 200);
    }

    #[tokio::test]
    async fn start_all_routes_login_works() {
        let server = MockServer::start(MockServerConfig::all())
            .await
            .expect("start");
        let base = server.base_url();

        let client = reqwest::Client::new();
        let resp = client
            .post(format!("{base}/v1/auth/login"))
            .json(&serde_json::json!({"username": "u", "password": "p"}))
            .send()
            .await
            .expect("request");
        assert_eq!(resp.status().as_u16(), 200);
        let body: serde_json::Value = resp.json().await.expect("json");
        assert!(body.get("access_token").is_some());
    }

    #[test]
    fn config_all_includes_everything() {
        let c = MockServerConfig::all();
        assert!(c.include_auth);
        assert!(c.include_user);
        assert!(c.include_project);
        assert!(c.include_task);
        assert!(c.include_audit);
        assert!(c.include_healthz);
    }

    #[test]
    fn config_default_is_empty() {
        let c = MockServerConfig::default();
        assert!(!c.include_auth);
        assert!(!c.include_healthz);
    }
}
