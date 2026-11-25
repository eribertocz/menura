"""Compatibility matrix (spec/09-continuous-integration.md CI-4).

Two commands:

- `summarize JOB ...` runs in a CI job and writes the small JSON summary that the job uploads
  as the artifact `ci-summary-<name>`: the values of the matrix that only that job knows.
- `release COMMIT OUTPUT-DIRECTORY` runs in `release-assets`. It reads the CI runs of the tag
  commit through the GitHub REST API, builds `compatibility-matrix.json` and
  `compatibility-matrix.md` (CI-4.1, CI-4.2, CI-4.3) and attaches both to the release,
  replacing assets of the same name. When a value is missing, or a job a row depends on did not
  pass or did not run on the commit, it fails and writes nothing (CI-4.5).

Usage:
  compatibility_matrix.py summarize conformance REPORT DESCRIPTION --output FILE
  compatibility_matrix.py summarize ossie-suites REPORT --output FILE
  compatibility_matrix.py summarize conformance-wrappers --output FILE
  compatibility_matrix.py summarize conformance-clickhouse RUN-SUMMARY --output FILE
  compatibility_matrix.py summarize python-wheels PLATFORM --output FILE
  compatibility_matrix.py release COMMIT OUTPUT-DIRECTORY
DESCRIPTION is the model description that the `menura` binary built in the same job wrote; its
capability profile (DSC-4) gives the Menura version and the Apache Ossie normative version and
tracked commit. RUN-SUMMARY is the JSON file the ClickHouse conformance runner wrote, with the
server and time-zone data versions it read from the server (E6-7.16). The summaries read RUNNER_OS and RUNNER_ARCH; `release` reads GITHUB_REPOSITORY,
GITHUB_TOKEN and RELEASE_TAG, and does nothing for a tag before 1.0.0 (CI-2 `release-assets`). From 1.1.0 the matrix also has
the `core` row of the dialect `clickhouse` (CI-4.2).
Exit status: 0 on success, 1 otherwise.
"""
import io
import json
import os
import pathlib
import re
import sys
import urllib.error
import urllib.parse
import urllib.request
import zipfile

# `python3 -I` leaves the script's directory off the path; the release upload is shared with the
# sibling script release_assets.py.
sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import release_assets  # noqa: E402

JSON_ASSET = "compatibility-matrix.json"
MARKDOWN_ASSET = "compatibility-matrix.md"
SUMMARY_PREFIX = "ci-summary-"
DIALECT = "duckdb"
CLICKHOUSE_DIALECT = "clickhouse"

# The wheel platforms of ARC-6.3, in the order their rows are written.
WHEEL_PLATFORMS = ("linux-x86_64", "linux-aarch64", "macos-x86_64", "macos-arm64", "windows-x86_64")
# The platform names of ARC-6.3 for each runner operating system and architecture.
RUNNER_PLATFORMS = {
    ("Linux", "X64"): "linux-x86_64",
    ("Linux", "ARM64"): "linux-aarch64",
    ("macOS", "X64"): "macos-x86_64",
    ("macOS", "ARM64"): "macos-arm64",
    ("Windows", "X64"): "windows-x86_64",
}
# The jobs of ci.yml that every row depends on (CI-4.2, CI-4.5).
CI_WORKFLOW = "ci.yml"
PYTHON_WORKFLOW = "python.yml"
CI_JOBS = ("conformance", "ossie-suites", "conformance-wrappers")
# The job of ci.yml that the `clickhouse` row depends on, from 1.1.0 (CI-4.2, E6-7.10).
CLICKHOUSE_JOB = "conformance-clickhouse"
WHEEL_JOB = re.compile(r"^python-wheels \(([a-z0-9_-]+)\)$")

CASE_LINE = re.compile(r"^  (\S+) (passed|pending|failed)(?:: (.*))?$")
TOTAL_LINE = re.compile(r"^(\d+) cases: (\d+) passed, (\d+) pending, (\d+) failed$")
DUCKDB_LINE = re.compile(r"^DuckDB (v\d+\.\d+\.\d+\S*)$")
PASSES_WHILE_LISTED = "passes while listed in expected-failures.yaml"
RELEASE_TAG = re.compile(r"^v?(\d+)\.(\d+)\.(\d+)(-[0-9A-Za-z.-]+)?$")
FIRST_MATRIX_RELEASE = (1, 0, 0)
FIRST_CLICKHOUSE_RELEASE = (1, 1, 0)


