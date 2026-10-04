//! `cats-common` — CATs 共享库
//!
//! 引用: doc/02-基础设计/技术选型/CATs_Rust技术选型书_v1.0.md §4.3/§9/§11
//! 引用: doc/02-基础设计/技术选型/CATs_技术基线_v1.0.md §1
//!
//! M0 阶段：本 crate 仅暴露 `version()` 与基础配置读取器占位。
//! 真正的共享类型 / 错误体系 / tracing 初始化器将在 M1 阶段按 Rust 选型书落地。

// 2026-10-04 接线：本文件原先没有这一行，`error.rs`（550 行的
// `ErrorCode` / `CatsError` / `Result` / `ErrorBody` + HTTP/gRPC 映射）
// **从未被编译**。后果不是"少了个模块"，而是全仓的错误模型分裂：
// 凡是用 `cats_common::{CatsError, ErrorCode}` 的文件都在未编译的
// 模块里（audit/file/notification 的 db.rs·handlers.rs·consumer.rs、
// translation-core 的 service.rs·db.rs·ai_gateway.rs、
// cats-rbac 的 service_helpers.rs、worker-service 的 scheduler.rs），
// 而真正在编译的服务（auth/user/project/task/report/bff/ai-gateway）
// 各自内联了一套错误处理，没有共享类型。
pub mod error;

// 根路径 re-export。仓库里两种写法都在用:
//   use cats_common::{CatsError, ErrorCode};              （audit/file/notification/…）
//   use cats_common::error::{CatsError, ErrorCode};       （worker-service）
// 两个都留着, 不要求调用方统一改写 —— 共享库加 re-export 的成本是一行,
// 逼 6 个 crate 改 import 的成本高得多, 而且改 import 会把"这个模块刚接上"
// 和"顺手统一风格"两件事混在一起, 掩盖真正的原因。
pub use error::{cats_error_to_response, CatsError, ErrorBody, ErrorCode, Result};

/// 当前 crate 语义版本（与 workspace.package.version 同步）
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// 当前 crate 名称
pub const NAME: &str = env!("CARGO_PKG_NAME");

/// 返回 crate 版本字符串（用于健康检查 / 调试输出）
///
/// # Examples
///
/// ```
/// use cats_common::version;
/// assert!(version().starts_with("0.1."));
/// ```
pub fn version() -> &'static str {
    VERSION
}

/// 返回 crate 名称
///
/// # Examples
///
/// ```
/// use cats_common::name;
/// assert_eq!(name(), "cats-common");
/// ```
pub fn name() -> &'static str {
    NAME
}

/// 应用元信息（用于 `/healthz` 等健康检查端点返回 JSON）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AppMeta {
    /// 应用名称（crate 名）
    pub name: String,
    /// 语义版本
    pub version: String,
}

impl AppMeta {
    /// 构造当前 crate 的 `AppMeta`
    pub fn current() -> Self {
        Self {
            name: NAME.to_string(),
            version: VERSION.to_string(),
        }
    }
}

/// 初始化 tracing subscriber（占位实现，M1 阶段替换为完整初始化器）
///
/// M0 阶段：仅设置默认 subscriber，环境变量 `RUST_LOG` 控制级别。
/// 真正的 OpenTelemetry exporter、JSON 输出格式等在 M1-S0 落地（per Rust 选型书 §9.2）。
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,cats_common=debug"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}

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
    fn name_is_cats_common() {
        assert_eq!(name(), "cats-common");
    }

    #[test]
    fn app_meta_serializable() {
        let meta = AppMeta::current();
        let json = serde_json::to_string(&meta).unwrap();
        assert!(json.contains("cats-common"));
        assert!(json.contains("0.1."));
    }
}
