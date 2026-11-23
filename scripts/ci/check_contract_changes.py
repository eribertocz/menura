"""Check that every changed contract schema has a bumped version and an updated example (A3, A5, CON-3).

Every schema under spec/contracts/schemas/ must be a valid JSON Schema 2020-12 document. A schema
whose content differs from its version at BASE must declare a higher version in its `$id`, by the
rules of architecture section 3.3: a major bump when a member is removed, a type is narrowed or a
new required member appears, and at least a minor bump for any other change. A changed schema
must also come with at least one changed example under spec/contracts/examples/.

Usage: python3 scripts/ci/check_contract_changes.py BASE
BASE is a commit; the comparison uses the merge base of BASE and HEAD.
"""
import json
import pathlib
import re
import subprocess
import sys

import jsonschema

ROOT = pathlib.Path(__file__).resolve().parent.parent.parent
sys.path.insert(0, str(ROOT / "scripts"))

from check_examples import PAIRS  # noqa: E402

SCHEMAS = "spec/contracts/schemas"
EXAMPLES = "spec/contracts/examples"
SCHEMA_SUFFIX = ".schema.json"
VERSION_IN_ID = re.compile(r"/(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$")

NONE, MINOR, MAJOR = 0, 1, 2
SCHEMA_MAPS = {"properties", "patternProperties", "$defs", "dependentSchemas"}
SCHEMA_LISTS = {"allOf", "anyOf", "oneOf", "prefixItems"}


def schema_version(schema):
    """Return the (major, minor, patch) version at the end of the schema's `$id`, or None."""
    match = VERSION_IN_ID.search(str(schema.get("$id", "")))
    return tuple(int(part) for part in match.groups()) if match else None


def without_id(schema):
    """Return the schema without its `$id`, so that a version bump alone is not a content change."""
    return {key: value for key, value in schema.items() if key != "$id"}


def type_set(value):
    """Return the set of JSON types a `type` keyword allows."""
    return {value} if isinstance(value, str) else set(value)


def breaking_changes(before, after, path="#"):
    """Return the breaking changes between two schema nodes: removed members, narrowed types, new required members."""
    if not isinstance(before, dict) or not isinstance(after, dict):
        return []
    changes = []
    before_properties = before.get("properties", {})
    after_properties = after.get("properties", {})
    if isinstance(before_properties, dict) and isinstance(after_properties, dict):
        for name in sorted(set(before_properties) - set(after_properties)):
            changes.append(f"{path}: member {name} removed")
    if "type" in before and "type" in after and not type_set(after["type"]) >= type_set(before["type"]):
        changes.append(f"{path}: type narrowed from {before['type']} to {after['type']}")
    for name in sorted(set(after.get("required", [])) - set(before.get("required", []))):
        changes.append(f"{path}: new required member {name}")
    for key in sorted(set(before) & set(after)):
        changes.extend(breaking_children(key, before[key], after[key], f"{path}/{key}"))
    return changes


def breaking_children(keyword, before, after, path):
    """Compare the sub-schemas under one keyword: maps of schemas by name, lists by position, else one schema."""
    if keyword in SCHEMA_MAPS and isinstance(before, dict) and isinstance(after, dict):
        pairs = [(f"{path}/{name}", before[name], after[name]) for name in sorted(set(before) & set(after))]
    elif keyword in SCHEMA_LISTS and isinstance(before, list) and isinstance(after, list):
        pairs = [(f"{path}/{index}", old, new) for index, (old, new) in enumerate(zip(before, after))]
    elif keyword not in SCHEMA_MAPS | SCHEMA_LISTS:
        pairs = [(path, before, after)]
    else:
        pairs = []
    changes = []
    for child_path, old, new in pairs:
        changes.extend(breaking_changes(old, new, child_path))
    return changes


def required_bump(before, after):
    """Return NONE, MINOR or MAJOR: the smallest version bump the change from before to after needs."""
    if without_id(before) == without_id(after):
        return NONE
    return MAJOR if breaking_changes(before, after) else MINOR


def bump_problem(name, before, after):
    """Return a message when the version of a changed schema is not bumped enough, else None."""
    needed = required_bump(before, after)
    if needed == NONE:
        return None
    old, new = schema_version(before), schema_version(after)
    if old is None or new is None:
        return f"{name}: `$id` does not end with a MAJOR.MINOR.PATCH version"
    if needed == MAJOR and new[0] <= old[0]:
        details = "; ".join(breaking_changes(before, after))
        return f"{name}: breaking change needs a major bump over {format_version(old)}, found {format_version(new)} ({details})"
    if new[:2] <= old[:2]:
        return f"{name}: changed schema needs at least a minor bump over {format_version(old)}, found {format_version(new)}"
    return None


def format_version(version):
    """Write a version tuple as MAJOR.MINOR.PATCH."""
    return ".".join(str(part) for part in version)


def examples_of(schema_name, example_names):
    """Return the example file names that belong to the contract of the given schema name."""
    owners = {f"{example}.example.json": schema for example, schema in PAIRS.items()}
    return sorted(name for name in example_names if owners.get(name) == schema_name)


def git(*arguments):
    """Run git in the repository root and return its standard output."""
    return subprocess.run(["git", *arguments], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def files_at(commit, directory):
    """Return {file name: content} for the files of a directory at a commit."""
    names = git("ls-tree", "--name-only", f"{commit}:{directory}").split()
    return {name: git("show", f"{commit}:{directory}/{name}") for name in names}


def files_in_tree(directory):
    """Return {file name: content} for the files of a directory in the working tree."""
    return {path.name: path.read_text(encoding="utf-8") for path in sorted((ROOT / directory).iterdir()) if path.is_file()}


def find_problems(base_schemas, head_schemas, base_examples, head_examples):
    """Return every problem of the head contracts compared with the base, as messages."""
    problems = []
    changed_examples = {name for name, text in head_examples.items() if base_examples.get(name) != text}
    for file_name in sorted(head_schemas):
        if not file_name.endswith(SCHEMA_SUFFIX):
            continue
        name = file_name[: -len(SCHEMA_SUFFIX)]
        try:
            after = json.loads(head_schemas[file_name])
            jsonschema.Draft202012Validator.check_schema(after)
        except (ValueError, jsonschema.SchemaError) as error:
            problems.append(f"{name}: not a valid JSON Schema 2020-12 document: {error}")
            continue
        if file_name in base_schemas:
            before = json.loads(base_schemas[file_name])
            if required_bump(before, after) == NONE:
                continue
            problem = bump_problem(name, before, after)
            if problem:
                problems.append(problem)
        if not examples_of(name, changed_examples):
            problems.append(f"{name}: schema changed but no example of this contract changed")
    return problems


def main(arguments):
    if len(arguments) != 1:
        print(__doc__)
        return 2
    base = git("merge-base", arguments[0], "HEAD").strip()
    problems = find_problems(
        files_at(base, SCHEMAS), files_in_tree(SCHEMAS), files_at(base, EXAMPLES), files_in_tree(EXAMPLES)
    )
    for problem in problems:
        print(problem)
    print(f"Compared {SCHEMAS} with {base}: {len(problems)} problem(s).")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
