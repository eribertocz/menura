"""Unit tests of the compatibility matrix (CI-4). Run: python3 -m unittest discover scripts/ci"""
import io
import json
import pathlib
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import compatibility_matrix as matrix  # noqa: E402

COMMIT = "0123456789abcdef0123456789abcdef01234567"
CI_URL = "https://github.com/owner/menura/actions/runs/11"
PYTHON_URL = "https://github.com/owner/menura/actions/runs/22"
LINUX = {"RUNNER_OS": "Linux", "RUNNER_ARCH": "X64"}
PROFILE = json.dumps(
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


def archive(document):
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as zipped:
        zipped.writestr("summary.json", json.dumps(document))
    return buffer.getvalue()


def ci_summaries():
    return {
        "conformance": matrix.conformance_summary(CONFORMANCE_REPORT, PROFILE, LINUX),
        "ossie-suites": matrix.ossie_summary(OSSIE_REPORT, LINUX),
        "conformance-wrappers": matrix.wrappers_summary(LINUX),
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


def full_actions(platforms=matrix.WHEEL_PLATFORMS, ci_jobs=None):
    jobs = ci_jobs or {name: "success" for name in matrix.CI_JOBS}
    wheel_jobs = {f"python-wheels ({platform})": "success" for platform in platforms}
    return FakeActions(jobs, wheel_jobs, ci_summaries(), wheel_summaries(platforms))


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
        summary = matrix.conformance_summary(CONFORMANCE_REPORT, PROFILE, LINUX)
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

    def test_ci_4_5_a_wheel_summary_on_another_runner_fails(self):
        with self.assertRaises(matrix.MatrixError):
            matrix.wheel_summary("macos-arm64", LINUX)


class MatrixTest(unittest.TestCase):
    """CI-4.1, CI-4.2, CI-4.3, CI-4.5."""

    def test_ci_4_2_a_full_matrix_has_core_cli_mcp_and_one_python_row_per_wheel_platform(self):
        document = matrix.build_matrix(full_actions(), COMMIT)
        rows = [(row["channel"], f"{row['platform']['os']}-{row['platform']['arch']}") for row in document["rows"]]
        self.assertEqual(
            rows,
            [("core", "linux-x86_64"), ("cli", "linux-x86_64"), ("mcp", "linux-x86_64")]
            + [("python", platform) for platform in matrix.WHEEL_PLATFORMS],
        )

    def test_ci_4_3_every_row_carries_the_columns(self):
        document = matrix.build_matrix(full_actions(), COMMIT)
        core, cli = document["rows"][0], document["rows"][1]
        self.assertEqual(core["menura_version"], "1.0.0")
        self.assertEqual(core["commit"], COMMIT)
        self.assertEqual(core["dialect"], "duckdb")
        self.assertEqual(core["duckdb_version"], "v1.2.0")
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

    def test_ci_4_2_a_wheel_platform_without_tests_has_no_row(self):
        platforms = [platform for platform in matrix.WHEEL_PLATFORMS if platform != "windows-x86_64"]
        document = matrix.build_matrix(full_actions(platforms), COMMIT)
        python = [row["platform"] for row in document["rows"] if row["channel"] == "python"]
        self.assertNotIn({"os": "windows", "arch": "x86_64"}, python)
        self.assertEqual(len(python), 4)

    def test_ci_4_5_a_missing_job_fails_without_writing_the_files(self):
        jobs = {"conformance": "success", "ossie-suites": "success"}
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory) / "out"
            status = matrix.release(full_actions(ci_jobs=jobs), FakeRelease(), COMMIT, output)
            self.assertEqual(status, 1)
            self.assertFalse(output.exists())

    def test_ci_4_5_a_job_that_did_not_pass_fails(self):
        jobs = {"conformance": "success", "ossie-suites": "failure", "conformance-wrappers": "success"}
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(full_actions(ci_jobs=jobs), COMMIT)

    def test_ci_4_5_a_failed_wheel_job_fails(self):
        actions = full_actions()
        jobs = actions.json["actions/runs/22/jobs?filter=latest&per_page=100"]["jobs"]
        jobs[0]["conclusion"] = "failure"
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT)

    def test_ci_4_5_a_missing_value_fails(self):
        actions = full_actions()
        summaries = ci_summaries()
        del summaries["conformance"]["duckdb_version"]
        actions.add_run("ci.yml", 11, CI_URL, {name: "success" for name in matrix.CI_JOBS}, summaries)
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT)

    def test_ci_4_5_a_workflow_that_did_not_run_on_the_commit_fails(self):
        actions = full_actions()
        query = f"head_sha={COMMIT}&event=push&per_page=100"
        actions.json[f"actions/workflows/python.yml/runs?{query}"] = {"workflow_runs": []}
        with self.assertRaises(matrix.MatrixError):
            matrix.build_matrix(actions, COMMIT)

    def test_ci_4_1_writes_and_attaches_both_assets_and_the_markdown_renders_the_json(self):
        release = FakeRelease()
        release.assets[matrix.JSON_ASSET] = b"old"
        with tempfile.TemporaryDirectory() as directory:
            output = pathlib.Path(directory)
            self.assertEqual(matrix.release(full_actions(), release, COMMIT, output), 0)
            written = json.loads((output / matrix.JSON_ASSET).read_text(encoding="utf-8"))
            markdown = (output / matrix.MARKDOWN_ASSET).read_text(encoding="utf-8")
        self.assertEqual(json.loads(release.assets[matrix.JSON_ASSET]), written)
        self.assertEqual(markdown, matrix.render_markdown(written))
        table = [line for line in markdown.splitlines() if line.startswith("| ")]
        self.assertEqual(len(table), 1 + len(written["rows"]))

    def test_ci_4_1_the_assets_are_byte_identical_for_the_same_runs(self):
        first = matrix.render_json(matrix.build_matrix(full_actions(), COMMIT))
        second = matrix.render_json(matrix.build_matrix(full_actions(), COMMIT))
        self.assertEqual(first, second)


if __name__ == "__main__":
    unittest.main()
