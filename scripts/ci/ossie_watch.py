"""Apache Ossie watch (spec/09-continuous-integration.md CI-2 `ossie-watch`, rule OSS-4.1).

Compares the latest Apache Ossie release and the head of its default branch with the versions
recorded in `conformance/ossie/upstream.json`, and opens an `ossie-watch` issue for each change,
with the link to the diff. An issue is not opened twice: a change whose issue is still open is
skipped.

- A release is a tag `osi-<major>.<minor>.<patch>` with an optional `-rc<n>` suffix; a final tag
  ranks above its release candidates. Its issue is labeled `spec`, because the mapping is updated
  first (OSS-4.5).
- A development change is a default-branch head other than the tracked commit. Its issue is
  labeled `impl`, because the tracked commit is updated in its own pull request (OSS-4.4).

Reads the upstream refs with `git ls-remote`, the repository from GITHUB_REPOSITORY and a token
that can write issues from GITHUB_TOKEN. RECORDED_COMMIT and RECORDED_RELEASE, when set and not
empty, replace the recorded values, so a manual run can prove that a change opens an issue.

Usage: python3 scripts/ci/ossie_watch.py [repository-root]
Exit status: 0 when the watch ran, 1 when it could not.
"""
import json
import os
import pathlib
import re
import subprocess
import sys
import urllib.request

UPSTREAM_FILE = pathlib.Path("conformance/ossie/upstream.json")
RELEASE_TAG = re.compile(r"^osi-(\d+)\.(\d+)\.(\d+)(?:-rc(\d+))?$")
WATCH_LABEL = "ossie-watch"
API = "https://api.github.com"


def parse_ls_remote(text):
    """`(head, tags)` from `git ls-remote` output: the commit of HEAD (or None) and every tag name."""
    head = None
    tags = set()
    for line in text.splitlines():
        parts = line.split("\t")
        if len(parts) != 2:
            continue
        commit, ref = parts
        if ref == "HEAD":
            head = commit
        elif ref.startswith("refs/tags/"):
            tags.add(ref[len("refs/tags/"):].removesuffix("^{}"))
    return head, sorted(tags)


def release_key(tag):
    """Sort key of a release tag, or None for a tag that is not a release."""
    match = RELEASE_TAG.match(tag)
    if match is None:
        return None
    major, minor, patch, candidate = match.groups()
    final = candidate is None
    return (int(major), int(minor), int(patch), final, int(candidate or 0))


def latest_release(tags):
    """The highest release tag, or None when there is none."""
    releases = [tag for tag in tags if release_key(tag) is not None]
    return max(releases, key=release_key, default=None)


def compare_link(repository_url, base, head):
    """The upstream diff between two refs."""
    return f"{repository_url}/compare/{base}...{head}"


def changes(upstream, head, release):
    """The issues to open: one dictionary with `title`, `body` and `labels` per change."""
    url = upstream["repository"]
    found = []
    recorded_release = upstream["recorded_release"]
    if release is not None and release != recorded_release:
        found.append({
            "title": f"Apache Ossie release {release}",
            "labels": [WATCH_LABEL, "spec"],
            "body": (
                f"Apache Ossie published the release `{release}`; the recorded release is "
                f"`{recorded_release}`.\n\n"
                f"Diff: {compare_link(url, recorded_release, release)}\n\n"
                "Handle it with the `ossie-update` skill (OSS-4.2, OSS-4.3: supported within 14 days)."
            ),
        })
    tracked = upstream["tracked_commit"]
    if head is not None and head != tracked:
        found.append({
            "title": f"Apache Ossie development line moved past {tracked[:7]}",
            "labels": [WATCH_LABEL, "impl"],
            "body": (
                f"The default branch of Apache Ossie is at `{head}`; the tracked commit is "
                f"`{tracked}`.\n\n"
                f"Diff: {compare_link(url, tracked, head)}\n\n"
                "Handle it with the `ossie-update` skill (OSS-4.4)."
            ),
        })
    return found


def new_changes(found, open_titles):
    """The changes whose title is not an open issue already."""
    return [change for change in found if change["title"] not in open_titles]


def read_upstream(root, environment):
    """The recorded versions, with the RECORDED_* overrides of a manual run."""
    upstream = json.loads((root / UPSTREAM_FILE).read_text(encoding="utf-8"))
    for variable, key in (("RECORDED_COMMIT", "tracked_commit"), ("RECORDED_RELEASE", "recorded_release")):
        if environment.get(variable):
            upstream[key] = environment[variable]
    return upstream


def github(method, path, token, payload=None):
    """One GitHub REST API call; returns the decoded JSON response."""
    data = None if payload is None else json.dumps(payload).encode("utf-8")
    request = urllib.request.Request(
        f"{API}{path}",
        data=data,
        method=method,
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {token}",
            "X-GitHub-Api-Version": "2022-11-28",
        },
    )
    with urllib.request.urlopen(request, timeout=30) as response:
        return json.load(response)


def main(arguments, environment):
    root = pathlib.Path(arguments[0]) if arguments else pathlib.Path(".")
    repository = environment.get("GITHUB_REPOSITORY")
    token = environment.get("GITHUB_TOKEN")
    if not repository or not token:
        print("GITHUB_REPOSITORY and GITHUB_TOKEN are required", file=sys.stderr)
        return 1
    upstream = read_upstream(root, environment)
    listing = subprocess.run(
        ["git", "ls-remote", upstream["repository"], "HEAD", "refs/tags/*"],
        check=True, capture_output=True, text=True, timeout=60,
    ).stdout
    head, tags = parse_ls_remote(listing)
    found = changes(upstream, head, latest_release(tags))
    issues = github("GET", f"/repos/{repository}/issues?labels={WATCH_LABEL}&state=open&per_page=100", token)
    open_titles = {issue["title"] for issue in issues if "pull_request" not in issue}
    for change in found:
        if change["title"] in open_titles:
            print(f"already open: {change['title']}")
    for change in new_changes(found, open_titles):
        issue = github("POST", f"/repos/{repository}/issues", token, change)
        print(f"opened #{issue['number']}: {change['title']} {issue['html_url']}")
    if not found:
        print(f"no change: release {upstream['recorded_release']}, commit {upstream['tracked_commit']}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:], os.environ))