class MatrixError(Exception):
    """A value of the matrix is missing, or a job it depends on did not pass (CI-4.5)."""


# Summaries, written by the CI jobs.


def runner_platform(environment):
    """The ARC-6.3 platform name of the runner, from RUNNER_OS and RUNNER_ARCH."""
    key = (environment.get("RUNNER_OS"), environment.get("RUNNER_ARCH"))
    if key not in RUNNER_PLATFORMS:
        raise MatrixError(f"unknown runner platform {key[0]} {key[1]}")
    return RUNNER_PLATFORMS[key]


def parse_report(text):
    """The case statuses (`id -> (status, reason)`) and DuckDB version of a runner report."""
    cases = {}
    totals = None
    duckdb = None
    for line in text.splitlines():
        case = CASE_LINE.match(line)
        if case:
            cases[case.group(1)] = (case.group(2), case.group(3) or "")
        elif TOTAL_LINE.match(line):
            totals = [int(number) for number in TOTAL_LINE.match(line).groups()]
        elif DUCKDB_LINE.match(line):
            duckdb = DUCKDB_LINE.match(line).group(1)
    if totals is None or duckdb is None:
        raise MatrixError("the runner report has no summary line or no DuckDB version")
    statuses = [status for status, _ in cases.values()]
    counted = [len(cases)] + [statuses.count(label) for label in ("passed", "pending", "failed")]
    if counted != totals:
        raise MatrixError(f"the runner report lists {counted} cases but its summary says {totals}")
    return cases, duckdb


def conformance_summary(report, description, environment):
    """The summary of `conformance`: counts, DuckDB version and the capability profile values of
    the model description the job's own `menura describe` wrote."""
    cases, duckdb = parse_report(report)
    capabilities = json.loads(description).get("capabilities", {})
    ossie = capabilities.get("ossie", {})
    if not (capabilities.get("compiler_version") and ossie.get("normative") and ossie.get("tracked_commit")):
        raise MatrixError("the model description has no complete capability profile (DSC-4)")
    statuses = [status for status, _ in cases.values()]
    return {
        "job": "conformance",
        "platform": runner_platform(environment),
        "dialect": DIALECT,
        "duckdb_version": duckdb,
        "menura_version": capabilities.get("compiler_version"),
        "ossie": {"normative": ossie.get("normative"), "tracked_commit": ossie.get("tracked_commit")},
        "cases": {"passed": statuses.count("passed"), "total": len(statuses)},
    }


def ossie_summary(report, environment):
    """The summary of `ossie-suites`: passed, expected failures and unexpected results (OSS-2.4)."""
    cases, duckdb = parse_report(report)
    passed = expected = unexpected = 0
    for status, reason in cases.values():
        if status == "passed":
            passed += 1
        elif status == "pending" and PASSES_WHILE_LISTED not in reason:
            expected += 1
        else:
            unexpected += 1
    return {
        "job": "ossie-suites",
        "platform": runner_platform(environment),
        "duckdb_version": duckdb,
        "outcomes": {"passed": passed, "expected_failures": expected, "unexpected": unexpected},
    }


def wrappers_summary(environment):
    """The summary of `conformance-wrappers`: its platform; it is uploaded only when it passed."""
    return {"job": "conformance-wrappers", "platform": runner_platform(environment), "parity": "passed"}


def wheel_summary(platform, environment):
    """The summary of one `python-wheels` job: its platform and that the package tests passed."""
    if runner_platform(environment) != platform:
        raise MatrixError(f"the wheel platform {platform} is not the runner's platform")
    return {"job": "python-wheels", "platform": platform, "package_tests": "passed"}


def clickhouse_summary(run_summary, environment):
    """The summary of `conformance-clickhouse`: the runner's own summary of the run, checked, with
    the runner's platform. The server and time-zone data versions are the ones the runner read
    from the server it ran on (E6-7.16), never written here."""
    run = json.loads(run_summary)
    if run.get("job") != CLICKHOUSE_JOB or run.get("dialect") != CLICKHOUSE_DIALECT:
        raise MatrixError("the ClickHouse run summary is not one of conformance-clickhouse")
    versions = {key: run.get(key) for key in ("clickhouse_version", "clickhouse_time_zone_data_version")}
    for key, value in versions.items():
        if not isinstance(value, str) or not value:
            raise MatrixError(f"the ClickHouse run summary has no {key}")
    cases = run.get("cases", {})
    passed, total = cases.get("passed"), cases.get("total")
    if not (isinstance(passed, int) and isinstance(total, int) and 0 <= passed <= total):
        raise MatrixError("the ClickHouse run summary has no valid case counts")
    summary = {"job": CLICKHOUSE_JOB, "platform": runner_platform(environment), "dialect": CLICKHOUSE_DIALECT}
    summary.update(versions)
    summary["cases"] = {"passed": passed, "total": total}
    return summary


