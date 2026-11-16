"""Check that a pull request branch changes only files its prefix allows (WF-6.3).

A `spec/<topic>` branch may change only files inside the specification scope;
an `issue-<number>-<topic>` branch may change only files outside it; any other
branch name fails. The scope is read from `.github/specification-scope.txt`.

Usage: python3 scripts/ci/check_scope.py BRANCH BASE_SHA HEAD_SHA
Exit status: 0 when the check passes, 1 when it fails.
"""
import pathlib
import re
import subprocess
import sys

SCOPE_FILE = pathlib.Path(__file__).resolve().parent.parent.parent / ".github" / "specification-scope.txt"
SPECIFICATION_BRANCH = re.compile(r"^spec/[^\s]+$")
IMPLEMENTATION_BRANCH = re.compile(r"^issue-[0-9]+-[^\s]+$")


def read_scope(text):
    """Return the scope entries of a scope file's text, without comments and blank lines."""
    entries = []
    for line in text.splitlines():
        entry = line.strip()
        if entry and not entry.startswith("#"):
            entries.append(entry)
    return entries


def in_scope(path, scope):
    """Return True when the repository-relative path is covered by a scope entry."""
    for entry in scope:
        if entry.endswith("/"):
            if path.startswith(entry):
                return True
        elif path == entry:
            return True
    return False


def find_violations(branch, changed_paths, scope):
    """Return the list of error messages for a branch and its changed paths; empty means pass."""
    if SPECIFICATION_BRANCH.match(branch):
        return [
            f"{path}: outside the specification scope, not allowed on a spec/ branch"
            for path in changed_paths
            if not in_scope(path, scope)
        ]
    if IMPLEMENTATION_BRANCH.match(branch):
        return [
            f"{path}: inside the specification scope, not allowed on an issue- branch"
            for path in changed_paths
            if in_scope(path, scope)
        ]
    return [
        f"branch '{branch}' is neither spec/<topic> nor issue-<number>-<topic>"
    ]


def changed_paths_between(base, head):
    """Return every path added, modified, deleted or renamed (both sides) since the merge base."""
    output = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", f"{base}...{head}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line for line in output.splitlines() if line]


def main(arguments):
    """Run the check and print one GitHub error annotation per violation."""
    if len(arguments) != 3:
        print(__doc__, file=sys.stderr)
        return 1
    branch, base, head = arguments
    scope = read_scope(SCOPE_FILE.read_text(encoding="utf-8"))
    violations = find_violations(branch, changed_paths_between(base, head), scope)
    for violation in violations:
        print(f"::error title=scope (WF-6.3)::{violation}")
    if not violations:
        print(f"scope: branch '{branch}' changes only allowed files")
    return 1 if violations else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
