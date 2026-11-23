"""Workspace dependency graph check (spec/09-continuous-integration.md CI-1 `dependency-graph`).

Reads `cargo metadata` and fails when a dependency between workspace crates breaks the
direction of spec/05-engineering-standards.md section 5.2 (S6, S7), when the graph has a
cycle (S6), when a wrapper depends on a workspace crate other than `core` (S8), or when any
crate depends on a tool. Tools (the test-tooling crates `conformance-runner`, ARC-5.3, and `benchmarks`, R4) are
outside the layers of section 5.2: nothing may depend on them, and they depend only on the
crates listed for them.
Every dependency kind counts: normal, build and dev.

Usage: python3 scripts/ci/check_dependency_graph.py [metadata.json]
Without an argument, runs `cargo metadata --format-version 1 --no-deps`.
"""
import json
import pathlib
import subprocess
import sys

PACKAGE_PREFIX = "menura-"

# Layers of section 5.2, from the bottom up. A crate depends only on crates of a strictly lower
# layer, so stages (one layer) never depend on each other (S7).
LAYERS = [
    ["contracts"],
    ["model"],
    ["expressions"],
    ["representations"],
    ["security"],
    ["e0-load", "e1-parse", "e2-bind", "e3-paths", "e4-plan", "e5-optimize"],
    ["generator-duckdb"],
    ["core"],
    ["cli", "mcp"],
]

WRAPPERS = ["cli", "mcp"]
# ARC-5.3, R4: each tool and the only workspace crates it may depend on.
TOOLS = {"conformance-runner": ["contracts", "core"], "benchmarks": ["core"]}
WRAPPER_DEPENDENCY = "core"


def crate_names():
    """Every crate of section 5.2, without the package prefix."""
    return [name for layer in LAYERS for name in layer]


def layer_of(name):
    """Index of the layer holding a crate, or None if the crate is in no layer."""
    for index, layer in enumerate(LAYERS):
        if name in layer:
            return index
    return None


def short_name(package_name):
    """A package name without the workspace prefix."""
    return package_name.removeprefix(PACKAGE_PREFIX)


def workspace_graph(document):
    """Map each workspace member to the sorted set of workspace members it depends on."""
    members = set(document["workspace_members"])
    packages = [package for package in document["packages"] if package["id"] in members]
    names = {package["name"] for package in packages}
    graph = {}
    for package in packages:
        dependencies = {
            short_name(dependency["name"])
            for dependency in package["dependencies"]
            if dependency["name"] in names
        }
        graph[short_name(package["name"])] = sorted(dependencies)
    return dict(sorted(graph.items()))


def find_cycles(graph):
    """Every elementary cycle reached by a depth-first search, each reported once."""
    cycles = []
    seen = set()
    state = {}

    def visit(node, path):
        state[node] = "open"
        path.append(node)
        for dependency in graph.get(node, []):
            if state.get(dependency) == "open":
                cycle = path[path.index(dependency):]
                key = frozenset(cycle)
                if key not in seen:
                    seen.add(key)
                    cycles.append(cycle + [dependency])
            elif dependency not in state:
                visit(dependency, path)
        path.pop()
        state[node] = "closed"

    for node in graph:
        if node not in state:
            visit(node, [])
    return cycles


def find_violations(graph):
    """Every S6, S7 and S8 violation of the graph, as readable lines."""
    violations = []
    for name in graph:
        if layer_of(name) is None and name not in TOOLS:
            violations.append(f"S6: `{name}` is not in a layer of section 5.2")
    for dependent, dependencies in graph.items():
        dependent_layer = layer_of(dependent)
        for dependency in dependencies:
            if dependency in TOOLS:
                violations.append(f"S6: `{dependent}` depends on the tool `{dependency}`")
                continue
            if dependent in TOOLS:
                if dependency not in TOOLS[dependent]:
                    violations.append(
                        f"ARC-5.3: tool `{dependent}` depends on `{dependency}`, which is not allowed"
                    )
                continue
            dependency_layer = layer_of(dependency)
            if dependent_layer is None or dependency_layer is None:
                continue
            if dependency_layer >= dependent_layer:
                violations.append(
                    f"S6/S7: `{dependent}` depends on `{dependency}`, which is not in a lower layer"
                )
        if dependent in WRAPPERS:
            for dependency in dependencies:
                if dependency != WRAPPER_DEPENDENCY:
                    violations.append(f"S8: wrapper `{dependent}` depends on `{dependency}`, not only on `core`")
    for cycle in find_cycles(graph):
        violations.append("S6: cycle " + " -> ".join(f"`{name}`" for name in cycle))
    return violations


def read_metadata(arguments):
    """The metadata document, from a file argument or from `cargo metadata`."""
    if arguments:
        return json.loads(pathlib.Path(arguments[0]).read_text(encoding="utf-8"))
    output = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return json.loads(output)


def main():
    violations = find_violations(workspace_graph(read_metadata(sys.argv[1:])))
    for violation in violations:
        print(f"::error title=dependency-graph::{violation}")
    sys.exit(1 if violations else 0)


if __name__ == "__main__":
    main()
