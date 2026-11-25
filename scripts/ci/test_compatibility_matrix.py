"""Unit tests of the compatibility matrix (CI-4). Run: python3 -m unittest discover scripts/ci"""
import io
import json
import pathlib
import sys
import tempfile
import unittest
import zipfile
from unittest import mock

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import compatibility_matrix as matrix  # noqa: E402

COMMIT = "0123456789abcdef0123456789abcdef01234567"
CI_URL = "https://github.com/owner/menura/actions/runs/11"
PYTHON_URL = "https://github.com/owner/menura/actions/runs/22"
LINUX = {"RUNNER_OS": "Linux", "RUNNER_ARCH": "X64"}
DESCRIPTION = json.dumps(
    {
        "capabilities": {
            "compiler_version": "1.0.0",
            "ossie": {"normative": "0.1.1", "tracked_commit": "aa500d37894c3d62c96cdce4192cbecea4d7f06a"},
        }
    }
)
CONFORMANCE_REPORT = """CS-1
  cs-001 passed
  cs-002 pending: needs E5
DSC-4
  cs-001 passed
2 cases: 1 passed, 1 pending, 0 failed
DuckDB v1.2.0
"""
OSSIE_REPORT = """OSS-2.3
  wide-001 passed
  wide-002 passed
  query-001 pending: not implemented, expected to pass in 1.1.0 (OSS-2.4): no
  query-002 pending: passes while listed in expected-failures.yaml; remove its entry (OSS-2.4)
4 cases: 2 passed, 2 pending, 0 failed
DuckDB v1.2.0
"""
CLICKHOUSE_RUN = json.dumps(
    {
        "job": "conformance-clickhouse",
        "dialect": "clickhouse",
        "clickhouse_version": "26.3.71.3",
        "clickhouse_time_zone": "America/La_Paz",
        "clickhouse_time_zone_data_version": "2025b",
        "cases": {"passed": 40, "total": 40},
    }
)


def archive(document):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as zipped:
        zipped.writestr("summary.json", json.dumps(document))
    return buffer.getvalue()


def ci_summaries():
    return {
        "conformance": matrix.conformance_summary(CONFORMANCE_REPORT, DESCRIPTION, LINUX),
        "ossie-suites": matrix.ossie_summary(OSSIE_REPORT, LINUX),
        "conformance-wrappers": matrix.wrappers_summary(LINUX),
        "conformance-clickhouse": matrix.clickhouse_summary(CLICKHOUSE_RUN, LINUX),
    }


def wheel_summaries(platforms):
    return {f"python-wheels-{platform}": {"job": "python-wheels", "platform": platform, "package_tests": "passed"}
            for platform in platforms}


class FakeActions:
    """Canned REST API responses for the runs of COMMIT."""

    def __init__(self, ci_jobs, wheel_jobs, ci_artifacts, wheel_artifacts):
        self.json = {}
        self.bytes = {}
        self.add_run("ci.yml", 11, CI_URL, ci_jobs, ci_artifacts)
        self.add_run("python.yml", 22, PYTHON_URL, wheel_jobs, wheel_artifacts)

    def add_run(self, workflow, identifier, url, jobs, artifacts):
        query = f"head_sha={COMMIT}&event=push&per_page=100"
        self.json[f"actions/workflows/{workflow}/runs?{query}"] = {
            "workflow_runs": [{"id": identifier, "head_sha": COMMIT, "html_url": url}]
        }
        self.json[f"actions/runs/{identifier}/jobs?filter=latest&per_page=100"] = {
            "jobs": [{"name": name, "conclusion": conclusion} for name, conclusion in jobs.items()]
        }
        listed = []
        for position, (name, document) in enumerate(sorted(artifacts.items())):
            download = f"https://api.example/artifacts/{identifier}/{position}"
            self.bytes[download] = archive(document)
            listed.append({"id": position, "name": f"ci-summary-{name}", "archive_download_url": download})
        self.json[f"actions/runs/{identifier}/artifacts?per_page=100"] = {"artifacts": listed}

    def get_json(self, path):
        return self.json[path]

    def get_bytes(self, url):
        return self.bytes[url]


