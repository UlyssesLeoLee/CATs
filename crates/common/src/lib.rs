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

/// `/readyz` 的统一响应体（2026-10-07 新增）
///
/// # 为什么要放在共享 crate 里
///
/// 就绪探针的含义只有一个：**依赖不可用时必须返回非 200**。k8s 的
/// `readinessProbe` 只看状态码、不看 body，所以"body 里写着 `db: "fail"`
/// 但状态码仍是 200"等于骗调度器 —— Pod 会继续被派发流量，而它根本处理不了。
///
/// 各服务自己写一遍这个结构体会立刻漂移（字段、状态词、状态码各不相同），
/// 于是"就绪探针"退化成"每个服务各写各的、其中几个在说谎"。所以形状收在这里、
/// 判定收在 [`ReadyResponse::status_code`]，各服务只负责真的去探一下自己的依赖。
///
/// # 契约
///
/// - `db == "ok"` → `status: "ready"` → **200**
/// - `db == "fail"` → `status: "not_ready"` → **503**
///
/// 503 是硬要求。任何把它改回 200 的改动，都会让该服务的 Pod 在数据库
/// 故障期间继续接流量。
#[derive(Debug, Clone, serde::Serialize)]
pub struct ReadyResponse {
    /// `ready` 或 `not_ready`
    pub status: &'static str,
    /// 服务名（crate 名）
    pub service: &'static str,
    /// 依赖探测结果：`ok` 或 `fail`
    pub db: &'static str,
}

impl ReadyResponse {
    /// 按探测结果构造。`service` 传 `env!("CARGO_PKG_NAME")`。
    pub fn new(service: &'static str, dep_ok: bool) -> Self {
        Self {
            status: if dep_ok { "ready" } else { "not_ready" },
            service,
            db: if dep_ok { "ok" } else { "fail" },
        }
    }

    /// 就绪判定：依赖不可用时返回 **503**，而不是 200。
    ///
    /// 这是本类型存在的唯一理由。
    pub fn status_code(&self) -> u16 {
        if self.db == "ok" {
            200
        } else {
            503
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

    // ---- ReadyResponse（/readyz 统一形状）----
    //
    // 这几条是"就绪探针不说谎"的最小保证。第一版实现把 `db: "fail"`
    // 写在 body 里却仍返回 200，k8s 只看状态码，于是 Pod 继续接流量。

    #[test]
    fn ready_response_ok_yields_200() {
        let r = ReadyResponse::new("audit-service", true);
        assert_eq!(r.status, "ready");
        assert_eq!(r.db, "ok");
        assert_eq!(
            r.status_code(),
            200,
            "依赖可用时必须 200，否则 Pod 永远进不了 Endpoints"
        );
    }

    #[test]
    fn ready_response_fail_yields_503_not_200() {
        let r = ReadyResponse::new("audit-service", false);
        assert_eq!(r.status, "not_ready");
        assert_eq!(r.db, "fail");
        assert_eq!(
            r.status_code(),
            503,
            "依赖不可用时必须 503。返回 200 会让 k8s 继续往这个 Pod 派流量，\
             而它连 DB 都连不上 —— 这正是 2026-10-07 修掉的那个缺陷"
        );
    }

    #[test]
    fn ready_response_serializes_with_stable_keys() {
        let r = ReadyResponse::new("worker-service", true);
        let v: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&r).expect("serialize"))
                .expect("valid json");
        let mut keys: Vec<&str> = v
            .as_object()
            .expect("object")
            .keys()
            .map(|s| s.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["db", "service", "status"], "顶层键必须固定");
        assert_eq!(v["service"], "worker-service");
    }
}
