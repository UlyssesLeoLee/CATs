//! HTTP client + JWT 注入 + 错误信封解析
//!
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §1.3（统一错误信封）
//! 引用: doc/02-基础设计/接口设计/CATs_接口设计书_v2.0.md §1.4（错误码表）
//!
//! MVP 阶段:
//! - HTTP 方法走 reqwest::Client
//! - 401 → 自动用 refresh_token 调一次 /v1/auth/refresh, 重试原请求一次
//! - 错误信封 schema (per §1.3 v2.0+2 patch):
//!     `{"error": {"code": "...", "message": "...", "trace_id": "...", "details": {...}}}`
//! - 非 2xx 响应统一解析成 `ApiError` 返回
//!
//! 已知缺口（per apps/cats-client/TODO.md）:
//! - 401 重试风暴防护（Idempotency-Key 透传）未实现
//! - 限流退避（429 RATE_LIMITED）未实现

pub mod auth;
pub mod client;
pub mod error;
pub mod projects;
pub mod tasks;
pub mod translate;

pub use client::ApiClient;
pub use error::{ApiError, ApiErrorBody, ApiErrorEnvelope};