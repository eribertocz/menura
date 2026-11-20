"""Unit tests of the dependency-graph check. Run: python3 -m unittest discover -s scripts/ci -p 'test_dependency_graph.py'"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import check_dependency_graph  # noqa: E402

PREFIX = check_dependency_graph.PACKAGE_PREFIX

# The edges of the workspace as specified by spec/05-engineering-standards.md section 5.2.
VALID_EDGES = [
    ("model", "contracts"),
    ("expressions", "model"),
    ("representations", "model"),
    ("security", "model"),
    ("security", "representations"),
    ("e0-load", "security"),
    ("e1-parse", "security"),
    ("e2-bind", "security"),
    ("e3-paths", "security"),
    ("e4-plan", "security"),
    ("e5-optimize", "security"),
    ("generator-duckdb", "e5-optimize"),
    ("core", "e0-load"),
    ("core", "generator-duckdb"),
    ("cli", "core"),
]


def metadata(edges, members=None, external=None):
    """Build a minimal `cargo metadata --no-deps` document for the given workspace edges."""
    names = members if members is not None else check_dependency_graph.crate_names()
    external = external or {}
    packages = []
    for name in names:
        dependencies = [
            {"name": PREFIX + dependency, "kind": None, "path": f"/w/crates/{dependency}"}
            for dependent, dependency in edges
            if dependent == name
        ]
        dependencies += [
            {"name": dependency, "kind": None} for dependency in external.get(name, [])
        ]
        packages.append({"name": PREFIX + name, "id": f"id-{name}", "dependencies": dependencies})
    return {"packages": packages, "workspace_members": [f"id-{name}" for name in names]}


def violations(document):
    return check_dependency_graph.find_violations(check_dependency_graph.workspace_graph(document))


class DependencyGraphTest(unittest.TestCase):
    """S6, S7, S8, ARC-5.3."""

    def test_con_0_5_packages_carry_the_project_name(self):
        self.assertEqual(PREFIX, "menura-")

    def test_s6_specified_graph_passes(self):
        self.assertEqual(violations(metadata(VALID_EDGES)), [])

    def test_s6_reversed_dependency_fails(self):
        found = violations(metadata(VALID_EDGES + [("contracts", "model")]))
        self.assertTrue(any("`contracts` depends on `model`" in line for line in found), found)

    def test_s6_cycle_fails(self):
        found = violations(metadata(VALID_EDGES + [("contracts", "model")]))
        self.assertTrue(any(line.startswith("S6: cycle") for line in found), found)

    def test_s6_cycle_is_reported_once(self):
        found = violations(metadata(VALID_EDGES + [("contracts", "model")]))
        self.assertEqual(len([line for line in found if line.startswith("S6: cycle")]), 1, found)

    def test_s6_self_dependency_is_a_cycle(self):
        found = violations(metadata(VALID_EDGES + [("model", "model")]))
        self.assertTrue(any(line.startswith("S6: cycle") for line in found), found)

    def test_s6_unknown_workspace_member_fails(self):
        found = violations(metadata(VALID_EDGES, members=check_dependency_graph.crate_names() + ["extra"]))
        self.assertTrue(any("`extra` is not in a layer" in line for line in found), found)

    def test_s6_dev_dependency_counts(self):
        document = metadata(VALID_EDGES)
        for package in document["packages"]:
            if package["name"] == PREFIX + "model":
                package["dependencies"].append(
                    {"name": PREFIX + "security", "kind": "dev", "path": "/w/crates/security"}
                )
        self.assertTrue(any("`model` depends on `security`" in line for line in violations(document)))

    def test_s6_renamed_dependency_counts(self):
        document = metadata(VALID_EDGES)
        for package in document["packages"]:
            if package["name"] == PREFIX + "model":
                package["dependencies"].append(
                    {"name": PREFIX + "core", "rename": "engine", "kind": None, "path": "/w/crates/core"}
                )
        self.assertTrue(any("`model` depends on `core`" in line for line in violations(document)))

    def test_s6_external_dependencies_are_ignored(self):
        self.assertEqual(violations(metadata(VALID_EDGES, external={"cli": ["clap"]})), [])

    def test_s7_stage_depending_on_another_stage_fails(self):
        found = violations(metadata(VALID_EDGES + [("e1-parse", "e0-load")]))
        self.assertTrue(any("`e1-parse` depends on `e0-load`" in line for line in found), found)

    def test_s6_layers_order_the_shared_crates(self):
        order = check_dependency_graph.crate_names()
        shared = ["model", "expressions", "representations", "security"]
        self.assertEqual([name for name in order if name in shared], shared)

    def test_s10_expressions_depending_on_a_stage_fails(self):
        found = violations(metadata(VALID_EDGES + [("expressions", "e0-load")]))
        self.assertTrue(any("`expressions` depends on `e0-load`" in line for line in found), found)

    def test_s9_representations_depending_on_security_fails(self):
        found = violations(metadata(VALID_EDGES + [("representations", "security")]))
        self.assertTrue(any("`representations` depends on `security`" in line for line in found), found)

    def test_s9_representations_depending_on_a_stage_fails(self):
        found = violations(metadata(VALID_EDGES + [("representations", "e0-load")]))
        self.assertTrue(any("`representations` depends on `e0-load`" in line for line in found), found)

    def test_s6_tool_outside_the_layers_passes(self):
        members = check_dependency_graph.crate_names() + list(check_dependency_graph.TOOLS)
        self.assertEqual(violations(metadata(VALID_EDGES, members=members)), [])

    def test_arc_5_3_tool_depending_on_core_and_contracts_passes(self):
        members = check_dependency_graph.crate_names() + list(check_dependency_graph.TOOLS)
        edges = VALID_EDGES + [("conformance-runner", "core"), ("conformance-runner", "contracts")]
        self.assertEqual(violations(metadata(edges, members=members)), [])

    def test_arc_5_3_tool_depending_on_a_stage_fails(self):
        members = check_dependency_graph.crate_names() + list(check_dependency_graph.TOOLS)
        found = violations(metadata(VALID_EDGES + [("conformance-runner", "e0-load")], members=members))
        self.assertTrue(any(line.startswith("ARC-5.3:") for line in found), found)

    def test_s6_crate_depending_on_a_tool_fails(self):
        members = check_dependency_graph.crate_names() + list(check_dependency_graph.TOOLS)
        found = violations(metadata(VALID_EDGES + [("cli", "conformance-runner")], members=members))
        self.assertTrue(any("depends on the tool `conformance-runner`" in line for line in found), found)

    def test_s6_core_depending_on_a_tool_fails(self):
        members = check_dependency_graph.crate_names() + list(check_dependency_graph.TOOLS)
        found = violations(metadata(VALID_EDGES + [("core", "conformance-runner")], members=members))
        self.assertTrue(any("`core` depends on the tool" in line for line in found), found)

    def test_s8_wrapper_depending_on_a_stage_fails(self):
        found = violations(metadata(VALID_EDGES + [("cli", "model")]))
        self.assertTrue(any(line.startswith("S8:") and "`model`" in line for line in found), found)


if __name__ == "__main__":
    unittest.main()
