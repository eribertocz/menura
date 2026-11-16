"""Warn about non-English pull request titles, bodies and commit subjects (WF-9.3).

Flags text containing characters outside ASCII letters, digits, punctuation and
whitespace, or common non-English stop words. Code spans, fenced code blocks and
URLs are not scanned for stop words. The check only warns: it always exits 0.

Reads the pull request from the event file at GITHUB_EVENT_PATH.

Usage: python3 scripts/ci/check_language.py BASE_SHA HEAD_SHA
"""
import json
import os
import re
import string
import subprocess
import sys

ALLOWED_CHARACTERS = frozenset(string.ascii_letters + string.digits + string.punctuation + string.whitespace)
# Frequent Spanish, Portuguese, French, German and Italian function words that are not English words.
STOP_WORDS = frozenset(
    """
    el los las del una unos unas que para por pero como esta este estos estas tambien
    sino porque cuando donde muy sobre entre hasta desde segun
    nao uma umas pelo pela pelos pelas seu sua mais isso
    les une avec pour dans sur ces cette qui sont aussi mais
    und der das ist nicht ein eine einen auf sich dem auch oder wird
    il della delle degli che sono gli questo questa anche
    """.split()
)
CODE_OR_URL = re.compile(r"```.*?```|`[^`\n]*`|https?://\S+", re.DOTALL)
WORD = re.compile(r"[A-Za-z]+")


def language_findings(text):
    """Return human-readable findings for one text; empty when it looks English."""
    findings = []
    foreign_characters = sorted({character for character in text if character not in ALLOWED_CHARACTERS})
    if foreign_characters:
        findings.append("characters outside ASCII letters, digits and punctuation: " + " ".join(foreign_characters))
    words = {word.lower() for word in WORD.findall(CODE_OR_URL.sub(" ", text))}
    stop_words = sorted(words & STOP_WORDS)
    if stop_words:
        findings.append("non-English stop words: " + ", ".join(stop_words))
    return findings


def commit_subjects(base, head):
    """Return the subject lines of the commits reachable from head and not from base."""
    output = subprocess.run(
        ["git", "log", "--reverse", "--format=%h %s", f"{base}..{head}"],
        check=True,
        capture_output=True,
        text=True,
    ).stdout
    return [line.partition(" ")[::2] for line in output.splitlines() if line]


def texts_to_check(pull_request, base, head):
    """Return (label, text) pairs: the title, the body and every commit subject."""
    texts = [("title", pull_request.get("title") or ""), ("body", pull_request.get("body") or "")]
    for short_sha, subject in commit_subjects(base, head):
        texts.append((f"commit {short_sha}", subject))
    return texts


def main(arguments):
    """Print one GitHub warning annotation per finding; always succeed."""
    if len(arguments) != 2:
        print(__doc__, file=sys.stderr)
        return 0
    base, head = arguments
    with open(os.environ["GITHUB_EVENT_PATH"], encoding="utf-8") as event_file:
        pull_request = json.load(event_file)["pull_request"]
    warnings = 0
    for label, text in texts_to_check(pull_request, base, head):
        for finding in language_findings(text):
            print(f"::warning title=language (WF-9.3)::{label}: {finding}")
            warnings += 1
    print(f"language: {warnings} warning(s); this check never fails")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
