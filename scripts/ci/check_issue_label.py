"""Check that the issue a pull request closes carries exactly one of `spec` or `impl` (WF-6.5).

The closed issues are the closing references in the pull request body (`Closes #12`,
`Fixes #12`, `Resolves owner/repo#12`, in any letter case). The check fails when the
body closes no issue, or when a closed issue carries neither label or both.

Reads the pull request from the event file at GITHUB_EVENT_PATH, the repository from
GITHUB_REPOSITORY and a read token from GITHUB_TOKEN.

Usage: python3 scripts/ci/check_issue_label.py
Exit status: 0 when the check passes, 1 when it fails.
"""
import json
import os
import re
import sys
import urllib.request

ASSIGNING_LABELS = ("spec", "impl")
CLOSING_REFERENCE = re.compile(
    r"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?)\s*:?\s+(?:([\w.-]+/[\w.-]+))?#([0-9]+)\b",
    re.IGNORECASE,
)


def closed_issue_numbers(body, repository):
    """Return the sorted, distinct issue numbers that the body closes in this repository."""
    numbers = set()
    for match in CLOSING_REFERENCE.finditer(body or ""):
        named_repository, number = match.groups()
        if named_repository is None or named_repository.lower() == repository.lower():
            numbers.add(int(number))
    return sorted(numbers)


def label_error(number, issue):
    """Return an error message for an issue's REST representation, or None when it passes."""
    if "pull_request" in issue:
        return f"#{number} is a pull request, not an issue"
    labels = {label["name"] for label in issue.get("labels", [])}
    assigned = [label for label in ASSIGNING_LABELS if label in labels]
    if len(assigned) != 1:
        found = ", ".join(assigned) if assigned else "none"
        return f"issue #{number} must carry exactly one of 'spec' or 'impl' (found: {found})"
    return None


def fetch_issue(repository, number, token):
    """Fetch one issue through the GitHub REST API."""
    request = urllib.request.Request(
        f"https://api.github.com/repos/{repository}/issues/{number}",
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def main():
    """Run the check and print one GitHub error annotation per failure."""
    repository = os.environ["GITHUB_REPOSITORY"]
    with open(os.environ["GITHUB_EVENT_PATH"], encoding="utf-8") as event_file:
        body = json.load(event_file)["pull_request"].get("body")
    numbers = closed_issue_numbers(body, repository)
    errors = [] if numbers else ["the pull request body closes no issue (expected 'Closes #<number>')"]
    for number in numbers:
        error = label_error(number, fetch_issue(repository, number, os.environ["GITHUB_TOKEN"]))
        if error:
            errors.append(error)
    for error in errors:
        print(f"::error title=issue-label (WF-6.5)::{error}")
    if not errors:
        print(f"issue-label: closed issues {numbers} each carry exactly one of 'spec' or 'impl'")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
