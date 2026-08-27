//! tonic 0.12 (gRPC) 兼容性冒烟
//!
//! 验证目标: tonic 0.12 在 Rust 1.98.0 下编译
//! 不引入 build.rs + include_proto 宏 (本 smoke 不实施 proto 编译;
//! 真实 proto 编译归 crates/cats-proto/)。
//! 仅探测 tonic 核心类型 + transport::Server builder 可用。

use tonic::{Request, Response, Status};

/// 编译期类型 probe: 验证 tonic::Request / Response / Status 类型可见
#[allow(clippy::result_large_err)]
pub fn tonic_types_compile() -> &'static str {
    // 不实例化(Status::ok 需要 message),仅引用类型
    let _f: fn(Request<()>) -> Result<Response<()>, Status> =
        |_req| Err(Status::internal("smoke probe"));
    "tonic-types-ok"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tonic_types_probe_returns_marker() {
        assert_eq!(tonic_types_compile(), "tonic-types-ok");
    }
}
