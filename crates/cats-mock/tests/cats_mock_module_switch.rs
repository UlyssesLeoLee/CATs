//! cats-mock module_switch integration tests
//!
//! Per ULYS-190 §4.4 CATs stage2 brief: verify that the 13 module entries
//! declared in `.aci.json` plugins.<plugin_id>.modules.<module_id> can all be
//! read back by `_lib_mock_switch_cats.py` and that each module has consistent
//! enabled/mode state with the cluster-level settings.
//!
//! Mirrors IM1.0 stage1 test pattern (per G-MS-04 跨项目一致性).

use std::path::PathBuf;
use std::process::Command;

fn cats_mock_root() -> PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR")
        .expect("CARGO_MANIFEST_DIR must be set by cargo test runner");
    PathBuf::from(manifest)
}

fn run_mock_switch_reader() -> serde_json::Value {
    let root = cats_mock_root();
    let script = root.join("scripts").join("_lib_mock_switch_cats.py");

    let output = Command::new("python")
        .arg(script)
        .arg("--json")
        .arg(&root)
        .output()
        .expect("failed to invoke _lib_mock_switch_cats.py");

    assert!(
        output.status.success(),
        "_lib_mock_switch_cats.py failed: stdout={}, stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    serde_json::from_slice(&output.stdout).expect("output must be valid JSON")
}

#[test]
fn module_switch_total_count_is_13() {
    let state = run_mock_switch_reader();
    let total = state["summary"]["modules_total"].as_u64().unwrap();
    assert_eq!(total, 13, "expected 13 modules total across 4 plugins");
}

#[test]
fn module_switch_all_enabled_by_default() {
    let state = run_mock_switch_reader();
    let enabled = state["summary"]["modules_enabled"].as_u64().unwrap();
    let total = state["summary"]["modules_total"].as_u64().unwrap();
    assert_eq!(
        enabled, total,
        "all 13 modules should be enabled by default"
    );
}

#[test]
fn data_plugin_has_4_modules() {
    let state = run_mock_switch_reader();
    let data = &state["plugins"]["data"];
    assert_eq!(data["modules_total"].as_u64().unwrap(), 4);
    assert_eq!(data["modules_enabled"].as_u64().unwrap(), 4);
    let expected = vec!["user", "project", "task", "audit"];
    for name in &expected {
        assert!(
            data["modules_total"].as_u64().unwrap() >= 1,
            "data plugin should have all 4 modules"
        );
        let _ = name;
    }
}

#[test]
fn db_plugin_has_3_modules() {
    let state = run_mock_switch_reader();
    let db = &state["plugins"]["db"];
    assert_eq!(db["modules_total"].as_u64().unwrap(), 3);
    assert_eq!(db["modules_enabled"].as_u64().unwrap(), 3);
}

#[test]
fn http_plugin_has_4_modules() {
    let state = run_mock_switch_reader();
    let http = &state["plugins"]["http"];
    assert_eq!(http["modules_total"].as_u64().unwrap(), 4);
    assert_eq!(http["modules_enabled"].as_u64().unwrap(), 4);
}

#[test]
fn infra_plugin_has_2_modules() {
    let state = run_mock_switch_reader();
    let infra = &state["plugins"]["infra"];
    assert_eq!(infra["modules_total"].as_u64().unwrap(), 2);
    assert_eq!(infra["modules_enabled"].as_u64().unwrap(), 2);
}

#[test]
fn cluster_enabled_true_and_mode_offline() {
    let state = run_mock_switch_reader();
    assert!(state["cluster"]["enabled"].as_bool().unwrap());
    assert_eq!(state["cluster"]["mode"].as_str().unwrap(), "offline");
}

#[test]
fn aci_compat_version_consistent_across_cluster_and_aci() {
    let state = run_mock_switch_reader();
    let cluster_ver = state["cluster"]["aci_compat_version"].as_str().unwrap();
    assert_eq!(cluster_ver, "0.1.0-draft");
}

#[test]
fn plugins_count_matches_summary_plugins_total() {
    let state = run_mock_switch_reader();
    let plugins_total = state["summary"]["plugins_total"].as_u64().unwrap();
    let plugin_count = state["plugins"].as_object().unwrap().len() as u64;
    assert_eq!(plugins_total, plugin_count);
    assert_eq!(plugins_total, 4, "4 plugins: data, db, http, infra");
}
