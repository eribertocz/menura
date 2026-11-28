"""Rule coverage check (spec/09-continuous-integration.md CI-1 `rule-coverage`, rule T1).

Reads the rule IDs of `spec/INDEX.md` and lists every behavioral rule ID that no conformance
case cites in its `rules`, and every process rule ID that no CI workflow and no item of the
`review-pr` skill cites (spec/README.md Conventions). A citation of an item `X.n` also covers
rule `X`, but never a rule with a letter suffix such as `X.na`. A citation of `X.na` covers
`X.na` and, like `X.n`, rule `X`, but never its sibling `X.n`. Rule IDs whose prefix is in neither list are reported as unclassified.

The check fails only for uncovered rule IDs defined in a file of the coverage scope,
`spec/coverage-scope.txt` (CS-2.6); every other finding is reported only. A scope entry is a path
from the repository root (`spec/08-conformance-suite.md`), which puts every rule ID of that file in
scope, or a path followed by rule IDs separated by whitespace, which puts only those rules in scope.
An entry that names no file of `spec/INDEX.md`, or a rule ID that the index does not define in that
file, fails the check, so a mistyped entry can never hide a rule.

A behavioral rule is also covered by a test function whose name starts with the rule ID in lower
case, `-` and `.` written `_` (CS-2.6a): a Rust `fn` under `#[test]`, or a Python `def test_…` in a
`test_*.py` file, read after its `test_` prefix. A process rule is never covered by a test. A rule
listed in `spec/coverage-exempt.txt` with a reason is covered (CS-2.6b); a listed ID that the index
does not define, or a line without a reason, fails the check.

Usage: python3 scripts/ci/check_rule_coverage.py [repository-root]
Requires PyYAML.
"""
import pathlib
import re
import sys

import yaml

# Prefixes of spec/README.md Conventions. Backlog items (`F`) are not rules.
BEHAVIORAL = re.compile(
    r"^(?:E[0-6][A-Z]?-|(?:STG|CAT|QRY|SEC|PLN|DSC|CHK|MDL|CON|ADD|POP|DER|CTX|SCY|SUM|EXT|EXE|DEV|VS|PLT|VAL)-"
    r"|OSS-[25](?:\.|$)|ARC-[56](?:\.|$)|(?:MIG|DEP)-)"
)
PROCESS = re.compile(r"^(?:[PASQTR]\d|(?:WF|CS|CI)-|OSS-[134](?:\.|$))")
BACKLOG = re.compile(r"^F\d")

INDEX_RULE = re.compile(r"^\|\s*\[([^\]]+)\]\([^)]*\)\s*\|[^|]*\|\s*rule\s*\|\s*([^|\s]+)\s*\|\s*$")
INDEX_ROOT = "spec/"
RULE_TOKEN = re.compile(
    r"\b(?:[A-Z]{1,4}\d+(?:\.\d+)*[a-z]?|[A-Z]{1,4}\d?[A-Z]?-\d+(?:\.\d+)*[a-z]?)\b"
)
RANGE_TOKEN = re.compile(r"\b([A-Z]{1,4})(\d+)([a-z]?)\s*[–-]\s*(?:\1)?(\d+)([a-z]?)\b")
SUFFIX = re.compile(r"(?<=\d)[a-z]$")

# CS-2.6a: the test name of a rule, e.g. `mig_3_2a_reads_x` or `e0a_1_2_reads_x`.
TEST_NAME_RULE = re.compile(r"^([a-z]+\d?[a-z]?)((?:_\d+)+)([a-z]?)(?=_|$)")
RUST_TEST = re.compile(
    r"#\[test\](?:\s*#\[[^\]]*\])*\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)"
)
PYTHON_TEST = re.compile(r"^\s*(?:async\s+)?def\s+(\w+)", re.MULTILINE)
PYTHON_TEST_PREFIX = "test_"
SKIPPED_DIRECTORIES = {"target", "node_modules", "__pycache__"}


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
    for match in RANGE_TOKEN.findall(text):
        cited.update(expand_range(*match))
    return cited


def expand_range(prefix, first, first_suffix, last, last_suffix):
    """The rule IDs of a range such as `T1–T4`, `T1a–T1c` or `T1–T3a`.

    A range within one number runs over its letter suffixes. Otherwise it runs over the numbers,
    and each endpoint keeps its own suffix.
    """
    if first == last and first_suffix and last_suffix:
        letters = range(ord(first_suffix), ord(last_suffix) + 1)
        return {f"{prefix}{first}{chr(letter)}" for letter in letters}
    numbers = range(int(first), int(last) + 1)
    expanded = {f"{prefix}{number}" for number in numbers[1:-1]}
    expanded.add(f"{prefix}{first}{first_suffix}")
    expanded.add(f"{prefix}{last}{last_suffix}")
    return expanded


def cited_in_cases(case_texts):
    """Rule IDs listed under `rules` by the given case files."""
    cited = set()
    for text in case_texts:
        document = yaml.safe_load(text)
        if isinstance(document, dict) and isinstance(document.get("rules"), list):
            cited.update(str(rule) for rule in document["rules"])
    return cited


def is_covered(rule, cited):
    """Whether `rule`, or one of its items `rule.n` or `rule.na`, is cited.

    The letter suffix of a citation is dropped before matching items, so `X.na` covers `X` but
    never `X.n`.
    """
    return rule in cited or any(
        SUFFIX.sub("", citation).startswith(rule + ".") for citation in cited
    )


