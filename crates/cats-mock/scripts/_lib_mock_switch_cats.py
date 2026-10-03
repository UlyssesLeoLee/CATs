#!/usr/bin/env python3
"""_lib_mock_switch_cats.py — CATs cats-mock mock switch reader.

Per ULYS-190 §4.4 IM1.0 stage1 brief v0.1 + CATs §4.4 stage2 brief v0.1
(跨项目範式, per G-MS-04 命名一致性):

Reads `crates/cats-mock/.aci.json` + `.mock-cluster.json` and emits a
`mock_switch_trace` dict that the CATs dispatcher / test harness can consume.

Usage:
    from _lib_mock_switch_cats import MockSwitchReader, build_trace_dict

    reader = MockSwitchReader(root=Path("crates/cats-mock"))
    state = reader.read()
    print(state["summary"])

跨语言调用: 通过 subprocess 调 (per 守门 #9); Rust native 版本跨 session.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any


REQUIRED_CLUSTER_KEYS = (
    "cluster_version",
    "project",
    "cluster_id",
    "enabled",
    "mode",
    "aci_compat_version",
)

VALID_MODES = {"offline", "passthrough", "proxy"}


class MockSwitchError(Exception):
    """Raised when .mock-cluster.json or .aci.json has inconsistent state."""


class MockSwitchReader:
    """Reads CATs cats-mock mock switch state from disk."""

    def __init__(self, root: Path) -> None:
        self.root = Path(root)
        self.cluster_path = self.root / ".mock-cluster.json"
        self.aci_path = self.root / ".aci.json"

    def read(self) -> dict[str, Any]:
        """Read both files and return a merged state dict."""
        cluster = self._read_cluster()
        aci = self._read_aci()
        self._validate_compat(cluster, aci)

        plugins_total = 0
        plugins_enabled = 0
        modules_total = 0
        modules_enabled = 0
        plugin_states: dict[str, dict[str, Any]] = {}

        for plugin_id, plugin in aci.get("plugins", {}).items():
            plugins_total += 1
            plugin_enabled = bool(plugin.get("enabled", False))
            if plugin_enabled:
                plugins_enabled += 1

            p_modules_total = 0
            p_modules_enabled = 0
            for module_id, module in plugin.get("modules", {}).items():
                modules_total += 1
                p_modules_total += 1
                if bool(module.get("enabled", False)):
                    modules_enabled += 1
                    p_modules_enabled += 1

            plugin_states[plugin_id] = {
                "plugin_id": plugin_id,
                "enabled": plugin_enabled,
                "default_mode": plugin.get("default_mode", "offline"),
                "modules_total": p_modules_total,
                "modules_enabled": p_modules_enabled,
            }

        return {
            "cluster": {
                "enabled": bool(cluster.get("enabled", False)),
                "mode": cluster.get("mode", "offline"),
                "version": cluster.get("cluster_version", "unknown"),
                "aci_compat_version": cluster.get("aci_compat_version", "unknown"),
                "project": cluster.get("project", "unknown"),
                "cluster_id": cluster.get("cluster_id", "unknown"),
            },
            "plugins": plugin_states,
            "summary": {
                "plugins_total": plugins_total,
                "plugins_enabled": plugins_enabled,
                "modules_total": modules_total,
                "modules_enabled": modules_enabled,
            },
        }

    def _read_cluster(self) -> dict[str, Any]:
        if not self.cluster_path.exists():
            raise MockSwitchError(f".mock-cluster.json not found: {self.cluster_path}")
        try:
            data = json.loads(self.cluster_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            raise MockSwitchError(f"Invalid JSON in {self.cluster_path}: {e}") from e

        for key in REQUIRED_CLUSTER_KEYS:
            if key not in data:
                raise MockSwitchError(
                    f".mock-cluster.json missing required key '{key}': {self.cluster_path}"
                )
        if data["mode"] not in VALID_MODES:
            raise MockSwitchError(
                f".mock-cluster.json mode '{data['mode']}' not in {VALID_MODES}: {self.cluster_path}"
            )
        return data

    def _read_aci(self) -> dict[str, Any]:
        if not self.aci_path.exists():
            raise MockSwitchError(f".aci.json not found: {self.aci_path}")
        try:
            data = json.loads(self.aci_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError as e:
            raise MockSwitchError(f"Invalid JSON in {self.aci_path}: {e}") from e
        return data

    def _validate_compat(self, cluster: dict[str, Any], aci: dict[str, Any]) -> None:
        cluster_ver = cluster.get("aci_compat_version")
        aci_ver = aci.get("aci_compat_version")
        if cluster_ver is None or aci_ver is None:
            if cluster_ver != aci_ver:
                raise MockSwitchError(
                    f"aci_compat_version mismatch: cluster={cluster_ver}, aci={aci_ver}"
                )
            return
        if cluster_ver != aci_ver:
            raise MockSwitchError(
                f"aci_compat_version mismatch: cluster={cluster_ver}, aci={aci_ver}"
            )


def build_trace_dict(state: dict[str, Any]) -> str:
    cluster = state["cluster"]
    summary = state["summary"]
    parts = [
        f"cluster.enabled={cluster['enabled']}",
        f"mode={cluster['mode']}",
    ]
    plugin_strs = []
    for pname, pstate in state["plugins"].items():
        if not pstate["enabled"]:
            continue
        plugin_strs.append(f"{pname}({pstate['modules_enabled']}m)")
    if plugin_strs:
        parts.append(f"plugins=[{','.join(plugin_strs)}]")
    parts.append(
        f"={summary['modules_enabled']}/{summary['modules_total']} modules"
    )
    return ",".join(parts)


if __name__ == "__main__":
    import sys

    args = sys.argv[1:]
    json_only = False
    positional = []
    i = 0
    while i < len(args):
        a = args[i]
        if a == "--json":
            json_only = True
        else:
            positional.append(a)
        i += 1

    if not positional:
        print("usage: _lib_mock_switch_cats.py [--json] <cats-mock-root>", file=sys.stderr)
        sys.exit(2)
    reader = MockSwitchReader(Path(positional[0]))
    state = reader.read()
    if json_only:
        print(json.dumps(state, ensure_ascii=False, indent=2))
    else:
        print(json.dumps(state, ensure_ascii=False, indent=2))
        print("---")
        print("trace:", build_trace_dict(state))
