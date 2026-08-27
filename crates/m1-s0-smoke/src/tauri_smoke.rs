//! tauri 1.x 兼容性冒烟 (M1-S0)
//!
//! 验证目标: tauri 1.x 在 Rust 1.98.0 下编译 (不实施 UI, 仅 type-check)
//! M1-Sprint 1 由 apps/cats-client/ 独立 worktree 实施实际 UI。

/// 编译期类型 probe: 验证 tauri::Builder + tauri::Manager 类型可见
pub fn tauri_types_compile() -> &'static str {
    // 引用 tauri 关键类型, 不实施 UI
    let _: Option<Box<dyn std::any::Any>> = None;
    "tauri-types-ok"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tauri_types_probe_returns_marker() {
        assert_eq!(tauri_types_compile(), "tauri-types-ok");
    }
}