def summarize(arguments, environment):
    """Builds the summary of the job named first in `arguments`."""
    job, *values = arguments
    if job == "conformance" and len(values) == 2:
        report, profile = (pathlib.Path(value).read_text(encoding="utf-8") for value in values)
        return conformance_summary(report, profile, environment)
    if job == "ossie-suites" and len(values) == 1:
        return ossie_summary(pathlib.Path(values[0]).read_text(encoding="utf-8"), environment)
    if job == "conformance-wrappers" and not values:
        return wrappers_summary(environment)
    if job == CLICKHOUSE_JOB and len(values) == 1:
        return clickhouse_summary(pathlib.Path(values[0]).read_text(encoding="utf-8"), environment)
    if job == "python-wheels" and len(values) == 1:
        return wheel_summary(values[0], environment)
    raise MatrixError(f"bad summarize arguments: {' '.join(arguments)}")


# The matrix, built from the CI runs of the tag commit.


def latest_run(api, workflow, commit):
    """The latest run of `workflow` pushed on `commit`."""
    query = urllib.parse.urlencode({"head_sha": commit, "event": "push", "per_page": 100})
    runs = api.get_json(f"actions/workflows/{workflow}/runs?{query}")["workflow_runs"]
    runs = [run for run in runs if run.get("head_sha") == commit]
    if not runs:
        raise MatrixError(f"{workflow} did not run on {commit}")
    return max(runs, key=lambda run: run["id"])


def run_jobs(api, run):
    """The latest attempt of every job of `run`, by name."""
    jobs = api.get_json(f"actions/runs/{run['id']}/jobs?filter=latest&per_page=100")["jobs"]
    return {job["name"]: job for job in jobs}


def run_summaries(api, run):
    """The JSON summary artifacts of `run`, by job summary name (without the prefix)."""
    artifacts = api.get_json(f"actions/runs/{run['id']}/artifacts?per_page=100")["artifacts"]
    latest = {}
    for artifact in artifacts:
        name = artifact["name"]
        if name.startswith(SUMMARY_PREFIX) and not artifact.get("expired"):
            if name not in latest or artifact["id"] > latest[name]["id"]:
                latest[name] = artifact
    summaries = {}
    for name, artifact in latest.items():
        with zipfile.ZipFile(io.BytesIO(api.get_bytes(artifact["archive_download_url"]))) as archive:
            members = archive.namelist()
            if len(members) != 1:
                raise MatrixError(f"artifact {name} must hold exactly one file")
            summaries[name[len(SUMMARY_PREFIX):]] = json.loads(archive.read(members[0]))
    return summaries


def require_passed(jobs, name, commit):
    """The job `name`; fails unless it ran on `commit` and passed (CI-4.5)."""
    job = jobs.get(name)
    if job is None:
        raise MatrixError(f"job {name} did not run on {commit}")
    if job.get("conclusion") != "success":
        raise MatrixError(f"job {name} did not pass on {commit}: {job.get('conclusion')}")
    return job


def require(summaries, name, *path):
    """The value at `path` of summary `name`; fails when it is missing (CI-4.5)."""
    value = summaries.get(name)
    for key in path:
        value = value.get(key) if isinstance(value, dict) else None
    if value is None or value == "":
        raise MatrixError(f"missing value {'.'.join(path)} of summary {name}")
    return value


def platform_fields(name):
    """The operating system and architecture of an ARC-6.3 platform name."""
    operating_system, architecture = name.split("-", 1)
    return {"os": operating_system, "arch": architecture}