def all_ci_jobs():
    return {name: "success" for name in matrix.CI_JOBS + (matrix.CLICKHOUSE_JOB,)}


def full_actions(platforms=matrix.WHEEL_PLATFORMS, ci_jobs=None, summaries=None):
    jobs = ci_jobs or all_ci_jobs()
    wheel_jobs = {f"python-wheels ({platform})": "success" for platform in platforms}
    return FakeActions(jobs, wheel_jobs, summaries or ci_summaries(), wheel_summaries(platforms))


def summaries_before_1_1_0():
    """The CI summaries of a tag commit that has no `conformance-clickhouse` job."""
    summaries = ci_summaries()
    del summaries[matrix.CLICKHOUSE_JOB]
    return summaries


# The row fields of the matrix asset as released in 1.0.0; later releases only add fields.
FIELDS_OF_1_0_0 = {
    "channel", "platform", "menura_version", "commit", "dialect", "duckdb_version", "ossie",
    "conformance", "ossie_suites", "runs",
}


class FakeRelease:
    def __init__(self):
        self.assets = {}

    def list_assets(self):
        return [{"id": index, "name": name} for index, name in enumerate(sorted(self.assets))]

    def delete_asset(self, identifier):
        name = sorted(self.assets)[identifier]
        del self.assets[name]

    def upload_asset(self, name, content):
        self.assets[name] = content


class SummaryTest(unittest.TestCase):
    """The summaries the CI jobs upload (CI-4.3)."""

    def test_ci_4_3_conformance_summary_counts_each_case_once(self):
        summary = matrix.conformance_summary(CONFORMANCE_REPORT, DESCRIPTION, LINUX)
        self.assertEqual(summary["cases"], {"passed": 1, "total": 2})
        self.assertEqual(summary["duckdb_version"], "v1.2.0")
        self.assertEqual(summary["platform"], "linux-x86_64")
        self.assertEqual(summary["ossie"]["normative"], "0.1.1")

    def test_ci_4_3_ossie_summary_separates_expected_failures_from_unexpected_results(self):
        summary = matrix.ossie_summary(OSSIE_REPORT, LINUX)
        self.assertEqual(summary["outcomes"], {"passed": 2, "expected_failures": 1, "unexpected": 1})

    def test_ci_4_5_a_report_without_the_duckdb_version_fails(self):
        with self.assertRaises(matrix.MatrixError):
            matrix.parse_report(CONFORMANCE_REPORT.replace("DuckDB v1.2.0\n", ""))

    def test_ci_4_5_a_description_without_the_capability_profile_fails(self):
        with self.assertRaises(matrix.MatrixError):
            matrix.conformance_summary(CONFORMANCE_REPORT, json.dumps({"capabilities": {}}), LINUX)

    def test_ci_4_3_the_profile_values_come_from_the_description_given(self):
        description = json.loads(DESCRIPTION)
        description["capabilities"]["compiler_version"] = "1.2.3"
        summary = matrix.conformance_summary(CONFORMANCE_REPORT, json.dumps(description), LINUX)
        self.assertEqual(summary["menura_version"], "1.2.3")

    def test_e6_7_16_clickhouse_summary_reads_both_versions_from_the_run(self):
        run = json.loads(CLICKHOUSE_RUN)
        run["clickhouse_version"] = "25.8.9.1"
        run["clickhouse_time_zone_data_version"] = "2024a"
        summary = matrix.clickhouse_summary(json.dumps(run), LINUX)
        self.assertEqual(summary["clickhouse_version"], "25.8.9.1")
        self.assertEqual(summary["clickhouse_time_zone_data_version"], "2024a")
        self.assertEqual(summary["platform"], "linux-x86_64")
        self.assertEqual(summary["cases"], {"passed": 40, "total": 40})

    def test_e6_7_16_a_clickhouse_run_without_its_time_zone_data_version_fails(self):
        for missing in ("clickhouse_version", "clickhouse_time_zone_data_version"):
            run = json.loads(CLICKHOUSE_RUN)
            run[missing] = ""
            with self.assertRaises(matrix.MatrixError, msg=missing):
                matrix.clickhouse_summary(json.dumps(run), LINUX)

    def test_e6_7_16_the_summarize_command_writes_the_clickhouse_summary(self):
        with tempfile.TemporaryDirectory() as directory:
            run = pathlib.Path(directory) / "run.json"
            run.write_text(CLICKHOUSE_RUN, encoding="utf-8")
            output = pathlib.Path(directory) / "summary" / "conformance-clickhouse.json"
            arguments = ["summarize", "conformance-clickhouse", str(run), "--output", str(output)]
            self.assertEqual(matrix.main(arguments, LINUX), 0)
            summary = json.loads(output.read_text(encoding="utf-8"))
        self.assertEqual(summary, matrix.clickhouse_summary(CLICKHOUSE_RUN, LINUX))

    def test_ci_4_5_a_clickhouse_run_with_malformed_case_counts_fails(self):
        for cases in (None, [], "40", {"passed": True, "total": 40}, {"passed": 40, "total": False},
                      {"passed": 41, "total": 40}, {"passed": -1, "total": 40}, {"total": 40}):
            run = json.loads(CLICKHOUSE_RUN)
            run["cases"] = cases
            with self.assertRaises(matrix.MatrixError, msg=repr(cases)):
                matrix.clickhouse_summary(json.dumps(run), LINUX)
        with self.assertRaises(matrix.MatrixError):
            matrix.clickhouse_summary("[]", LINUX)

    def test_ci_4_5_a_clickhouse_run_of_another_dialect_fails(self):
        run = json.loads(CLICKHOUSE_RUN)
        run["dialect"] = "duckdb"
        with self.assertRaises(matrix.MatrixError):
            matrix.clickhouse_summary(json.dumps(run), LINUX)

    def test_ci_4_5_a_wheel_summary_on_another_runner_fails(self):
        with self.assertRaises(matrix.MatrixError):
            matrix.wheel_summary("macos-arm64", LINUX)


