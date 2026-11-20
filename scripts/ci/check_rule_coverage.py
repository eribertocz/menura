"""Rule coverage check (spec/09-continuous-integration.md CI-1 `rule-coverage`, rule T1).

Reads the rule IDs of `spec/INDEX.md` and lists every behavioral rule ID that no conformance
case cites in its `rules`, and every process rule ID that no CI workflow and no item of the
`review-pr` skill cites (spec/README.md Conventions). A citation of an item `X.n` also covers
rule `X`. Rule IDs whose prefix is in neither list are reported as unclassified.

The check fails only for uncovered rule IDs defined in a file of the coverage scope,
`spec/coverage-scope.txt` (CS-2.6); every other finding is reported only. A scope entry is a path
from the repository root (`spec/08-conformance-suite.md`), which puts every rule ID of that file in
scope, or a path followed by rule IDs separated by whitespace, which puts only those rules in scope.
An entry that names no file of `spec/INDEX.md`, or a rule ID that the index does not define in that
file, fails the check, so a mistyped entry can never hide a rule.

Usage: python3 scripts/ci/check_rule_coverage.py [repository-root]
Requires PyYAML.
"""
import pathlib
import re
import sys

import yaml

# Prefixes of spec/README.md Conventions. Backlog items (`F`) are not rules.
BEHAVIORAL = re.compile(
    r"^(?:E[0-6][A-Z]?-|(?:STG|CAT|QRY|SEC|PLN|DSC|CHK|MDL|CON|ADD|POP|DER|CTX|SCY|SUM|EXT|DEV)-"
    r"|OSS-[25](?:\.|$)|ARC-5(?:\.|$))"
)
PROCESS = re.compile(r"^(?:[PASQTR]\d|(?:WF|CS|CI)-|OSS-[134](?:\.|$))")
BACKLOG = re.compile(r"^F\d")

INDEX_RULE = re.compile(r"^\|\s*\[([^\]]+)\]\([^)]*\)\s*\|[^|]*\|\s*rule\s*\|\s*([^|\s]+)\s*\|\s*$")
INDEX_ROOT = "spec/"
RULE_TOKEN = re.compile(r"\b(?:[A-Z]{1,4}\d+(?:\.\d+)*|[A-Z]{1,4}\d?[A-Z]?-\d+(?:\.\d+)*)\b")
RANGE_TOKEN = re.compile(r"\b([A-Z]{1,4})(\d+)\s*[–-]\s*(?:\1)?(\d+)\b")



def index_rules(index_text):
    """`(file, rule)` pairs of `spec/INDEX.md`, in file order, without repeated rule IDs.

    `file` is the path from the repository root of the file that defines the rule.
    """
    rules = []
    seen = set()
    for line in index_text.splitlines():
        match = INDEX_RULE.match(line)
        if match and match.group(2) not in seen:
            seen.add(match.group(2))
            rules.append((INDEX_ROOT + match.group(1), match.group(2)))
    return rules


def scope_files(scope_text):
    """The entries of `spec/coverage-scope.txt` as `(path, rule_ids)` pairs, in file order.

    One entry per line; `#` lines and blanks are ignored. `rule_ids` is `None` for a whole-file
    entry, and the tuple of listed rule IDs for a rule-level entry.
    """
    entries = []
    for line in scope_text.splitlines():
        fields = line.split()
        if fields and not fields[0].startswith("#"):
            entries.append((fields[0], tuple(fields[1:]) or None))
    return entries


def unknown_scope_files(scope, rules):
    """Paths of scope entries that name no file of the index, in scope order."""
    indexed = {file for file, _ in rules}
    return [path for path, _ in scope if path not in indexed]


def unknown_scope_rules(scope, rules):
    """`(path, rule)` pairs of rule-level entries whose rule the index does not define in `path`.

    Only entries whose path is a file of the index are checked; an unknown path is reported by
    `unknown_scope_files` alone.
    """
    defined = set(rules)
    indexed = {file for file, _ in rules}
    return [
        (path, rule)
        for path, rule_ids in scope
        if path in indexed
        for rule in rule_ids or ()
        if (path, rule) not in defined
    ]


def scope_rules(scope, rules):
    """The rule IDs in the coverage scope: every rule of whole-file entries, and listed rules."""
    whole_files = {path for path, rule_ids in scope if rule_ids is None}
    listed = {(path, rule) for path, rule_ids in scope for rule in rule_ids or ()}
    return {rule for file, rule in rules if file in whole_files or (file, rule) in listed}


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
    """The uncovered rule IDs of `(file, rule)` pairs, as `(kind, rule)` pairs in index order."""
    findings = []
    for _, rule in rules:
        kind = classify(rule)
        if kind == "behavioral" and not is_covered(rule, case_citations):
            findings.append((kind, rule))
        elif kind == "process" and not is_covered(rule, process_citations):
            findings.append((kind, rule))
        elif kind == "unclassified":
            findings.append((kind, rule))
    return findings


def in_scope(findings, rules, scope):
    """The findings whose rule is in the coverage scope."""
    selected = scope_rules(scope, rules)
    return [(kind, rule) for kind, rule in findings if rule in selected]


def read_texts(paths):
    """The UTF-8 text of each path, sorted by path."""
    return [path.read_text(encoding="utf-8") for path in sorted(paths)]


def report(findings, blocking):
    """Prints one line per finding, marking those that fail the check."""
    for kind, rule in findings:
        marker = "error" if (kind, rule) in blocking else "report"
        if kind == "unclassified":
            print(f"{marker}: {rule}: prefix is neither behavioral nor process")
        else:
            what = "conformance case" if kind == "behavioral" else "CI check or review item"
            print(f"{marker}: {rule}: {kind} rule without a {what}")


def citations(root):
    """Rule IDs cited by the conformance cases, and by the CI workflows and review items."""
    case_citations = cited_in_cases(read_texts((root / "conformance" / "cases").glob("*/*.yaml")))
    process_sources = list((root / ".github" / "workflows").glob("*.yml"))
    process_sources.append(root / ".claude" / "skills" / "review-pr" / "SKILL.md")
    process_citations = set()
    for text in read_texts(path for path in process_sources if path.is_file()):
        process_citations |= cited_in_text(text)
    return case_citations, process_citations


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    rules = index_rules((root / "spec" / "INDEX.md").read_text(encoding="utf-8"))
    scope = scope_files((root / "spec" / "coverage-scope.txt").read_text(encoding="utf-8"))
    findings = uncovered(rules, *citations(root))
    blocking = in_scope(findings, rules, scope)
    report(findings, set(blocking))
    unknown_files = unknown_scope_files(scope, rules)
    for path in unknown_files:
        print(f"::error title=rule-coverage::CS-2.6: scope entry `{path}` names no file of spec/INDEX.md")
    unknown_rules = unknown_scope_rules(scope, rules)
    for path, rule in unknown_rules:
        print(
            f"::error title=rule-coverage::CS-2.6: scope entry `{path}` lists `{rule}`, "
            "which spec/INDEX.md does not define in that file"
        )
    level = "error" if blocking else "notice"
    print(
        f"::{level} title=rule-coverage::{len(findings)} of {len(rules)} rule IDs uncovered; "
        f"{len(blocking)} in the coverage scope ({len(scope)} entries, "
        f"{len(scope_rules(scope, rules))} rule IDs)"
    )
    sys.exit(1 if blocking or unknown_files or unknown_rules else 0)


if __name__ == "__main__":
    main()
