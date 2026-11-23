"""ARC-6.9: for every conformance case, the documents of the Python package equal those
of the CLI byte for byte, and its usage errors carry the CLI's usage reason.

The CLI binary is named by the environment variable `MENURA_CLI`. Requires PyYAML.
"""

import json
import os
import pathlib
import subprocess
import sys
import tempfile
import unittest

import yaml

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parents[1]))

import menura  # noqa: E402
from support import MODELS, ROOT  # noqa: E402

CLI = os.environ.get("MENURA_CLI", "")
CASES = sorted((ROOT / "conformance" / "cases").glob("*/*.yaml"))
QUERY_OUTCOMES = {"error", "plan", "runtime_error"}


def run_cli(*arguments):
    """The CLI's exit status, standard output and standard error for `arguments`."""
    completed = subprocess.run(
        [CLI, *[str(argument) for argument in arguments]], capture_output=True, check=False
    )
    return completed.returncode, completed.stdout.decode("utf-8"), completed.stderr.decode("utf-8")


class Case:
    """The inputs of one conformance case, as files for the CLI and values for Python."""

    def __init__(self, path, directory):
        self.case = yaml.safe_load(path.read_text(encoding="utf-8"))
        self.model_directory = MODELS / self.case["model"]
        self.directory = pathlib.Path(directory)
        self.load_options = self.case.get("load_options")
        self.compile_options = dict(self.case.get("compile_options") or {})
        if "audit" in self.case["expect"]:
            self.compile_options["audit"] = True

    def model_file(self, name):
        return self.model_directory / name

    def catalog_state(self):
        path = self.model_file("catalog-state.json")
        return path if path.exists() else None

    def options_file(self, name, options):
        path = self.directory / name
        path.write_text(json.dumps(options), encoding="utf-8")
        return path

    def model_flags(self):
        flags = ["--model", self.model_file("model.yaml")]
        flags += ["--catalog-schema", self.model_file("catalog-schema.json")]
        if self.load_options is not None:
            flags += ["--load-options", self.options_file("load.json", self.load_options)]
        return flags

    def handle_flags(self):
        flags = self.model_flags()
        if self.catalog_state() is not None:
            flags += ["--catalog-state", self.catalog_state()]
        return flags

    def load(self):
        state = self.catalog_state()
        return menura.Handle.load(
            self.model_file("model.yaml").read_bytes(),
            self.model_file("catalog-schema.json").read_bytes(),
            state.read_bytes() if state else None,
            self.load_options,
        )

    def security_context(self):
        return self.model_file("contexts") / self.case["security_context"]

    def query(self):
        path = self.directory / "query.json"
        path.write_text(json.dumps(self.case["query"]), encoding="utf-8")
        return path


