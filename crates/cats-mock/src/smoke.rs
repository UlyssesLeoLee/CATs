//! 16 个 service 的 boilerplate smoke 测试统一入口
//!
//! 引用: 设计书 §5.2 + §6.3
//!
//! ## 用法
//!
//! 16 个 service 的 `tests/smoke.rs` 全部 16 行 boilerplate 改为 3 行:
//!
//! ```ignore
//! // tests/smoke.rs
//! use cats_mock::smoke::name_matches_crate;
//!
//! name_matches_crate!(env!("CARGO_PKG_NAME"));
//! ```
//!
//! 宏内部自动生成 2 个 `#[test]`:
//! 1. `version_is_semver_like` — 验证 version 以 `0.1.` 开头
//! 2. `name_matches_crate` — 验证传入的 name 与 crate 的 name 一致
//!
//! ## 设计要点
//!
//! - **DRY**: 16 个 service 共享同一组断言
//! - **零 boilerplate**: 1 行 `use` + 1 行宏调用
//! - **不依赖测试函数的 #[test]**: 宏在调用方 crate 中展开,自动生成 #[test]
//!
//! ## 为什么不放在 `tests/` 目录
//!
//! `tests/` 是集成测试目录, 不能被其他 crate 通过 `use` 引用。
//! smoke.rs 必须在 `src/` 下, 作为 lib crate 公开 API 暴露。

/// 宏: 在调用方 crate 的 `tests/smoke.rs` 顶部调用一次
///
/// 自动生成 2 个 `#[test]`:
/// - `version_is_semver_like`: 验证 `CARGO_PKG_VERSION` 以 `0.1.` 开头
/// - `name_matches_crate`: 验证传入的 `name` == `CARGO_PKG_NAME`
///
/// # 用法
///
/// ```ignore
/// // crates/auth-service/tests/smoke.rs
/// use cats_mock::smoke::name_matches_crate;
///
/// name_matches_crate!(env!("CARGO_PKG_NAME"));
/// ```
#[macro_export]
macro_rules! name_matches_crate {
    ($expected:expr) => {
        /// 验证 crate version 以 `0.1.` 开头
        #[test]
        fn version_is_semver_like() {
            let v = env!("CARGO_PKG_VERSION");
            assert!(
                v.starts_with("0.1."),
                "version should start with '0.1.', got {v}"
            );
        }

        /// 验证传入的 name == CARGO_PKG_NAME
        #[test]
        fn name_matches_crate() {
            let expected: &str = $expected;
            let actual = env!("CARGO_PKG_NAME");
            assert_eq!(actual, expected, "CARGO_PKG_NAME mismatch");
        }
    };
}

#[cfg(test)]
mod tests {
    // 验证宏能在自身 crate 内正确展开: 必须在 mod 顶层调用 (与 16 个真实服务的
    // tests/smoke.rs 用法一致), 不能包在另一个 fn 里——包在 fn 里会让宏生成的
    // #[test] 变成嵌套在函数体内的 "unnameable test items", clippy -D warnings
    // 下无法通过 (且实际上也测不出宏是否真的生成了可运行的测试)。
    name_matches_crate!(env!("CARGO_PKG_NAME"));
}
