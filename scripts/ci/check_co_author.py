"""Check that every commit of a pull request has a Co-Authored-By trailer (WF-6.4).

Usage: python3 scripts/ci/check_co_author.py BASE_SHA HEAD_SHA
Exit status: 0 when every commit has the trailer, 1 otherwise.
"""
import re
import subprocess
import sys

TRAILER = re.compile(r"^co-authored-by:\s*\S", re.IGNORECASE)


def has_co_author(trailers):
    """Return True when the commit's parsed trailer block contains a Co-Authored-By line."""
    return any(TRAILER.match(line) for line in trailers.splitlines())


def git(*arguments):
    """Run a git command and return its standard output."""
    return subprocess.run(["git", *arguments], check=True, capture_output=True, text=True).stdout


def commits_between(base, head):
    """Return the commits reachable from head and not from base, oldest first."""
    return git("rev-list", "--reverse", f"{base}..{head}").split()


def commit_trailers(commit):
    """Return the trailer block of a commit message, as git parses it."""
    return git("log", "-1", "--format=%(trailers:only,unfold)", commit)


def commit_subject(commit):
    """Return the subject line of a commit."""
    return git("log", "-1", "--format=%s", commit).strip()


def main(arguments):
    """Run the check and print one GitHub error annotation per commit without the trailer."""
    if len(arguments) != 2:
        print(__doc__, file=sys.stderr)
        return 1
    base, head = arguments
    missing = [commit for commit in commits_between(base, head) if not has_co_author(commit_trailers(commit))]
    for commit in missing:
        print(f"::error title=co-author (WF-6.4)::commit {commit[:12]} '{commit_subject(commit)}' has no Co-Authored-By trailer")
    if not missing:
        print("co-author: every commit has a Co-Authored-By trailer")
    return 1 if missing else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