class Arc69(unittest.TestCase):
    def setUp(self):
        self.assertTrue(CLI, "MENURA_CLI must name the menura binary")

    def assert_same(self, cli, result):
        status, output, _ = cli
        self.assertEqual(result.document + "\n", output)
        self.assertEqual(result.is_error, status == 1)

    def test_arc_6_9_validate_and_migrate_equal_the_cli(self):
        for path in CASES:
            with self.subTest(case=path.stem), tempfile.TemporaryDirectory() as directory:
                case = Case(path, directory)
                model = case.model_file("model.yaml").read_bytes()
                schema = case.model_file("catalog-schema.json").read_bytes()
                self.assert_same(
                    run_cli("validate", *case.model_flags()),
                    menura.validate(model, schema, case.load_options),
                )
                migrate_flags = ["--model", case.model_file("model.yaml")]
                migrate_flags += case.model_flags()[4:]
                self.assert_same(
                    run_cli("migrate", *migrate_flags), menura.migrate(model, case.load_options)
                )

    def test_arc_6_9_handle_calls_equal_the_cli(self):
        for path in CASES:
            with self.subTest(case=path.stem), tempfile.TemporaryDirectory() as directory:
                self.check_handle_case(Case(path, directory))

    def check_handle_case(self, case):
        try:
            handle = case.load()
        except menura.CompileError as error:
            status, output, _ = run_cli("dependencies", *case.model_flags())
            self.assertEqual((status, output), (1, error.document + "\n"))
            return
        self.assert_same(run_cli("dependencies", *case.model_flags()), handle.dependencies())
        self.assert_same(
            run_cli("integrity-checks", *case.model_flags()), handle.integrity_checks()
        )
        if "security_context" not in case.case:
            return
        context = case.security_context()
        self.assert_same(
            run_cli("describe", *case.handle_flags(), "--security-context", context),
            handle.describe(context.read_bytes()),
        )
        if case.case["expect"]["outcome"] in QUERY_OUTCOMES and "query" in case.case:
            self.check_compile_case(case, handle, context)

    def check_compile_case(self, case, handle, context):
        query = case.query()
        flags = [*case.handle_flags(), "--query", query, "--security-context", context]
        options = case.options_file("compile.json", case.compile_options)
        record = case.directory / "audit.json"
        audit = ["--audit-record", record] if case.compile_options.get("audit") else []
        result = handle.compile(query.read_bytes(), context.read_bytes(), case.compile_options)
        self.assert_same(
            run_cli("compile", *flags, "--compile-options", options, *audit), result
        )
        if audit:
            self.assertEqual(result.audit_record + "\n", record.read_text(encoding="utf-8"))
        self.assert_same(
            run_cli("explain", *flags, "--compile-options", options),
            handle.explain(query.read_bytes(), context.read_bytes(), case.compile_options),
        )

    def test_arc_6_9_diff_equals_the_cli(self):
        for old_path, new_path in zip(CASES, CASES[1:]):
            subtest = self.subTest(old=old_path.stem, new=new_path.stem)
            with subtest, tempfile.TemporaryDirectory() as directory:
                old, new = Case(old_path, directory), Case(new_path, directory)
                files = [
                    old.model_file("model.yaml"),
                    old.model_file("catalog-schema.json"),
                    new.model_file("model.yaml"),
                    new.model_file("catalog-schema.json"),
                ]
                flags = ["--old-model", files[0], "--old-catalog-schema", files[1]]
                flags += ["--new-model", files[2], "--new-catalog-schema", files[3]]
                options = old.load_options
                if options is not None:
                    flags += ["--load-options", old.options_file("load.json", options)]
                self.assert_same(
                    run_cli("diff", *flags),
                    menura.diff(*(path.read_bytes() for path in files), options),
                )

    def test_arc_6_9_with_state_equals_the_cli_catalog_state(self):
        for path in CASES:
            with self.subTest(case=path.stem), tempfile.TemporaryDirectory() as directory:
                case = Case(path, directory)
                state = case.catalog_state()
                if state is None or "security_context" not in case.case:
                    continue
                try:
                    handle = menura.Handle.load(
                        case.model_file("model.yaml").read_bytes(),
                        case.model_file("catalog-schema.json").read_bytes(),
                        None,
                        case.load_options,
                    )
                except menura.CompileError:
                    continue
                context = case.security_context()
                self.assert_same(
                    run_cli("describe", *case.handle_flags(), "--security-context", context),
                    handle.with_state(state.read_bytes()).describe(context.read_bytes()),
                )

    def test_arc_6_9_usage_errors_carry_the_cli_reason(self):
        model = MODELS / "slice" / "model.yaml"
        schema = MODELS / "slice" / "catalog-schema.json"
        for options in ({"colour": 1}, {"namespace": None}, {"namespace": 3}, []):
            with self.subTest(options=options), tempfile.TemporaryDirectory() as directory:
                path = pathlib.Path(directory) / "load.json"
                path.write_text(json.dumps(options), encoding="utf-8")
                status, output, error = run_cli(
                    "validate", "--model", model, "--catalog-schema", schema, "--load-options", path
                )
                self.assertEqual((status, output), (2, ""))
                with self.assertRaises(ValueError) as raised:
                    menura.validate(model.read_bytes(), schema.read_bytes(), options)
                expected = error.removeprefix("menura: ").rstrip("\n")
                expected = expected.replace("--load-options", "load_options")
                self.assertEqual(str(raised.exception), expected)


if __name__ == "__main__":
    unittest.main()