def rust_test_names(text):
    """Names of the `fn` items under `#[test]` in Rust source text."""
    return RUST_TEST.findall(text)


def python_test_names(text):
    """Names of the `test_` functions of a Python test file, without that prefix.

    A function whose name does not start with `test_` is not a test and is never returned.
    """
    return [
        name[len(PYTHON_TEST_PREFIX):]
        for name in PYTHON_TEST.findall(text)
        if name.startswith(PYTHON_TEST_PREFIX)
    ]


def rule_of_test_name(name):
    """The rule ID a test name starts with (CS-2.6a), or `None` when it starts with none.

    `arc_5_7_reads_x` gives `ARC-5.7`, `mig_3_2a_reads_x` gives `MIG-3.2a` and `e0a_1_2_reads_x`
    gives `E0A-1.2`. Whether the rule exists and is behavioral is decided by the caller.
    """
    match = TEST_NAME_RULE.match(name)
    if not match:
        return None
    head, numbers, suffix = match.groups()
    return f"{head.upper()}-{'.'.join(numbers.split('_')[1:])}{suffix}"


def cited_in_tests(test_names):
    """Rule IDs that the given test names start with (CS-2.6a)."""
    return {rule for rule in map(rule_of_test_name, test_names) if rule}


def exempt_rules(exempt_text):
    """The entries of `spec/coverage-exempt.txt` as `(rule, reason)` pairs, in file order.

    One entry per line; `#` lines and blanks are ignored. `reason` is empty when the line has none.
    """
    entries = []
    for line in exempt_text.splitlines():
        fields = line.split(maxsplit=1)
        if fields and not fields[0].startswith("#"):
            entries.append((fields[0], fields[1].strip() if len(fields) > 1 else ""))
    return entries


def invalid_exemptions(exempt, rules):
    """`(rule, problem)` pairs of exemptions the index does not define or that give no reason."""
    defined = {rule for _, rule in rules}
    problems = []
    for rule, reason in exempt:
        if rule not in defined:
            problems.append((rule, "spec/INDEX.md does not define it"))
        elif not reason:
            problems.append((rule, "the line gives no reason"))
    return problems


def coverage_counts(rules, test_citations, exempt):
    """How many rule IDs a test covers (CS-2.6a), and how many an exemption covers (CS-2.6b)."""
    by_test = sum(1 for _, rule in rules if is_covered_by_test(rule, test_citations))
    by_exemption = len({rule for _, rule in rules} & {rule for rule, _ in exempt})
    return by_test, by_exemption


def is_covered_by_test(rule, test_citations):
    """Whether a test name covers `rule`: only a behavioral rule (CS-2.6a)."""
    return classify(rule) == "behavioral" and is_covered(rule, test_citations)


def uncovered(rules, case_citations, process_citations, test_citations=(), exempt=()):
    """The uncovered rule IDs of `(file, rule)` pairs, as `(kind, rule)` pairs in index order.

    A rule listed in `exempt` is covered (CS-2.6b); a behavioral rule is also covered by
    `test_citations` (CS-2.6a); a process rule never is.
    """
    test_citations = set(test_citations)
    exempted = {rule for rule, _ in exempt}
    findings = []
    for _, rule in rules:
        kind = classify(rule)
        if rule in exempted:
            continue
        if kind == "behavioral" and not (
            is_covered(rule, case_citations) or is_covered_by_test(rule, test_citations)
        ):
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


def is_skipped(directory):
    """Whether a directory holds build output or is hidden."""
    return directory in SKIPPED_DIRECTORIES or directory.startswith(".")


def repository_files(root, pattern):
    """Files of the repository matching `pattern`, outside build output and hidden directories."""
    return [
        path
        for path in root.rglob(pattern)
        if not any(is_skipped(part) for part in path.relative_to(root).parts[:-1])
    ]


def test_citations(root):
    """Rule IDs that the Rust and Python test functions of the repository start with (CS-2.6a)."""
    names = []
    for text in read_texts(repository_files(root, "*.rs")):
        names.extend(rust_test_names(text))
    for text in read_texts(repository_files(root, "test_*.py")):
        names.extend(python_test_names(text))
    return cited_in_tests(names)


def main():
    root = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else ".")
    rules = index_rules((root / "spec" / "INDEX.md").read_text(encoding="utf-8"))
    scope = scope_files((root / "spec" / "coverage-scope.txt").read_text(encoding="utf-8"))
    exempt_path = root / "spec" / "coverage-exempt.txt"
    exempt = exempt_rules(exempt_path.read_text(encoding="utf-8")) if exempt_path.is_file() else []
    tests = test_citations(root)
    findings = uncovered(rules, *citations(root), tests, exempt)
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
    problems = invalid_exemptions(exempt, rules)
    for rule, problem in problems:
        print(f"::error title=rule-coverage::CS-2.6b: exemption `{rule}`: {problem}")
    by_test, by_exemption = coverage_counts(rules, tests, exempt)
    print(
        f"::notice title=rule-coverage::{by_test} rule IDs covered by tests (CS-2.6a), "
        f"{by_exemption} by exemption (CS-2.6b)"
    )
    level = "error" if blocking else "notice"
    print(
        f"::{level} title=rule-coverage::{len(findings)} of {len(rules)} rule IDs uncovered; "
        f"{len(blocking)} in the coverage scope ({len(scope)} entries, "
        f"{len(scope_rules(scope, rules))} rule IDs)"
    )
    sys.exit(1 if blocking or unknown_files or unknown_rules or problems else 0)


if __name__ == "__main__":
    main()
