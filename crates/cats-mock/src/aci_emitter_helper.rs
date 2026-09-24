//! `aci_emitter_helper` — CATs ACI emitter helper (per ULYS-191 §4.3.2 brief v0.1).
//!
//! 1 公开函数 `emit_smoke_assertion()`, 与 IDE1.0 §4.2.2 + RGS §4.3.1 + IM1.0 §4.3.3 pattern 1:1 对齐.
//!
//! 守门:
//! - #7 unsafe_code=forbid: cats-mock/Cargo.toml [lints] workspace = true (CATs workspace.lints.rust.unsafe_code = "forbid")
//! - #11 缺标比错标: git dep 锁 rev=df28c56, 字段名 1:1 对齐 aci-emitter + Star Python emitter
//! - #13 W/T/M: emit 公开函数 + 2 单测 + IT 配对
//! - #24 vendor 中立: 仅 1 git dep aci-emitter (自家)

use aci_emitter::{AciEmitter, ExpectActual, ExpectValueType, Layer, Scope, Severity, Status};

/// Emit 一条 CATs placeholder smoke assertion.
///
/// 默认值:
/// - assertion_id: `"cats-mock:smoke:g-1"`
/// - scope: project=`cats-mock`, module=`smoke`
/// - expect: response_within_ms = 100, "CATs API should respond within 100ms"
/// - actual: response_within_ms = 20, "measured 20ms (placeholder)"
/// - status: PASS
/// - severity: info
///
/// # Returns
///
/// `Assertion` (per aci-emitter v0.1.0)
///
/// # Example
///
/// ```
/// use cats_mock::aci_emitter_helper::emit_smoke_assertion;
/// let a = emit_smoke_assertion();
/// assert_eq!(a.assertion_id, "cats-mock:smoke:g-1");
/// ```
#[must_use]
pub fn emit_smoke_assertion() -> aci_emitter::Assertion {
    let em = AciEmitter::new(Layer::It);
    em.build(
        "cats-mock:smoke:g-1".to_string(),
        Scope::new(
            "cats-mock".to_string(),
            Some("smoke".to_string()),
            None,
            None,
            None,
            None,
        ),
        ExpectActual::new(
            ExpectValueType::ResponseWithinMs,
            serde_json::json!(100),
            "CATs API should respond within 100ms",
        ),
        ExpectActual::new(
            ExpectValueType::ResponseWithinMs,
            serde_json::json!(20),
            "measured 20ms (placeholder)",
        ),
        Status::Pass,
        Severity::Info,
        "actual << expect (5x margin) — placeholder per §4.3.2 brief v0.1",
    )
    .expect("smoke assertion build must succeed")
}

/// 公开所有 10 必填字段名常量 (供 IT 与 cross-language parity 测试引用).
///
/// Per `.aci.json` schema_required_fields, alphabetic 排序.
pub const REQUIRED_FIELDS: [&str; 10] = [
    "aci_version",
    "actual",
    "assertion_id",
    "captured_at",
    "expect",
    "layer",
    "reasoning",
    "scope",
    "severity",
    "status",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smoke_basic() {
        let a = emit_smoke_assertion();
        assert_eq!(a.assertion_id, "cats-mock:smoke:g-1");
        assert_eq!(a.aci_version, "0.1.0-draft");
        assert_eq!(a.layer, Layer::It);
        assert_eq!(a.status, Status::Pass);
        assert_eq!(a.severity, Severity::Info);
        assert_eq!(a.scope.project, "cats-mock");
        assert_eq!(a.scope.module.as_deref(), Some("smoke"));
    }

    #[test]
    fn test_required_fields_sorted() {
        let mut sorted = REQUIRED_FIELDS;
        sorted.sort_unstable();
        for (a, b) in REQUIRED_FIELDS.iter().zip(sorted.iter()) {
            assert_eq!(a, b, "REQUIRED_FIELDS must be in alphabetic order");
        }
    }
}