def build_matrix(api, commit, with_clickhouse):
    """The matrix document of `commit` (CI-4.2, CI-4.3); raises MatrixError per CI-4.5.
    `with_clickhouse` adds the `core` row of the dialect `clickhouse`, for a tag 1.1.0 or later."""
    ci_run = latest_run(api, CI_WORKFLOW, commit)
    python_run = latest_run(api, PYTHON_WORKFLOW, commit)
    ci_jobs = run_jobs(api, ci_run)
    for name in CI_JOBS + ((CLICKHOUSE_JOB,) if with_clickhouse else ()):
        require_passed(ci_jobs, name, commit)
    summaries = run_summaries(api, ci_run)
    platform = require(summaries, "conformance", "platform")
    for name in ("ossie-suites", "conformance-wrappers"):
        if require(summaries, name, "platform") != platform:
            raise MatrixError(f"{name} did not run on the platform of conformance ({platform})")
    profile = release_profile(summaries, commit)
    common = dict(profile, **duckdb_columns(summaries))
    require(summaries, "conformance-wrappers", "parity")
    rows = [row(common, "core", platform, None, [ci_run["html_url"]])]
    if with_clickhouse:
        rows.append(clickhouse_row(summaries, profile, ci_run["html_url"]))
    rows.append(row(common, "cli", platform, "conformance-wrappers", [ci_run["html_url"]]))
    rows.append(row(common, "mcp", platform, "conformance-wrappers", [ci_run["html_url"]]))
    rows.extend(python_rows(api, python_run, common, platform, ci_run["html_url"], commit))
    return {"rows": rows}


def release_profile(summaries, commit):
    """The columns every row shares: Menura version, commit and the capability profile (DSC-4)
    of the model description that `conformance` wrote on that commit."""
    return {
        "menura_version": require(summaries, "conformance", "menura_version"),
        "commit": commit,
        "ossie": {
            "normative": require(summaries, "conformance", "ossie", "normative"),
            "tracked_commit": require(summaries, "conformance", "ossie", "tracked_commit"),
        },
    }


def duckdb_columns(summaries):
    """The dialect, engine, conformance and Apache Ossie suite columns of the `duckdb` rows."""
    if require(summaries, "ossie-suites", "duckdb_version") != require(summaries, "conformance", "duckdb_version"):
        raise MatrixError("conformance and ossie-suites ran on different DuckDB versions")
    return {
        "dialect": require(summaries, "conformance", "dialect"),
        "engine": {"name": DIALECT, "version": require(summaries, "conformance", "duckdb_version")},
        "conformance": {
            "passed": require(summaries, "conformance", "cases", "passed"),
            "total": require(summaries, "conformance", "cases", "total"),
        },
        "ossie_suites": {
            key: require(summaries, "ossie-suites", "outcomes", key)
            for key in ("passed", "expected_failures", "unexpected")
        },
    }


def clickhouse_row(summaries, profile, ci_url):
    """The `core` row of the dialect `clickhouse` on the platform of `conformance-clickhouse`
    (CI-4.2). Its Apache Ossie suite outcomes are null: the suites run on DuckDB only (CI-4.3)."""
    if require(summaries, CLICKHOUSE_JOB, "dialect") != CLICKHOUSE_DIALECT:
        raise MatrixError(f"summary {CLICKHOUSE_JOB} is not of the dialect {CLICKHOUSE_DIALECT}")
    columns = dict(profile)
    columns.update({
        "dialect": CLICKHOUSE_DIALECT,
        "engine": {
            "name": CLICKHOUSE_DIALECT,
            "version": require(summaries, CLICKHOUSE_JOB, "clickhouse_version"),
            "time_zone_data_version": require(summaries, CLICKHOUSE_JOB, "clickhouse_time_zone_data_version"),
        },
        "conformance": {
            "passed": require(summaries, CLICKHOUSE_JOB, "cases", "passed"),
            "total": require(summaries, CLICKHOUSE_JOB, "cases", "total"),
        },
        "ossie_suites": None,
    })
    return row(columns, "core", require(summaries, CLICKHOUSE_JOB, "platform"), None, [ci_url])


def python_rows(api, python_run, common, wrappers_platform, ci_url, commit):
    """One `python` row per wheel platform whose `python-wheels` job ran (CI-4.2)."""
    jobs = run_jobs(api, python_run)
    tested = {WHEEL_JOB.match(name).group(1): name for name in jobs if WHEEL_JOB.match(name)}
    summaries = run_summaries(api, python_run)
    rows = []
    for platform in WHEEL_PLATFORMS:
        if platform not in tested:
            continue
        require_passed(jobs, tested[platform], commit)
        name = f"python-wheels-{platform}"
        if require(summaries, name, "platform") != platform:
            raise MatrixError(f"summary {name} names another platform")
        require(summaries, name, "package_tests")
        if platform == wrappers_platform:
            rows.append(row(common, "python", platform, "conformance-wrappers", [ci_url, python_run["html_url"]]))
        else:
            rows.append(row(common, "python", platform, "package tests", [python_run["html_url"]]))
    return rows


