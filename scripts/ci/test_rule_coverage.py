"""Unit tests of the rule coverage check. Run: python3 -m unittest discover -s scripts/ci -p 'test_rule_coverage.py'"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import check_rule_coverage  # noqa: E402

INDEX = """\
| File | Line | Kind | Entry |
|---|---|---|---|
| [a.md](a.md#L1) | 1 | section | 1. Title |
| [a.md](a.md#L5) | 5 | rule | E3-3.2 |
| [a.md](a.md#L6) | 6 | rule | CAT-7.2 |
| [a.md](a.md#L7) | 7 | rule | T1 |
| [a.md](a.md#L8) | 8 | rule | CS-4.2 |
| [a.md](a.md#L9) | 9 | rule | ZZ-1 |
"""


class RuleCoverageTest(unittest.TestCase):
    """T1."""

    def test_t1_reads_rule_rows_of_the_index_only(self):
        self.assertEqual(
            check_rule_coverage.index_rules(INDEX), ["E3-3.2", "CAT-7.2", "T1", "CS-4.2", "ZZ-1"]
        )

    def test_t1_classifies_by_the_prefixes_of_the_conventions(self):
        classify = check_rule_coverage.classify
        self.assertEqual(classify("E0A-2.1"), "behavioral")
        self.assertEqual(classify("OSS-2.4"), "behavioral")
        self.assertEqual(classify("ARC-5.1"), "behavioral")
        self.assertEqual(classify("DEV-3"), "behavioral")
        self.assertEqual(classify("OSS-4.6"), "process")
        self.assertEqual(classify("WF-6.3"), "process")
        self.assertEqual(classify("P10"), "process")
        self.assertEqual(classify("F28"), "backlog")
        self.assertEqual(classify("ZZ-1"), "unclassified")

    def test_t1_behavioral_rule_is_covered_by_a_case_only(self):
        cases = check_rule_coverage.cited_in_cases(["id: a\nrules: [CAT-7.2.4]\n"])
        findings = check_rule_coverage.uncovered(["E3-3.2", "CAT-7.2"], cases, {"E3-3.2"})
        self.assertEqual(findings, [("behavioral", "E3-3.2")])

    def test_t1_process_rule_is_covered_by_a_ci_check_or_review_item(self):
        process = check_rule_coverage.cited_in_text("run: just conformance  # T1–T4, CS-4.2")
        self.assertTrue({"T1", "T2", "T3", "T4", "CS-4.2"} <= process)
        findings = check_rule_coverage.uncovered(["T1", "CS-4.2", "CS-4.3"], {"CS-4.3"}, process)
        self.assertEqual(findings, [("process", "CS-4.3")])

    def test_t1_unclassified_prefix_is_reported(self):
        findings = check_rule_coverage.uncovered(["ZZ-1"], set(), set())
        self.assertEqual(findings, [("unclassified", "ZZ-1")])

    def test_t1_item_citation_does_not_cover_a_sibling(self):
        self.assertFalse(check_rule_coverage.is_covered("CS-2.1", {"CS-2.10"}))

    def test_t1_reads_the_workspace_version(self):
        text = '[workspace.package]\nversion = "0.1.0"\n'
        self.assertEqual(check_rule_coverage.workspace_version(text), (0, 1, 0))
        self.assertLess((0, 1, 0), check_rule_coverage.FAILING_FROM)
        self.assertGreaterEqual((0, 2, 0), check_rule_coverage.FAILING_FROM)


if __name__ == "__main__":
    unittest.main()