class MatrixTest(unittest.TestCase):
    """CI-4.1, CI-4.2, CI-4.3, CI-4.5."""

    def test_ci_4_2_a_full_matrix_has_core_cli_mcp_and_one_python_row_per_wheel_platform(self):
        document = matrix.build_matrix(full_actions(), COMMIT, False)
        rows = [(row["channel"], f"{row['platform']['os']}-{row['platform']['arch']}") for row in document["rows"]]
        self.assertEqual(
            rows,
            [("core", "linux-x86_64"), ("cli", "linux-x86_64"), ("mcp", "linux-x86_64")]
            + [("python", platform) for platform in matrix.WHEEL_PLATFORMS],
        )

    def test_ci_4_3_every_row_carries_the_columns(self):
        document = matrix.build_matrix(full_actions(), COMMIT, False)
        core, cli = document["rows"][0], document["rows"][1]
        self.assertEqual(core["menura_version"], "1.0.0")
        self.assertEqual(core["commit"], COMMIT)
        self.assertEqual(core["dialect"], "duckdb")
        self.assertEqual(core["engine"], {"name": "duckdb", "version": "v1.2.0"})
        self.assertEqual(core["ossie"]["tracked_commit"], "aa500d37894c3d62c96cdce4192cbecea4d7f06a")
        self.assertEqual(core["conformance"], {"passed": 1, "total": 2})
        self.assertEqual(core["ossie_suites"], {"passed": 2, "expected_failures": 1, "unexpected": 1})
        self.assertNotIn("wrappers", core)
        self.assertEqual(core["runs"], [CI_URL])
        self.assertEqual(cli["wrappers"], {"check": "conformance-wrappers", "passed": True})
        python_linux, python_macos = document["rows"][3], document["rows"][5]
        self.assertEqual(python_linux["wrappers"]["check"], "conformance-wrappers")
        self.assertEqual(python_linux["runs"], [CI_URL, PYTHON_URL])
        self.assertEqual(python_macos["wrappers"]["check"], "package tests")
        self.assertEqual(python_macos["runs"], [PYTHON_URL])

    def test_ci_4_2_from_1_1_0_core_has_a_clickhouse_row_after_its_duckdb_row(self):
        document = matrix.build_matrix(full_actions(), COMMIT, True)
        rows = [(row["channel"], row["dialect"]) for row in document["rows"]]
        self.assertEqual(rows[:4], [("core", "duckdb"), ("core", "clickhouse"), ("cli", "duckdb"), ("mcp", "duckdb")])
        self.assertEqual([dialect for _, dialect in rows].count("clickhouse"), 1)
        self.assertTrue(all(dialect == "duckdb" for channel, dialect in rows if channel != "core"))

    def test_ci_4_3_the_clickhouse_row_carries_its_own_columns(self):
        clickhouse = matrix.build_matrix(full_actions(), COMMIT, True)["rows"][1]
        self.assertEqual(clickhouse["platform"], {"os": "linux", "arch": "x86_64"})
        self.assertEqual(clickhouse["menura_version"], "1.0.0")
        self.assertEqual(clickhouse["commit"], COMMIT)
        self.assertEqual(clickhouse["ossie"]["normative"], "0.1.1")
        self.assertEqual(clickhouse["conformance"], {"passed": 40, "total": 40})
        self.assertNotIn("wrappers", clickhouse)
        self.assertEqual(clickhouse["runs"], [CI_URL])

    def test_e6_7_16_the_clickhouse_row_gives_the_server_and_time_zone_data_versions(self):
        clickhouse = matrix.build_matrix(full_actions(), COMMIT, True)["rows"][1]
        self.assertEqual(
            clickhouse["engine"],
            {"name": "clickhouse", "version": "26.3.71.3", "time_zone_data_version": "2025b"},
        )
        markdown = matrix.render_markdown({"rows": [clickhouse]})
        self.assertIn("| ClickHouse 26.3.71.3, time-zone data 2025b |", markdown)

    def test_ci_4_3_the_clickhouse_row_has_null_ossie_suite_outcomes(self):
        document = matrix.build_matrix(full_actions(), COMMIT, True)
        self.assertIsNone(document["rows"][1]["ossie_suites"])
        self.assertIn('"ossie_suites": null', matrix.render_json(document))
        self.assertIsNotNone(document["rows"][0]["ossie_suites"])

    def test_ci_4_2_before_1_1_0_there_is_no_clickhouse_row_and_no_dependency_on_its_job(self):
        jobs = {name: "success" for name in matrix.CI_JOBS}
        actions = full_actions(ci_jobs=jobs, summaries=summaries_before_1_1_0())
        document = matrix.build_matrix(actions, COMMIT, False)
        self.assertNotIn("clickhouse", [row["dialect"] for row in document["rows"]])
        self.assertEqual(len(document["rows"]), 3 + len(matrix.WHEEL_PLATFORMS))

    def test_ci_4_3_the_duckdb_rows_keep_every_field_of_1_0_0_and_add_the_engine(self):
        document = matrix.build_matrix(full_actions(), COMMIT, True)
        for entry in document["rows"]:
            self.assertIn("engine", entry)
            if entry["dialect"] == "duckdb":
                self.assertLessEqual(FIELDS_OF_1_0_0, set(entry), entry["channel"])
                self.assertEqual(entry["duckdb_version"], "v1.2.0")
                self.assertEqual(entry["engine"], {"name": "duckdb", "version": entry["duckdb_version"]})
            else:
                self.assertNotIn("duckdb_version", entry)

    def test_ci_4_5_a_clickhouse_job_that_did_not_run_fails_from_1_1_0(self):
        jobs = {name: "success" for name in matrix.CI_JOBS}
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory) / "out"
            status = matrix.release(full_actions(ci_jobs=jobs), FakeRelease(), COMMIT, output, True)
            self.assertEqual(status, 1)
            self.assertFalse(output.exists())

    def test_ci_4_5_a_clickhouse_job_that_did_not_pass_fails_from_1_1_0(self):
        jobs = all_ci_jobs()
        jobs[matrix.CLICKHOUSE_JOB] = "failure"
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(full_actions(ci_jobs=jobs), COMMIT, True)

    def test_ci_4_5_a_missing_clickhouse_summary_fails_from_1_1_0(self):
        actions = full_actions()
        summaries = ci_summaries()
        del summaries["conformance-clickhouse"]
        actions.add_run("ci.yml", 11, CI_URL, all_ci_jobs(), summaries)
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT, True)

    def test_ci_4_2_a_wheel_platform_without_tests_has_no_row(self):
        platforms = [platform for platform in matrix.WHEEL_PLATFORMS if platform != "windows-x86_64"]
        document = matrix.build_matrix(full_actions(platforms), COMMIT, False)
        python = [row["platform"] for row in document["rows"] if row["channel"] == "python"]
        self.assertNotIn({"os": "windows", "arch": "x86_64"}, python)
        self.assertEqual(len(python), 4)

    def test_ci_4_5_a_missing_job_fails_without_writing_the_files(self):
        jobs = {"conformance": "success", "ossie-suites": "success"}
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory) / "out"
            status = matrix.release(full_actions(ci_jobs=jobs), FakeRelease(), COMMIT, output, False)
            self.assertEqual(status, 1)
            self.assertFalse(output.exists())

    def test_ci_4_5_a_job_that_did_not_pass_fails(self):
        jobs = {"conformance": "success", "ossie-suites": "failure", "conformance-wrappers": "success"}
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(full_actions(ci_jobs=jobs), COMMIT, False)

    def test_ci_4_5_a_failed_wheel_job_fails(self):
        actions = full_actions()
        jobs = actions.json["actions/runs/22/jobs?filter=latest&per_page=100"]["jobs"]
        jobs[0]["conclusion"] = "failure"
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT, False)

    def test_ci_4_5_a_missing_value_fails(self):
        actions = full_actions()
        summaries = ci_summaries()
        del summaries["conformance"]["duckdb_version"]
        actions.add_run("ci.yml", 11, CI_URL, all_ci_jobs(), summaries)
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT, False)

    def test_ci_4_5_a_workflow_that_did_not_run_on_the_commit_fails(self):
        actions = full_actions()
        query = f"head_sha={COMMIT}&event=push&per_page=100"
        actions.json[f"actions/workflows/python.yml/runs?{query}"] = {"workflow_runs": []}
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT, False)

    def test_ci_4_1_writes_and_attaches_both_assets_and_the_markdown_renders_the_json(self):
        release = FakeRelease()
        release.assets[matrix.JSON_ASSET] = b"old"
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory)
            self.assertEqual(matrix.release(full_actions(), release, COMMIT, output, True), 0)
            written = json.loads((output / matrix.JSON_ASSET).read_text(encoding="utf-8"))
            markdown = (output / matrix.MARKDOWN_ASSET).read_text(encoding="utf-8")
        self.assertEqual(json.loads(release.assets[matrix.JSON_ASSET]), written)
        self.assertEqual(markdown, matrix.render_markdown(written))
        table = [line for line in markdown.splitlines() if line.startswith("| ")]
        self.assertEqual(len(table), 1 + len(written["rows"]))

    def test_ci_4_1_the_assets_are_byte_identical_for_the_same_runs(self):
        first = matrix.render_json(matrix.build_matrix(full_actions(), COMMIT, False))
        second = matrix.render_json(matrix.build_matrix(full_actions(), COMMIT, False))
        self.assertEqual(first, second)


