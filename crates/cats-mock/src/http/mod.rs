//! HTTP API mock: actix-web 路由 + 响应工厂
//!
//! 引用: 设计书 §4.4
//!
//! 提供:
//! - [`MockServer`]: 启动 actix-web test server (0.0.0.0:0 随机端口), 返回 base_url
//! - [`ResponseBuilder`]: 预制 JSON 响应 (200/201/4xx/5xx, ErrorBody 格式)
//! - [`auth_routes`] / [`user_routes`] / [`project_routes`]: 预制业务路由注册
//!
//! 设计原则:
//! - **不替代真 service 路由**: 仅提供"能跑通端到端"的 mock 响应
//! - **行为可配置**: 每个 route handler 接受 state (e.g. fixed response, dynamic factory)
//! - **生产路径同步**: 路由表与 `api/openapi/cats-openapi-v1.yaml` 保持路径一致

pub mod routes;
pub mod server;
pub mod response;

pub use response::*;
pub use routes::*;
pub use server::*;
