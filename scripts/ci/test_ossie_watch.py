"""Unit tests of the Apache Ossie watch (OSS-4.1). Run: python3 -m unittest discover scripts/ci"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import ossie_watch  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]
TRACKED = "aa500d37894c3d62c96cdce4192cbecea4d7f06a"
UPSTREAM = {
    "repository": "https://github.com/apache/ossie",
    "tracked_commit": TRACKED,
    "recorded_release": "osi-0.1.1-rc1",
}


class OssieWatchTest(unittest.TestCase):
    """OSS-4.1."""

    def test_oss_4_1_reads_the_head_and_the_tags(self):
        listing = (
            "e4f545a\tHEAD\n"
            "bfacc23\trefs/tags/osi-0.1.1-rc1\n"
            "faf5810\trefs/tags/osi-0.1.1-rc1^{}\n"
            "1234567\trefs/tags/other\n"
        )
        self.assertEqual(ossie_watch.parse_ls_remote(listing), ("e4f545a", ["osi-0.1.1-rc1", "other"]))

    def test_oss_4_1_a_final_release_ranks_above_its_candidates(self):
        tags = ["osi-0.1.1-rc1", "osi-0.1.1", "osi-0.1.1-rc2", "osi-0.1.0", "other"]
        self.assertEqual(ossie_watch.latest_release(tags), "osi-0.1.1")

    def test_oss_4_1_versions_compare_as_numbers(self):
        self.assertEqual(ossie_watch.latest_release(["osi-0.9.0", "osi-0.10.0-rc1"]), "osi-0.10.0-rc1")

    def test_oss_4_1_no_change_opens_nothing(self):
        self.assertEqual(ossie_watch.changes(UPSTREAM, TRACKED, "osi-0.1.1-rc1"), [])

    def test_oss_4_1_a_development_change_opens_an_impl_issue_with_the_diff(self):
        [change] = ossie_watch.changes(UPSTREAM, "e4f545a4", "osi-0.1.1-rc1")
        self.assertEqual(change["labels"], ["ossie-watch", "impl"])
        self.assertIn(f"https://github.com/apache/ossie/compare/{TRACKED}...e4f545a4", change["body"])

    def test_oss_4_1_a_release_opens_a_spec_issue_with_the_diff(self):
        [change] = ossie_watch.changes(UPSTREAM, TRACKED, "osi-0.1.1")
        self.assertEqual(change["title"], "Apache Ossie release osi-0.1.1")
        self.assertEqual(change["labels"], ["ossie-watch", "spec"])
        self.assertIn("compare/osi-0.1.1-rc1...osi-0.1.1", change["body"])

    def test_oss_4_1_an_open_issue_is_not_opened_again(self):
        found = ossie_watch.changes(UPSTREAM, "e4f545a4", "osi-0.1.1")
        remaining = ossie_watch.new_changes(found, {"Apache Ossie release osi-0.1.1"})
        self.assertEqual([change["title"] for change in remaining], ["Apache Ossie development line moved past aa500d3"])

    def test_oss_4_1_a_manual_run_replaces_the_recorded_commit(self):
        upstream = ossie_watch.read_upstream(ROOT, {"RECORDED_COMMIT": "0" * 40, "RECORDED_RELEASE": ""})
        self.assertEqual(upstream["tracked_commit"], "0" * 40)
        self.assertEqual(upstream["recorded_release"], "osi-0.1.1-rc1")

    def test_oss_4_4_the_recorded_commit_is_the_vendored_one(self):
        upstream = ossie_watch.read_upstream(ROOT, {})
        self.assertEqual(upstream["tracked_commit"], TRACKED)


if __name__ == "__main__":
    unittest.main()
