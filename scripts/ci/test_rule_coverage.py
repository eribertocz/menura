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
| [b/c.md](b/c.md#L7) | 7 | rule | T1 |
| [b/c.md](b/c.md#L8) | 8 | rule | CS-4.2 |
| [b/c.md](b/c.md#L9) | 9 | rule | ZZ-1 |
"""


class RuleCoverageTest(unittest.TestCase):
    """T1, CS-2.6."""

    def test_t1_reads_rule_rows_of_the_index_only_with_their_file(self):
        self.assertEqual(
            check_rule_coverage.index_rules(INDEX),
            [
                ("spec/a.md", "E3-3.2"),
                ("spec/a.md", "CAT-7.2"),
                ("spec/b/c.md", "T1"),
                ("spec/b/c.md", "CS-4.2"),
                ("spec/b/c.md", "ZZ-1"),
            ],
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
        rules = [("spec/a.md", "E3-3.2"), ("spec/a.md", "CAT-7.2")]
        findings = check_rule_coverage.uncovered(rules, cases, {"E3-3.2"})
        self.assertEqual(findings, [("behavioral", "E3-3.2")])

    def test_t1_process_rule_is_covered_by_a_ci_check_or_review_item(self):
        process = check_rule_coverage.cited_in_text("run: just conformance  # T1–T4, CS-4.2")
        self.assertTrue({"T1", "T2", "T3", "T4", "CS-4.2"} <= process)
        rules = [("spec/a.md", "T1"), ("spec/a.md", "CS-4.2"), ("spec/a.md", "CS-4.3")]
        findings = check_rule_coverage.uncovered(rules, {"CS-4.3"}, process)
        self.assertEqual(findings, [("process", "CS-4.3")])

    def test_t1_unclassified_prefix_is_reported(self):
        findings = check_rule_coverage.uncovered([("spec/a.md", "ZZ-1")], set(), set())
        self.assertEqual(findings, [("unclassified", "ZZ-1")])

    def test_t1_item_citation_does_not_cover_a_sibling(self):
        self.assertFalse(check_rule_coverage.is_covered("CS-2.1", {"CS-2.10"}))

    def test_cs_2_6_reads_scope_entries_without_comments_or_blanks(self):
        text = "# Coverage scope.\n\nspec/a.md\n  spec/b/c.md  \n# spec/d.md\n"
        self.assertEqual(
            check_rule_coverage.scope_files(text), [("spec/a.md", None), ("spec/b/c.md", None)]
        )

    def test_cs_2_6_reads_rule_level_entries_separated_by_whitespace(self):
        text = "spec/b/c.md T1\tCS-4.2   ZZ-1\n"
        self.assertEqual(
            check_rule_coverage.scope_files(text), [("spec/b/c.md", ("T1", "CS-4.2", "ZZ-1"))]
        )

    def test_cs_2_6_whole_file_entry_puts_every_rule_of_the_file_in_scope(self):
        rules = check_rule_coverage.index_rules(INDEX)
        findings = check_rule_coverage.uncovered(rules, set(), set())
        blocking = check_rule_coverage.in_scope(findings, rules, [("spec/b/c.md", None)])
        self.assertEqual(
            blocking, [("process", "T1"), ("process", "CS-4.2"), ("unclassified", "ZZ-1")]
        )

    def test_cs_2_6_rule_level_entry_puts_only_its_rules_in_scope(self):
        rules = check_rule_coverage.index_rules(INDEX)
        findings = check_rule_coverage.uncovered(rules, set(), set())
        blocking = check_rule_coverage.in_scope(findings, rules, [("spec/b/c.md", ("CS-4.2",))])
        self.assertEqual(blocking, [("process", "CS-4.2")])

    def test_cs_2_6_scope_rule_ids_count_whole_file_and_rule_level_entries(self):
        rules = check_rule_coverage.index_rules(INDEX)
        scope = [("spec/a.md", None), ("spec/b/c.md", ("T1",)), ("spec/b/c.md", ("T1", "ZZ-1"))]
        self.assertEqual(
            check_rule_coverage.scope_rules(scope, rules), {"E3-3.2", "CAT-7.2", "T1", "ZZ-1"}
        )

    def test_cs_2_6_empty_scope_blocks_nothing(self):
        rules = check_rule_coverage.index_rules(INDEX)
        findings = check_rule_coverage.uncovered(rules, set(), set())
        self.assertEqual(check_rule_coverage.in_scope(findings, rules, []), [])

    def test_cs_2_6_scope_entry_without_index_file_is_reported(self):
        rules = check_rule_coverage.index_rules(INDEX)
        scope = [("spec/a.md", None), ("a.md", None), ("spec/x.md", ("T1",))]
        unknown = check_rule_coverage.unknown_scope_files(scope, rules)
        self.assertEqual(unknown, ["a.md", "spec/x.md"])
        self.assertEqual(check_rule_coverage.unknown_scope_rules(scope, rules), [])

    def test_cs_2_6_rule_level_entry_with_unknown_rule_id_is_reported(self):
        rules = check_rule_coverage.index_rules(INDEX)
        scope = [("spec/b/c.md", ("T1", "T9"))]
        unknown = check_rule_coverage.unknown_scope_rules(scope, rules)
        self.assertEqual(unknown, [("spec/b/c.md", "T9")])

    def test_cs_2_6_rule_id_defined_in_another_file_is_reported(self):
        rules = check_rule_coverage.index_rules(INDEX)
        scope = [("spec/b/c.md", ("E3-3.2",))]
        unknown = check_rule_coverage.unknown_scope_rules(scope, rules)
        self.assertEqual(unknown, [("spec/b/c.md", "E3-3.2")])
        findings = check_rule_coverage.uncovered(rules, set(), set())
        self.assertEqual(check_rule_coverage.in_scope(findings, rules, scope), [])

    def test_cs_2_6_unknown_rule_id_is_not_in_scope(self):
        rules = check_rule_coverage.index_rules(INDEX)
        scope = [("spec/x.md", ("T1",))]
        self.assertEqual(check_rule_coverage.scope_rules(scope, rules), set())

    def test_cs_2_1_suffixed_citation_covers_its_rule(self):
        cases = check_rule_coverage.cited_in_cases(["id: a\nrules: [E4-6.9a]\n"])
        self.assertTrue(check_rule_coverage.is_covered("E4-6.9a", cases))
        rules = [("spec/a.md", "E4-6.9a"), ("spec/a.md", "E4-6.9b")]
        findings = check_rule_coverage.uncovered(rules, cases, set())
        self.assertEqual(findings, [("behavioral", "E4-6.9b")])

    def test_cs_2_1_suffixed_citation_covers_its_parent_section_but_not_its_sibling(self):
        self.assertTrue(check_rule_coverage.is_covered("E4-6", {"E4-6.9a"}))
        self.assertFalse(check_rule_coverage.is_covered("E4-6.9", {"E4-6.9a"}))
        self.assertFalse(check_rule_coverage.is_covered("E4-6.9b", {"E4-6.9a"}))

    def test_cs_2_1_item_citation_does_not_cover_a_suffixed_rule(self):
        self.assertTrue(check_rule_coverage.is_covered("E4-6", {"E4-6.9"}))
        self.assertFalse(check_rule_coverage.is_covered("E4-6.9a", {"E4-6.9"}))

    def test_t1_reads_suffixed_rule_ids_in_text(self):
        cited = check_rule_coverage.cited_in_text("checks CS-2.4a, SEC-3.2a and T2b; PLN-5.1")
        self.assertEqual(cited, {"CS-2.4a", "SEC-3.2a", "T2b", "PLN-5.1"})

    def test_t1_reads_suffixed_rule_inside_a_range(self):
        self.assertEqual(check_rule_coverage.cited_in_text("T1a–T1c"), {"T1a", "T1b", "T1c"})
        self.assertEqual(check_rule_coverage.cited_in_text("T1–T3a"), {"T1", "T2", "T3a"})

    def test_t1_classifies_vs_rules_as_behavioral(self):
        self.assertEqual(check_rule_coverage.classify("VS-1"), "behavioral")

if __name__ == "__main__":
    unittest.main()
