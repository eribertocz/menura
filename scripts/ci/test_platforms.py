"""PLT-1.1: the platforms that `core`, `cli` and `mcp` are built and tested on, and the runners
of the jobs that test them. Run: python3 -m unittest discover -s scripts/ci -p 'test_platforms.py'"""
import pathlib
import re
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import compatibility_matrix as matrix  # noqa: E402

CI_WORKFLOW = pathlib.Path(__file__).resolve().parent.parent.parent / ".github" / "workflows" / "ci.yml"
# The platforms of PLT-1.1 for `core`, `cli` and `mcp`.
SUPPORTED = {"linux-x86_64", "macos-x86_64", "macos-arm64", "windows-x86_64"}
# The runner of each platform of the `platforms` job.
PLATFORMS_RUNNERS = {"macos-arm64": "macos-15", "macos-x86_64": "macos-15-intel", "windows-x86_64": "windows-2025"}
MATRIX_ENTRY = re.compile(r"^\s+- \{ platform: ([a-z0-9_-]+), runner: ([a-z0-9.-]+) \}$")


def job_text(name):
    """The lines of the job `name` of ci.yml, up to the next job."""
    lines = CI_WORKFLOW.read_text(encoding="utf-8").splitlines()
    start = lines.index(f"  {name}:")
    end = next((index for index in range(start + 1, len(lines)) if re.match(r"^  [a-z]", lines[index])), len(lines))
    return lines[start:end]


def matrix_entries(name):
    """The `platform` and `runner` of each matrix entry of the job `name`."""
    return dict(match.groups() for match in map(MATRIX_ENTRY.match, job_text(name)) if match)


class PlatformsTest(unittest.TestCase):
    def test_plt_1_1_the_platforms_job_runs_on_its_three_runners(self):
        self.assertEqual(matrix_entries("platforms"), PLATFORMS_RUNNERS)

    def test_plt_1_1_with_the_conformance_platform_the_set_is_exactly_the_four_of_plt_1_1(self):
        self.assertIn("    runs-on: ubuntu-latest", job_text("conformance"))
        self.assertEqual(set(matrix_entries("platforms")) | {"linux-x86_64"}, SUPPORTED)

    def test_plt_1_1_the_matrix_rows_follow_the_platforms_of_the_job(self):
        self.assertEqual(set(matrix.PLATFORMS_JOB_PLATFORMS), set(PLATFORMS_RUNNERS))

    def test_plt_1_1_each_runner_reports_the_platform_of_its_entry(self):
        runners = {"macos-15": ("macOS", "ARM64"), "macos-15-intel": ("macOS", "X64"),
                   "windows-2025": ("Windows", "X64")}
        for platform, runner in PLATFORMS_RUNNERS.items():
            operating_system, architecture = runners[runner]
            environment = {"RUNNER_OS": operating_system, "RUNNER_ARCH": architecture}
            self.assertEqual(matrix.runner_platform(environment), platform)


if __name__ == "__main__":
    unittest.main()