def row(common, channel, platform, wrapper_check, runs):
    """One matrix row; `wrapper_check` is None for `core`, which has no wrapper."""
    entry = {"channel": channel, "platform": platform_fields(platform)}
    entry.update(common)
    if wrapper_check is not None:
        entry["wrappers"] = {"check": wrapper_check, "passed": True}
    entry["runs"] = sorted(set(runs))
    return entry


def render_json(matrix):
    """The JSON asset: sorted keys, two-space indentation, final line feed."""
    return json.dumps(matrix, indent=2, sort_keys=True, ensure_ascii=False) + "\n"


MARKDOWN_HEADER = (
    "| Channel | Platform | Menura | Commit | Dialect | Engine | Apache Ossie | Ossie tracked commit "
    "| Conformance | Ossie suites | Wrappers | CI runs |\n"
    "|---|---|---|---|---|---|---|---|---|---|---|---|\n"
)


ENGINE_NAMES = {DIALECT: "DuckDB", CLICKHOUSE_DIALECT: "ClickHouse"}


def render_engine(engine):
    """The engine cell: its name and exact version, then its time-zone data version if any."""
    text = f"{ENGINE_NAMES[engine['name']]} {engine['version']}"
    if "time_zone_data_version" in engine:
        text += f", time-zone data {engine['time_zone_data_version']}"
    return text


def render_suites(suites):
    """The Apache Ossie suites cell; empty when the outcomes are null (CI-4.3)."""
    if suites is None:
        return ""
    return (
        f"{suites['passed']} passed, {suites['expected_failures']} expected failures, "
        f"{suites['unexpected']} unexpected"
    )


def render_markdown(matrix):
    """The Markdown asset: the rows of the JSON asset as a table, and nothing else (CI-4.1)."""
    lines = ["# Compatibility matrix\n\n", MARKDOWN_HEADER]
    for entry in matrix["rows"]:
        wrappers = entry.get("wrappers")
        cells = [
            entry["channel"],
            f"{entry['platform']['os']} {entry['platform']['arch']}",
            entry["menura_version"],
            entry["commit"],
            entry["dialect"],
            render_engine(entry["engine"]),
            entry["ossie"]["normative"],
            entry["ossie"]["tracked_commit"],
            f"{entry['conformance']['passed']} / {entry['conformance']['total']}",
            render_suites(entry["ossie_suites"]),
            f"{wrappers['check']} passed" if wrappers else "",
            " ".join(f"<{url}>" for url in entry["runs"]),
        ]
        lines.append("| " + " | ".join(str(cell) for cell in cells) + " |\n")
    return "".join(lines)


def write_assets(matrix, directory):
    """Writes both assets to `directory`; returns them as `name -> bytes`."""
    assets = {
        JSON_ASSET: render_json(matrix).encode("utf-8"),
        MARKDOWN_ASSET: render_markdown(matrix).encode("utf-8"),
    }
    directory.mkdir(parents=True, exist_ok=True)
    for name, content in assets.items():
        (directory / name).write_bytes(content)
    return assets


class ActionsApi:
    """Read access to the Actions runs of one repository, through the REST API."""

    def __init__(self, repository, token):
        self.repository = repository
        self.token = token

    def headers(self):
        return {
            "Accept": "application/vnd.github+json",
            "Authorization": f"Bearer {self.token}",
            "X-GitHub-Api-Version": "2022-11-28",
        }

    def get_json(self, path):
        """GET `repos/<repository>/<path>`, decoded."""
        url = f"{release_assets.API}/repos/{self.repository}/{path}"
        request = urllib.request.Request(url, headers=self.headers())
        with urllib.request.urlopen(request, timeout=60) as response:
            return json.loads(response.read())

    def get_bytes(self, url):
        """GET an artifact archive. The API answers with a redirect to signed storage, which is
        followed without the token."""
        opener = urllib.request.build_opener(NoRedirect)
        try:
            with opener.open(urllib.request.Request(url, headers=self.headers()), timeout=60) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code not in (301, 302, 303, 307, 308):
                raise
            location = error.headers["Location"]
        with urllib.request.urlopen(urllib.request.Request(location), timeout=120) as response:
            return response.read()


