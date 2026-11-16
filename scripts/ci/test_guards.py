"""Unit tests of the workflow guard checks. Run: python3 -m unittest discover scripts/ci"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import check_co_author  # noqa: E402
import check_issue_label  # noqa: E402
import check_language  # noqa: E402
import check_scope  # noqa: E402

SCOPE = check_scope.read_scope(check_scope.SCOPE_FILE.read_text(encoding="utf-8"))


class ScopeTest(unittest.TestCase):
    """WF-6.3."""

    def test_wf_6_3_scope_file_lists_section_6_6(self):
        self.assertEqual(
            SCOPE,
            ["spec/", "AGENTS.md", "CLAUDE.md", ".claude/", ".github/ISSUE_TEMPLATE/",
             ".github/PULL_REQUEST_TEMPLATE.md", "scripts/build_index.py", "scripts/check_examples.py"],
        )

    def test_wf_6_3_spec_branch_inside_scope_passes(self):
        self.assertEqual(check_scope.find_violations("spec/topic", ["spec/INDEX.md", ".claude/settings.json"], SCOPE), [])

    def test_wf_6_3_spec_branch_outside_scope_fails(self):
        self.assertEqual(len(check_scope.find_violations("spec/topic", ["spec/a.md", "README.md"], SCOPE)), 1)

    def test_wf_6_3_issue_branch_outside_scope_passes(self):
        self.assertEqual(check_scope.find_violations("issue-4-guards", [".github/workflows/x.yml", "scripts/ci/a.py"], SCOPE), [])

    def test_wf_6_3_issue_branch_inside_scope_fails(self):
        self.assertEqual(len(check_scope.find_violations("issue-4-guards", ["AGENTS.md", "scripts/check_examples.py"], SCOPE)), 2)

    def test_wf_6_3_other_branch_name_fails(self):
        for branch in ("main", "feature/x", "issue-x-name", "issue-4", "spec/", "claude/topic"):
            with self.subTest(branch=branch):
                self.assertEqual(len(check_scope.find_violations(branch, [], SCOPE)), 1)

    def test_wf_6_3_directory_entry_is_a_prefix_only(self):
        self.assertFalse(check_scope.in_scope("specification/a.md", SCOPE))
        self.assertFalse(check_scope.in_scope("AGENTS.md.bak", SCOPE))


class CoAuthorTest(unittest.TestCase):
    """WF-6.4."""

    def test_wf_6_4_trailer_present(self):
        self.assertTrue(check_co_author.has_co_author("Co-Authored-By: Claude <noreply@anthropic.com>\n"))
        self.assertTrue(check_co_author.has_co_author("co-authored-by: Claude <noreply@anthropic.com>\n"))

    def test_wf_6_4_trailer_missing(self):
        self.assertFalse(check_co_author.has_co_author(""))
        self.assertFalse(check_co_author.has_co_author("Signed-off-by: Someone <a@b.c>\n"))
        self.assertFalse(check_co_author.has_co_author("Co-Authored-By:\n"))


class IssueLabelTest(unittest.TestCase):
    """WF-6.5."""

    def test_wf_6_5_closing_references(self):
        body = "Closes #4\nfixes: #7, resolves owner/repo#9 and other/repo#11; see #12"
        self.assertEqual(check_issue_label.closed_issue_numbers(body, "owner/repo"), [4, 7, 9])
        self.assertEqual(check_issue_label.closed_issue_numbers(None, "owner/repo"), [])
        self.assertEqual(check_issue_label.closed_issue_numbers("Closes #", "owner/repo"), [])

    def test_wf_6_5_exactly_one_label_passes(self):
        issue = {"labels": [{"name": "impl"}, {"name": "spec-defect"}]}
        self.assertIsNone(check_issue_label.label_error(4, issue))

    def test_wf_6_5_no_or_both_labels_fail(self):
        self.assertIsNotNone(check_issue_label.label_error(4, {"labels": [{"name": "study"}]}))
        self.assertIsNotNone(check_issue_label.label_error(4, {"labels": [{"name": "spec"}, {"name": "impl"}]}))

    def test_wf_6_5_pull_request_reference_fails(self):
        self.assertIsNotNone(check_issue_label.label_error(4, {"pull_request": {}, "labels": [{"name": "impl"}]}))


class LanguageTest(unittest.TestCase):
    """WF-9.3."""

    def test_wf_9_3_english_text_has_no_findings(self):
        self.assertEqual(check_language.language_findings("feat(ci): add workflow guards (#4)\n\n- [x] Done, 100 %"), [])

    def test_wf_9_3_non_ascii_characters_are_flagged(self):
        self.assertEqual(len(check_language.language_findings("fix: corrección")), 1)

    def test_wf_9_3_stop_words_are_flagged(self):
        findings = check_language.language_findings("agregar la regla para el compilador")
        self.assertEqual(findings, ["non-English stop words: el, para"])

    def test_wf_9_3_code_and_urls_are_not_scanned_for_stop_words(self):
        self.assertEqual(check_language.language_findings("see `el para` and https://example.org/el/para"), [])


if __name__ == "__main__":
    unittest.main()
