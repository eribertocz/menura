"""Rule coverage check (spec/09-continuous-integration.md CI-1 `rule-coverage`, rule T1).

Reads the rule IDs of `spec/INDEX.md` and lists every behavioral rule ID that no conformance
case cites in its `rules`, and every process rule ID that no CI workflow and no item of the
`review-pr` skill cites (spec/README.md Conventions). A citation of an item `X.n` also covers
rule `X`. Rule IDs whose prefix is in neither list are reported as unclassified.

In version 0.1.0 the check only reports; from 0.2.0 on it fails when anything is uncovered.

Usage: python3 scripts/ci/check_rule_coverage.py [repository-root]
Requires PyYAML.
"""
import pathlib
import re
import sys
import tomllib

import yaml

# Prefixes of spec/README.md Conventions. Backlog items (`F`) are not rules.
BEHAVIORAL = re.compile(
    r"^(?:E[0-6][A-Z]?-|(?:STG|CAT|QRY|SEC|PLN|DSC|CHK|MDL|CON|ADD|POP|DER|CTX|SCY|SUM|EXT|DEV)-"
    r"|OSS-[25](?:\.|$)|ARC-5(?:\.|$))"
)
PROCESS = re.compile(r"^(?:[PASQTR]\d|(?:WF|CS|CI)-|OSS-[134](?:\.|$))")
BACKLOG = re.compile(r"^F\d")

INDEX_RULE = re.compile(r"^\|[^|]*\|[^|]*\|\s*rule\s*\|\s*([^|\s]+)\s*\|\s*$")
RULE_TOKEN = re.compile(r"\b(?:[A-Z]{1,4}\d+(?:\.\d+)*|[A-Z]{1,4}\d?[A-Z]?-\d+(?:\.\d+)*)\b")
RANGE_TOKEN = re.compile(r"\b([A-Z]{1,4})(\d+)\s*[–-]\s*(?:\1)?(\d+)\b")

FAILING_FROM = (0, 2, 0)


def index_rules(index_text):
    """Rule IDs of `spec/INDEX.md`, in file order, without repetition."""
    rules = []
    for line in index_text.splitlines():
        match = INDEX_RULE.match(line)
        if match and match.group(1) not in rules:
            rules.append(match.group(1))
    return rules


def classify(rule):
    """`behavioral`, `process`, `backlog` or `unclassified`, by prefix."""
    if BEHAVIORAL.match(rule):
        return "behavioral"
    if PROCESS.match(rule):
        return "process"
    if BACKLOG.match(rule):
        return "backlog"
    return "unclassified"


def cited_in_text(text):
    """Rule IDs cited in free text, with ranges such as `T1–T4` expanded."""
    cited = set(RULE_TOKEN.findall(text))
    for prefix, first, last in RANGE_TOKEN.findall(text):
        cited.update(f"{prefix}{number}" for number in range(int(first), int(last) + 1))
    return cited


def cited_in_cases(case_texts):
    """Rule IDs listed under `rules` by the given case files."""
    cited = set()
    for text in case_texts:
        document = yaml.safe_load(text)
        if isinstance(document, dict) and isinstance(document.get("rules"), list):
            cited.update(str(rule) for rule in document["rules"])
    return cited


def is_covered(rule, cited):
    """Whether `rule`, or one of its items `rule.n`, is cited."""
    return rule in cited or any(citation.startswith(rule + ".") for citation in cited)


def uncovered(rules, case_citations, process_citations):
    """The uncovered rule IDs, as `(kind, rule)` pairs in index order."""
    findings = []
    for rule in rules:
        kind = classify(rule)
        if kind == "behavioral" and not is_covered(rule, case_citations):
            findings.append((kind, rule))
        elif kind == "process" and not is_covered(rule, process_citations):
            findings.append((kind, rule))
        elif kind == "unclassified":
            findings.append((kind, rule))
    return findings


def workspace_version(cargo_text):
    """The `[workspace.package]` version of the root `Cargo.toml`, as a tuple of integers."""
    version = tomllib.loads(cargo_text)["workspace"]["package"]["version"]
    return tuple(int(part) for part in version.split("."))


def read_texts(paths):
    """The UTF-8 text of each path, sorted by path."""
    return [path.read_text(encoding="utf-8") for path in sorted(paths)]


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    rules = index_rules((root / "spec" / "INDEX.md").read_text(encoding="utf-8"))
    case_citations = cited_in_cases(read_texts((root / "conformance" / "cases").glob("*/*.yaml")))
    process_sources = list((root / ".github" / "workflows").glob("*.yml"))
    process_sources.append(root / ".claude" / "skills" / "review-pr" / "SKILL.md")
    process_citations = set()
    for text in read_texts(path for path in process_sources if path.is_file()):
        process_citations |= cited_in_text(text)
    findings = uncovered(rules, case_citations, process_citations)
    for kind, rule in findings:
        what = "conformance case" if kind == "behavioral" else "CI check or review item"
        if kind == "unclassified":
            print(f"{rule}: prefix is neither behavioral nor process")
        else:
            print(f"{rule}: {kind} rule without a {what}")
    version = workspace_version((root / "Cargo.toml").read_text(encoding="utf-8"))
    failing = version >= FAILING_FROM
    level = "error" if failing and findings else "notice"
    print(f"::{level} title=rule-coverage::{len(findings)} of {len(rules)} rule IDs uncovered")
    sys.exit(1 if failing and findings else 0)


if __name__ == "__main__":
    main()