class ReleaseTagTest(unittest.TestCase):
    """CI-2 `release-assets`: the matrix is attached from 1.0.0."""

    def test_ci_4_1_tags_from_1_0_0_publish_a_matrix(self):
        for tag in ("v1.0.0", "1.0.0", "v1.0.1", "v1.2.0", "v2.0.0-rc1"):
            self.assertTrue(matrix.matrix_release(tag), tag)

    def test_ci_4_1_tags_before_1_0_0_publish_no_matrix(self):
        for tag in ("v0.1.0", "v0.9.0", "v0.10.3", "v1.0.0-rc1"):
            self.assertFalse(matrix.matrix_release(tag), tag)

    def test_ci_4_2_tags_from_1_1_0_by_semver_precedence_have_a_clickhouse_row(self):
        for tag in ("v1.1.0", "1.1.0", "v1.1.1", "v1.2.0-rc1", "v2.0.0"):
            self.assertTrue(matrix.clickhouse_release(tag), tag)

    def test_ci_4_2_tags_before_1_1_0_have_no_clickhouse_row(self):
        for tag in ("v1.0.0", "v1.0.9", "v1.1.0-rc1", "v0.9.0"):
            self.assertFalse(matrix.clickhouse_release(tag), tag)

    def test_ci_4_1_a_tag_that_is_not_a_version_fails(self):
        with self.assertRaises(matrix.MatrixError):
            matrix.matrix_release("latest")

    def release_with_tag(self, tag, actions):
        """Drives `main release` with RELEASE_TAG `tag` against `actions`; the exit status and
        the dialects of the rows attached."""
        environment = {"GITHUB_REPOSITORY": "owner/menura", "GITHUB_TOKEN": "token", "RELEASE_TAG": tag}
        attached = FakeRelease()
        with tempfile.TemporaryDirectory() as directory, \
                mock.patch.object(matrix, "ActionsApi", return_value=actions), \
                mock.patch.object(matrix, "MatrixRelease", return_value=attached):
            status = matrix.main(["release", COMMIT, str(pathlib.Path(directory) / "out")], environment)
        rows = json.loads(attached.assets[matrix.JSON_ASSET])["rows"] if attached.assets else []
        return status, [entry["dialect"] for entry in rows]

    def test_ci_4_2_main_with_a_1_0_x_tag_attaches_no_clickhouse_row_without_its_job(self):
        jobs = {name: "success" for name in matrix.CI_JOBS}
        for tag in ("v1.0.0", "v1.0.3", "v1.1.0-rc1"):
            actions = full_actions(ci_jobs=jobs, summaries=summaries_before_1_1_0())
            status, dialects = self.release_with_tag(tag, actions)
            self.assertEqual(status, 0, tag)
            self.assertEqual(set(dialects), {"duckdb"}, tag)

    def test_ci_4_2_main_with_a_1_1_0_tag_attaches_the_clickhouse_row(self):
        status, dialects = self.release_with_tag("v1.1.0", full_actions())
        self.assertEqual(status, 0)
        self.assertEqual(dialects.count("clickhouse"), 1)

    def test_ci_4_5_main_with_a_1_1_0_tag_fails_without_the_clickhouse_job(self):
        jobs = {name: "success" for name in matrix.CI_JOBS}
        actions = full_actions(ci_jobs=jobs, summaries=summaries_before_1_1_0())
        status, dialects = self.release_with_tag("v1.1.0", actions)
        self.assertEqual(status, 1)
        self.assertEqual(dialects, [])

    def test_ci_4_1_a_release_before_1_0_0_reads_no_ci_run(self):
        environment = {"GITHUB_REPOSITORY": "owner/menura", "GITHUB_TOKEN": "token", "RELEASE_TAG": "v0.9.0"}
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory) / "out"
            self.assertEqual(matrix.main(["release", COMMIT, str(output)], environment), 0)
            self.assertFalse(output.exists())


if __name__ == "__main__":
    unittest.main()
