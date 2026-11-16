"""Unsafe check (spec/09-continuous-integration.md CI-1 `unsafe`, rule Q1).

Every crate root of the workspace (library, binary and procedural macro targets, as listed
by `cargo metadata`) must carry the inner attribute `#![forbid(unsafe_code)]` on a line of
its own.

Usage: python3 scripts/ci/check_unsafe.py [metadata.json]
Without an argument, runs `cargo metadata --format-version 1 --no-deps`.
"""
import json
import pathlib
import re
import subprocess
import sys

ROOT_KINDS = {"lib", "rlib", "dylib", "cdylib", "staticlib", "bin", "proc-macro"}
ATTRIBUTE = re.compile(r"^\s*#!\[\s*forbid\s*\(\s*unsafe_code\s*\)\s*\]\s*$", re.MULTILINE)


def crate_roots(document):
    """Source paths of every crate root of the workspace members, in metadata order."""
    members = set(document["workspace_members"])
    roots = []
    for package in document["packages"]:
        if package["id"] not in members:
            continue
        for target in package["targets"]:
            if ROOT_KINDS.intersection(target["kind"]):
                roots.append(target["src_path"])
    return roots


def forbids_unsafe(source):
    """Whether a crate root source has `#![forbid(unsafe_code)]` on a line of its own."""
    return ATTRIBUTE.search(source) is not None


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
    missing = [
        root
        for root in crate_roots(read_metadata(sys.argv[1:]))
        if not forbids_unsafe(pathlib.Path(root).read_text(encoding="utf-8"))
    ]
    for root in missing:
        print(f"::error file={root},title=unsafe::Q1: crate root lacks #![forbid(unsafe_code)]")
    sys.exit(1 if missing else 0)


if __name__ == "__main__":
    main()