class NoRedirect(urllib.request.HTTPRedirectHandler):
    """Leaves redirects to the caller, so the token is never sent to the storage host."""

    def redirect_request(self, request, file, code, message, headers, new_url):
        return None


class MatrixRelease(release_assets.ReleaseApi):
    """The release assets, uploaded with the content type of their extension."""

    def upload_asset(self, name, content):
        query = urllib.parse.urlencode({"name": name})
        url = f"{release_assets.UPLOADS}/repos/{self.repository}/releases/{self.release_id}/assets?{query}"
        content_type = "application/json" if name.endswith(".json") else "text/markdown; charset=utf-8"
        self.call("POST", url, data=content, content_type=content_type)


def release(api, attach_to, commit, directory, with_clickhouse):
    """Builds the matrix of `commit`, writes it to `directory` and attaches it; exit status.
    `with_clickhouse` is whether the tag is 1.1.0 or later (CI-4.2)."""
    try:
        matrix = build_matrix(api, commit, with_clickhouse)
    except MatrixError as error:
        print(f"no compatibility matrix (CI-4.5): {error}", file=sys.stderr)
        return 1
    assets = write_assets(matrix, directory)
    release_assets.attach(attach_to, assets)
    absent = release_assets.missing(attach_to, assets)
    for name in absent:
        print(f"missing release asset: {name}", file=sys.stderr)
    if absent:
        return 1
    print(f"attached {JSON_ASSET} and {MARKDOWN_ASSET} with {len(matrix['rows'])} rows")
    return 0


def at_or_after(tag, version):
    """Whether the release `tag` is `version` or later by SemVer precedence: a pre-release of
    `version` comes before it. Raises MatrixError for a tag that is not a version."""
    match = RELEASE_TAG.match(tag)
    if match is None:
        raise MatrixError(f"the tag {tag} is not a MAJOR.MINOR.PATCH version")
    numbers = tuple(int(number) for number in match.groups()[:3])
    if numbers == version:
        return match.group(4) is None
    return numbers > version


def matrix_release(tag):
    """Whether the release `tag` publishes a matrix: version 1.0.0 or later (CI-2)."""
    return at_or_after(tag, FIRST_MATRIX_RELEASE)


def clickhouse_release(tag):
    """Whether the matrix of the release `tag` has the `clickhouse` row: 1.1.0 or later (CI-4.2)."""
    return at_or_after(tag, FIRST_CLICKHOUSE_RELEASE)


def main(arguments, environment):
    if len(arguments) >= 2 and arguments[0] == "summarize" and "--output" in arguments:
        position = arguments.index("--output")
        if position + 1 >= len(arguments):
            print(__doc__, file=sys.stderr)
            return 1
        output = pathlib.Path(arguments[position + 1])
        try:
            summary = summarize(arguments[1:position] + arguments[position + 2:], environment)
        except (MatrixError, OSError, ValueError) as error:
            print(f"no summary (CI-4.5): {error}", file=sys.stderr)
            return 1
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes((json.dumps(summary, indent=2, sort_keys=True) + "\n").encode("utf-8"))
        return 0
    if len(arguments) == 3 and arguments[0] == "release":
        repository = environment.get("GITHUB_REPOSITORY")
        token = environment.get("GITHUB_TOKEN")
        tag = environment.get("RELEASE_TAG")
        if not repository or not token or not tag:
            print("GITHUB_REPOSITORY, GITHUB_TOKEN and RELEASE_TAG are required", file=sys.stderr)
            return 1
        try:
            if not matrix_release(tag):
                print(f"{tag} is before 1.0.0: no compatibility matrix (CI-2)")
                return 0
            with_clickhouse = clickhouse_release(tag)
        except MatrixError as error:
            print(f"no compatibility matrix: {error}", file=sys.stderr)
            return 1
        try:
            attach_to = MatrixRelease(repository, token, tag)
            directory = pathlib.Path(arguments[2])
            return release(ActionsApi(repository, token), attach_to, arguments[1], directory, with_clickhouse)
        except urllib.error.HTTPError as error:
            print(f"GitHub REST API error {error.code} on {error.url}: {error.reason}", file=sys.stderr)
            return 1
    print(__doc__, file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:], os.environ))
